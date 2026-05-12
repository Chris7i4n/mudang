/// Ruby-specific language plugin.
///
/// Extracts Ruby symbols, lexical parent relationships, and v1 metadata:
/// method visibility regions, singleton receivers, namespaces, doc comments,
/// endless-method markers, and common parameter forms.
use anyhow::Result;
use serde::Serialize;

/// Structured metadata for a Ruby symbol.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RubyMetadata {
    /// Ruby visibility: public, protected, private, or unknown.
    pub visibility: String,
    /// Whether this is a singleton method (`def self.foo`).
    pub is_singleton: bool,
    /// Lexical namespace, when extracted.
    pub namespace: Option<String>,
    /// Receiver text for singleton methods.
    pub receiver: Option<String>,
    /// Parameter list.
    pub parameters: Vec<RubyParameterInfo>,
    /// Whether parameters include a block parameter (`&block`).
    pub has_block_param: bool,
    /// Whether parameters include `*args` or `**kwargs`.
    pub has_splat: bool,
    /// Whether parameters include keyword arguments.
    pub has_keyword_args: bool,
    /// Whether this is an endless method (`def foo = expr`).
    pub is_endless: bool,
}

/// Information about a single Ruby method parameter.
#[derive(Debug, Clone, Serialize)]
pub struct RubyParameterInfo {
    /// Parameter name.
    pub name: String,
    /// Parameter kind: required, optional, keyword, splat, etc.
    pub kind: String,
    /// Whether the parameter has a default value.
    pub has_default: bool,
}

/// Extract metadata from a Ruby AST node.
///
/// `private :foo` and `private_class_method :foo` are intentionally outside v1:
/// only bare `private`, `protected`, and `public` calls define visibility
/// regions for later methods in the same `body_statement`.
pub fn extract_metadata(node: &tree_sitter::Node, source: &str, kind: &str) -> Result<String> {
    let receiver = extract_receiver(node, source);
    let is_singleton =
        node.kind() == "singleton_method" || receiver.is_some() || is_in_singleton_class(node);
    let parameters = extract_parameters_for_symbol(node, source);

    let meta = RubyMetadata {
        visibility: if kind == "method" {
            visibility_for_node(node, source)
        } else {
            "public".to_string()
        },
        is_singleton,
        namespace: extract_namespace(node, source),
        receiver,
        has_block_param: parameters.iter().any(|p| p.kind == "block"),
        has_splat: parameters
            .iter()
            .any(|p| p.kind == "splat" || p.kind == "double_splat"),
        has_keyword_args: parameters.iter().any(|p| {
            p.kind == "keyword" || p.kind == "keyword_optional" || p.kind == "double_splat"
        }),
        is_endless: is_endless_method(node, source),
        parameters,
    };

    Ok(serde_json::to_string(&meta)?)
}

fn extract_parameters_for_symbol(node: &tree_sitter::Node, source: &str) -> Vec<RubyParameterInfo> {
    match node.kind() {
        "method" | "singleton_method" | "lambda" => node
            .child_by_field_name("parameters")
            .map(|params| extract_parameters(&params, source))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn extract_parameters(params_node: &tree_sitter::Node, source: &str) -> Vec<RubyParameterInfo> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor).filter(|n| n.is_named()) {
        match child.kind() {
            "identifier" => params.push(RubyParameterInfo {
                name: node_text(&child, source),
                kind: "positional".to_string(),
                has_default: false,
            }),
            "optional_parameter" => params.push(RubyParameterInfo {
                name: child_field_text(&child, "name", source)
                    .unwrap_or_else(|| node_text(&child, source)),
                kind: "optional".to_string(),
                has_default: true,
            }),
            "keyword_parameter" => {
                let has_default = child.child_by_field_name("value").is_some();
                params.push(RubyParameterInfo {
                    name: child_field_text(&child, "name", source).unwrap_or_else(|| {
                        node_text(&child, source).trim_end_matches(':').to_string()
                    }),
                    kind: if has_default {
                        "keyword_optional"
                    } else {
                        "keyword"
                    }
                    .to_string(),
                    has_default,
                });
            }
            "splat_parameter" => params.push(RubyParameterInfo {
                name: child_field_text(&child, "name", source).unwrap_or_else(|| "*".to_string()),
                kind: "splat".to_string(),
                has_default: false,
            }),
            "hash_splat_parameter" => params.push(RubyParameterInfo {
                name: child_field_text(&child, "name", source).unwrap_or_else(|| "**".to_string()),
                kind: "double_splat".to_string(),
                has_default: false,
            }),
            "block_parameter" => params.push(RubyParameterInfo {
                name: child_field_text(&child, "name", source).unwrap_or_else(|| "&".to_string()),
                kind: "block".to_string(),
                has_default: false,
            }),
            "forward_parameter" => params.push(RubyParameterInfo {
                name: "...".to_string(),
                kind: "forwarded".to_string(),
                has_default: false,
            }),
            "destructured_parameter" => params.push(RubyParameterInfo {
                name: node_text(&child, source),
                kind: "positional".to_string(),
                has_default: false,
            }),
            _ => {}
        }
    }

    params
}

fn visibility_for_node(node: &tree_sitter::Node, source: &str) -> String {
    let Some(body) = nearest_parent_kind(node, "body_statement") else {
        return "public".to_string();
    };

    let mut visibility = "public".to_string();
    let mut cursor = body.walk();

    for child in body.children(&mut cursor).filter(|n| n.is_named()) {
        if child == *node {
            break;
        }

        match node_text(&child, source).as_str() {
            "private" => visibility = "private".to_string(),
            "protected" => visibility = "protected".to_string(),
            "public" => visibility = "public".to_string(),
            _ => {}
        }
    }

    visibility
}

fn extract_receiver(node: &tree_sitter::Node, source: &str) -> Option<String> {
    node.child_by_field_name("object")
        .and_then(|object| object.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            if is_in_singleton_class(node) {
                Some("self".to_string())
            } else {
                None
            }
        })
}

fn extract_namespace(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let name = node
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())?
        .trim()
        .to_string();

    name.rsplit_once("::")
        .map(|(namespace, _)| namespace.to_string())
        .filter(|namespace| !namespace.is_empty())
        .or_else(|| lexical_namespace(node, source))
}

fn lexical_namespace(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut current = node.parent();

    while let Some(parent) = current {
        if parent.kind() == "class" || parent.kind() == "module" {
            if let Some(name) = child_field_text(&parent, "name", source) {
                parts.push(name.trim_start_matches("::").to_string());
            }
        }
        current = parent.parent();
    }

    if parts.is_empty() {
        None
    } else {
        parts.reverse();
        Some(parts.join("::"))
    }
}

fn is_endless_method(node: &tree_sitter::Node, source: &str) -> bool {
    if node.kind() != "method" && node.kind() != "singleton_method" {
        return false;
    }

    node.child_by_field_name("body")
        .map(|body| body.kind() != "body_statement" && node_text(&body, source).trim() != "end")
        .unwrap_or(false)
}

fn is_in_singleton_class(node: &tree_sitter::Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "singleton_class" {
            return true;
        }
        if parent.kind() == "class" || parent.kind() == "module" {
            return false;
        }
        current = parent.parent();
    }
    false
}

fn nearest_parent_kind<'a>(
    node: &tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == kind {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn child_field_text(node: &tree_sitter::Node, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|child| child.utf8_text(source.as_bytes()).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn node_text(node: &tree_sitter::Node, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn extract_docstring(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut current = node.prev_sibling();
    let mut next_start_row = node.start_position().row;

    while let Some(prev) = current {
        if prev.kind() != "comment" || prev.end_position().row + 1 != next_start_row {
            break;
        }

        if let Ok(text) = prev.utf8_text(source.as_bytes()) {
            let cleaned = clean_ruby_comment(text);
            lines.push(cleaned);
        }

        next_start_row = prev.start_position().row;
        current = prev.prev_sibling();
    }

    if lines.is_empty() {
        return None;
    }

    lines.reverse();
    let doc = lines.join("\n").trim().to_string();
    if doc.is_empty() {
        None
    } else {
        Some(doc)
    }
}

fn clean_ruby_comment(text: &str) -> String {
    text.lines()
        .map(|line| line.trim().trim_start_matches('#').trim())
        .collect::<Vec<_>>()
        .join("\n")
}
