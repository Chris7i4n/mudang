/// Rust-specific metadata extraction and language plugin.
///
/// Extracts visibility modifiers (pub, pub(crate), pub(super), private),
/// Rust-specific modifiers (async, const, unsafe), attributes, return type,
/// and parameters from Rust AST nodes.
use anyhow::Result;
use serde::Serialize;

use crate::extract::MetadataEntry;

/// Structured metadata for a Rust symbol.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RustMetadata {
    /// Visibility: "pub", "pub(crate)", "pub(super)", or "private".
    pub visibility: String,
    /// Whether the symbol is async.
    pub is_async: bool,
    /// Whether the symbol is const.
    pub is_const: bool,
    /// Whether the symbol is unsafe.
    pub is_unsafe: bool,
    /// Attributes applied to this symbol — reserved key `annotations`
    /// per `LANGUAGE-PLAYBOOK.md` § Step 5 (Rust attributes map to the
    /// playbook's universal "annotation" surface). Empty array means
    /// "looked, found none". Examples: `#[test]`, `#[derive(Debug)]`,
    /// `#[tokio::main]`.
    #[serde(rename = "annotations")]
    pub annotations: Vec<MetadataEntry>,
    /// Return type, if present.
    pub return_type: Option<String>,
    /// Parameter list with names and types.
    pub parameters: Vec<RustParameterInfo>,
}

/// Information about a single Rust function/method parameter.
#[derive(Debug, Clone, Serialize)]
pub struct RustParameterInfo {
    /// Parameter name.
    pub name: String,
    /// Type annotation, if present.
    #[serde(rename = "type")]
    pub type_annotation: Option<String>,
    /// Whether the parameter binding is mutable.
    pub is_mutable: bool,
}

/// Extract metadata from a Rust AST node.
///
/// Returns a JSON string suitable for the `metadata` column.
pub fn extract_metadata(node: &tree_sitter::Node, source: &str, kind: &str) -> Result<String> {
    let mut meta = RustMetadata {
        visibility: "private".to_string(),
        ..Default::default()
    };

    // Walk direct children to find modifiers and attributes
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        if child.kind() == "visibility_modifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                meta.visibility = match text.trim() {
                    "pub" => "pub".to_string(),
                    s if s.starts_with("pub(crate)") => "pub(crate)".to_string(),
                    s if s.starts_with("pub(super)") => "pub(super)".to_string(),
                    s if s.starts_with("pub") => s.to_string(),
                    _ => "private".to_string(),
                };
            }
        }
    }

    // Rust attributes attach as PRECEDING SIBLINGS of the item node
    // (`function_item`, `struct_item`, `enum_variant`, …) in tree-sitter-rust,
    // not as direct children. Walk the prev_sibling chain, accepting
    // `attribute_item` runs and skipping interleaved doc comments — both
    // shapes are legal between attrs and the item they apply to. Stop at
    // any other sibling kind (which marks the previous item / declaration
    // boundary).
    let mut prev_attrs: Vec<MetadataEntry> = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        match s.kind() {
            "attribute_item" => {
                if let Ok(text) = s.utf8_text(source.as_bytes()) {
                    let entry = parse_rust_attribute(text);
                    if let Some(entry) = entry {
                        prev_attrs.push(entry);
                    }
                }
            }
            "line_comment" | "block_comment" => {
                // Doc / regular comments are transparent between attrs and
                // the item they decorate; keep walking.
            }
            _ => break,
        }
        sibling = s.prev_sibling();
    }
    // We walked backwards; reverse so source order is preserved in the
    // annotations array.
    prev_attrs.reverse();
    meta.annotations.extend(prev_attrs);

    // Check for async, const, unsafe keywords in function items
    if kind == "function" || kind == "method" {
        let mut fn_cursor = node.walk();
        for child in node.children(&mut fn_cursor) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                match text {
                    "async" => meta.is_async = true,
                    "const" => meta.is_const = true,
                    "unsafe" => meta.is_unsafe = true,
                    _ => {}
                }
            }
        }

        // Extract return type
        if let Some(return_node) = node.child_by_field_name("return_type") {
            if let Ok(text) = return_node.utf8_text(source.as_bytes()) {
                // Strip the leading `-> ` from return types
                let clean = text.trim_start_matches("->").trim();
                if !clean.is_empty() {
                    meta.return_type = Some(clean.to_string());
                }
            }
        }

        // Extract parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            meta.parameters = extract_parameters(&params_node, source);
        }
    }

    // For const/static items, mark is_const
    if kind == "const" {
        meta.is_const = true;
    }

    let json = serde_json::to_string(&meta)?;
    Ok(json)
}

/// Parse one Rust `attribute_item` text into a [`MetadataEntry`].
///
/// Strips the `#[…]` (outer) or `#![…]` (inner) wrapping, then splits on
/// the first `(` to separate the attribute path from its argument list.
/// `#[derive(Debug)]` becomes `{name: "derive", args_text: Some("(Debug)")}`;
/// `#[tokio::main]` becomes `{name: "tokio::main", args_text: None}`.
/// Returns `None` when the parsed name is empty (malformed input).
fn parse_rust_attribute(text: &str) -> Option<MetadataEntry> {
    let stripped = text
        .trim()
        .trim_start_matches('#')
        .trim_start_matches('!')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    let (name, args_text) = match stripped.find('(') {
        Some(idx) => (
            stripped[..idx].trim().to_string(),
            Some(stripped[idx..].trim().to_string()),
        ),
        None => (stripped.to_string(), None),
    };
    if name.is_empty() {
        None
    } else {
        Some(MetadataEntry { name, args_text })
    }
}

/// Extract parameter info from a parameters node.
fn extract_parameters(params_node: &tree_sitter::Node, source: &str) -> Vec<RustParameterInfo> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                let mut name = String::new();
                let mut type_annotation = None;
                let mut is_mutable = false;

                // Extract pattern (name) and type
                if let Some(pattern_node) = child.child_by_field_name("pattern") {
                    if let Ok(text) = pattern_node.utf8_text(source.as_bytes()) {
                        let text = text.trim();
                        if let Some(stripped) = text.strip_prefix("mut ") {
                            name = stripped.to_string();
                            is_mutable = true;
                        } else {
                            name = text.to_string();
                        }
                    }
                }

                if let Some(type_node) = child.child_by_field_name("type") {
                    if let Ok(text) = type_node.utf8_text(source.as_bytes()) {
                        type_annotation = Some(text.trim().to_string());
                    }
                }

                if !name.is_empty() {
                    params.push(RustParameterInfo {
                        name,
                        type_annotation,
                        is_mutable,
                    });
                }
            }
            "self_parameter" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    let text = text.trim();
                    let is_mutable = text.contains("mut");
                    params.push(RustParameterInfo {
                        name: "self".to_string(),
                        type_annotation: None,
                        is_mutable,
                    });
                }
            }
            _ => {}
        }
    }

    params
}

#[cfg(test)]
mod rust_annotation_tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_and_extract_first_fn_metadata(source: &str) -> serde_json::Value {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the first function_item / struct_item node by walking children.
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "function_item" || child.kind() == "struct_item" {
                let kind = if child.kind() == "function_item" {
                    "function"
                } else {
                    "struct"
                };
                let json = extract_metadata(&child, source, kind).unwrap();
                return serde_json::from_str(&json).unwrap();
            }
        }
        panic!("no function_item or struct_item found in source");
    }

    #[test]
    fn outer_attribute_on_function_captured_as_annotation() {
        let source = "#[test]\nfn smoke() {}";
        let meta = parse_and_extract_first_fn_metadata(source);
        let annotations = meta.get("annotations").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            annotations.len(),
            1,
            "expected exactly one annotation, got {meta:?}"
        );
        assert_eq!(
            annotations[0].get("name").and_then(|v| v.as_str()),
            Some("test")
        );
        // No args — args_text key should be omitted.
        assert!(
            annotations[0].get("args_text").is_none(),
            "args-less attribute must omit args_text; got {:?}",
            annotations[0]
        );
    }

    #[test]
    fn derive_attribute_captures_args_text() {
        let source = "#[derive(Debug, Clone)]\nstruct S;";
        let meta = parse_and_extract_first_fn_metadata(source);
        let annotations = meta.get("annotations").and_then(|v| v.as_array()).unwrap();
        assert_eq!(annotations.len(), 1);
        assert_eq!(
            annotations[0].get("name").and_then(|v| v.as_str()),
            Some("derive")
        );
        assert_eq!(
            annotations[0].get("args_text").and_then(|v| v.as_str()),
            Some("(Debug, Clone)")
        );
    }

    #[test]
    fn multiple_attrs_preserve_source_order() {
        let source = "#[cfg(test)]\n#[allow(dead_code)]\nfn x() {}";
        let meta = parse_and_extract_first_fn_metadata(source);
        let annotations = meta.get("annotations").and_then(|v| v.as_array()).unwrap();
        assert_eq!(annotations.len(), 2);
        assert_eq!(
            annotations[0].get("name").and_then(|v| v.as_str()),
            Some("cfg")
        );
        assert_eq!(
            annotations[1].get("name").and_then(|v| v.as_str()),
            Some("allow")
        );
    }

    #[test]
    fn doc_comments_between_attrs_are_transparent() {
        // /// doc lines interleaved with attrs: both attrs should still be captured.
        let source = "#[test]\n/// docs for smoke\nfn smoke() {}";
        let meta = parse_and_extract_first_fn_metadata(source);
        let annotations = meta.get("annotations").and_then(|v| v.as_array()).unwrap();
        assert_eq!(
            annotations.len(),
            1,
            "doc comment must not block prev-sibling walk; got {meta:?}"
        );
        assert_eq!(
            annotations[0].get("name").and_then(|v| v.as_str()),
            Some("test")
        );
    }

    #[test]
    fn item_with_no_attrs_emits_empty_annotations() {
        let source = "fn plain() {}";
        let meta = parse_and_extract_first_fn_metadata(source);
        let annotations = meta.get("annotations").and_then(|v| v.as_array()).unwrap();
        assert!(annotations.is_empty(), "expected [], got {annotations:?}");
    }
}
