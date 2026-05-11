/// Ruby-specific language plugin.
///
/// Extracts Ruby symbols, lexical parent relationships, and v1 metadata:
/// method visibility regions, singleton receivers, namespaces, doc comments,
/// endless-method markers, and common parameter forms.
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use tree_sitter::Language;

use crate::types::Edge;
use crate::parser::SupportedLanguage;
use crate::languages::{make_edge, resolve_scope_id, LanguagePlugin};

/// Ruby language plugin.
pub struct RubyPlugin;

impl LanguagePlugin for RubyPlugin {
    fn language(&self) -> SupportedLanguage {
        SupportedLanguage::Ruby
    }

    fn extensions(&self) -> &[&str] {
        &["rb"]
    }

    fn ts_language(&self) -> Language {
        tree_sitter_ruby::LANGUAGE.into()
    }

    fn symbol_query_source(&self) -> &str {
        include_str!("../queries/ruby/symbols.scm")
    }

    fn edge_query_source(&self) -> &str {
        include_str!("../queries/ruby/edges.scm")
    }

    fn infer_symbol_kind(&self, node_kind: &str) -> &str {
        match node_kind {
            "class" => "class",
            "module" => "interface",
            "method" | "singleton_method" => "method",
            "assignment" => "const",
            "lambda" | "call" => "function",
            _ => "class",
        }
    }

    fn scope_node_types(&self) -> &[&str] {
        &[
            "class",
            "module",
            "method",
            "singleton_method",
            "lambda",
            "call",
        ]
    }

    fn class_body_node_types(&self) -> &[&str] {
        &["body_statement"]
    }

    fn class_decl_node_types(&self) -> &[&str] {
        &["class", "module"]
    }

    fn extract_metadata(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        kind: &str,
    ) -> Result<String> {
        extract_metadata(node, source, kind)
    }

    fn extract_edge(
        &self,
        pattern_index: usize,
        captures: &HashMap<String, (String, u32)>,
        file_path: &str,
        enclosing_scope_id: Option<&str>,
    ) -> Vec<Edge> {
        extract_ruby_edge(pattern_index, captures, file_path, enclosing_scope_id)
    }

    fn generic_name_stopwords(&self) -> &[&str] {
        &[
            "new",
            "initialize",
            "call",
            "to_s",
            "inspect",
            "class",
            "module",
        ]
    }

    fn extract_docstring(&self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        extract_docstring(node, source)
    }
}

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
            infer_visibility(node, source)
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

fn infer_visibility(node: &tree_sitter::Node, source: &str) -> String {
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

fn extract_docstring(node: &tree_sitter::Node, source: &str) -> Option<String> {
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

/// Ruby edge extraction by semantic capture name.
///
/// The extractor validates call names explicitly instead of relying only on
/// tree-sitter query predicates. That keeps command output stable if a broad
/// structural capture also matches a Ruby metaprogramming or receiver-call form.
fn extract_ruby_edge(
    _pattern: usize,
    captures: &HashMap<String, (String, u32)>,
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<Edge> {
    let mut edges = Vec::new();

    let from_scope = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_class = resolve_scope_id(enclosing_scope_id, file_path, "class");
    let from_module = format!("{file_path}::__module__::function");

    // require/require_relative/load/autoload — always module-level imports.
    if let (Some((method, _)), Some((path, line)), Some((call_text, _))) = (
        captures.get("import.method"),
        captures.get("import.path"),
        captures.get("import.call"),
    ) {
        if matches!(
            method.as_str(),
            "require" | "require_relative" | "load" | "autoload"
        ) && is_plain_ruby_call(call_text, method)
        {
            edges.push(make_edge(
                from_module,
                clean_ruby_literal(path),
                "imports",
                file_path,
                *line,
            ));
        }
        return edges;
    }

    // Instantiation via `.new`.
    if let (Some((class_name, line)), Some((method, _))) = (
        captures.get("instantiate.class"),
        captures.get("instantiate.method"),
    ) {
        let class_name = clean_ruby_edge_name(class_name);
        if method == "new" && !class_name.is_empty() {
            edges.push(make_edge(
                from_scope,
                class_name,
                "instantiates",
                file_path,
                *line,
            ));
        }
        return edges;
    }

    // Class inheritance.
    if let Some((parent, line)) = captures.get("extends.parent") {
        let parent = clean_ruby_edge_name(parent);
        if !parent.is_empty() {
            edges.push(make_edge(from_class, parent, "extends", file_path, *line));
        }
        return edges;
    }

    // Mixins: include/prepend/extend.
    if let (Some((method, _)), Some((module_name, line)), Some((call_text, _))) = (
        captures.get("implements.method"),
        captures.get("implements.module"),
        captures.get("implements.call"),
    ) {
        if matches!(method.as_str(), "include" | "prepend" | "extend")
            && is_plain_ruby_call(call_text, method)
        {
            edges.push(make_edge(
                from_class,
                clean_ruby_edge_name(module_name),
                "implements",
                file_path,
                *line,
            ));
        }
        return edges;
    }

    // Constant/type references in selected expression positions.
    if let Some((type_name, line)) = captures.get("type.name") {
        let type_name = clean_ruby_edge_name(type_name);
        if !type_name.is_empty() {
            edges.push(make_edge(
                from_scope,
                type_name,
                "references_type",
                file_path,
                *line,
            ));
        }
        return edges;
    }

    // Literal send/public_send/define_method/const_get.
    if let (Some((method, line)), Some((literal, _))) =
        (captures.get("meta.method"), captures.get("meta.literal"))
    {
        let literal = clean_ruby_literal(literal);
        if literal.is_empty() || is_dynamic_ruby_literal(&literal) {
            return edges;
        }

        match method.as_str() {
            "send" | "public_send" | "__send__" => {
                edges.push(make_edge(from_scope, literal, "calls", file_path, *line));
            }
            "define_method" => {
                edges.push(make_edge(
                    from_scope,
                    literal,
                    "references",
                    file_path,
                    *line,
                ));
            }
            "const_get" => {
                edges.push(make_edge(
                    from_scope,
                    literal,
                    "references_type",
                    file_path,
                    *line,
                ));
            }
            _ => {}
        }
        return edges;
    }

    // super
    if let Some((_, line)) = captures.get("ruby.super") {
        edges.push(make_edge(from_scope, "super", "calls", file_path, *line));
        return edges;
    }

    // yield
    if let Some((_, line)) = captures.get("ruby.yield") {
        edges.push(make_edge(from_scope, "yield", "calls", file_path, *line));
        return edges;
    }

    // Receiver call.
    if let (Some((receiver, line)), Some((method, _)), Some((call_text, _))) = (
        captures.get("call.receiver"),
        captures.get("call.name"),
        captures.get("call.node"),
    ) {
        let receiver = clean_ruby_edge_name(receiver);
        let method = clean_ruby_edge_name(method);
        if !receiver.is_empty()
            && !method.is_empty()
            && method != "new"
            && !is_reserved_edge_call(&method)
            && !is_plain_ruby_call(call_text, &method)
        {
            edges.push(make_edge(
                from_scope,
                format!("{receiver}.{method}"),
                "calls",
                file_path,
                *line,
            ));
        }
        return edges;
    }

    // Direct call. The query also structurally matches receiver calls, so
    // validate the full call text and skip cases handled by narrower patterns.
    if let (Some((callee, line)), Some((call_text, _))) =
        (captures.get("call.name"), captures.get("call.node"))
    {
        let callee = clean_ruby_edge_name(callee);
        if !callee.is_empty()
            && !is_reserved_edge_call(&callee)
            && is_plain_ruby_call(call_text, &callee)
        {
            edges.push(make_edge(from_scope, callee, "calls", file_path, *line));
        }
    }

    edges
}

fn is_reserved_edge_call(name: &str) -> bool {
    matches!(
        name,
        "require"
            | "require_relative"
            | "load"
            | "autoload"
            | "include"
            | "prepend"
            | "extend"
            | "send"
            | "public_send"
            | "__send__"
            | "define_method"
            | "const_get"
            | "private"
            | "protected"
            | "public"
    )
}

fn is_plain_ruby_call(call_text: &str, method: &str) -> bool {
    call_text.trim_start().starts_with(method)
}

fn clean_ruby_literal(text: &str) -> String {
    let cleaned = text
        .trim()
        .trim_start_matches(':')
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();

    clean_ruby_edge_name(&cleaned)
}

fn clean_ruby_edge_name(text: &str) -> String {
    text.trim().trim_start_matches("::").trim().to_string()
}

fn is_dynamic_ruby_literal(text: &str) -> bool {
    text.contains("#{") || text.contains('\\') || text.contains('\n')
}
