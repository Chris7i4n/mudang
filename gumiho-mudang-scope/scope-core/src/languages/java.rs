/// Java-specific metadata extraction and language plugin.
///
/// Extracts access modifiers (public, protected, private, package-private),
/// Java-specific modifiers (static, final, abstract, synchronized),
/// annotations, return type, parameters, and throws declarations from
/// Java AST nodes.
use anyhow::Result;
use serde::Serialize;

use crate::extract::MetadataEntry;

/// Structured metadata for a Java symbol.
#[derive(Debug, Clone, Serialize, Default)]
pub struct JavaMetadata {
    /// Access modifier: "public", "protected", "private", or "package".
    pub access: String,
    /// Whether the symbol is static.
    pub is_static: bool,
    /// Whether the symbol is final.
    pub is_final: bool,
    /// Whether the symbol is abstract.
    pub is_abstract: bool,
    /// Whether the symbol is synchronized.
    pub is_synchronized: bool,
    /// Annotations on this symbol — reserved key per
    /// `LANGUAGE-PLAYBOOK.md` § Step 5. Empty array means "looked, found
    /// none". Java's AST exposes `marker_annotation` and `annotation`
    /// nodes so the key is always present (struct-field presence pins
    /// the contract).
    pub annotations: Vec<MetadataEntry>,
    /// Return type, if present (for methods).
    pub return_type: Option<String>,
    /// Parameter list with names and types.
    pub parameters: Vec<JavaParameterInfo>,
    /// Checked exceptions declared in throws clause.
    pub throws: Vec<String>,
}

/// Information about a single Java method/constructor parameter.
#[derive(Debug, Clone, Serialize)]
pub struct JavaParameterInfo {
    /// Parameter name.
    pub name: String,
    /// Type annotation, if present.
    #[serde(rename = "type")]
    pub type_annotation: Option<String>,
    /// Whether the parameter is declared final.
    pub is_final: bool,
}

/// Extract metadata from a Java AST node.
///
/// Returns a JSON string suitable for the `metadata` column.
pub fn extract_metadata(node: &tree_sitter::Node, source: &str, kind: &str) -> Result<String> {
    let mut meta = JavaMetadata::default();

    // Walk direct children to find modifiers
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        if child.kind() == "modifiers" {
            let mut mod_cursor = child.walk();
            for mod_child in child.children(&mut mod_cursor) {
                match mod_child.kind() {
                    "public" => meta.access = "public".to_string(),
                    "protected" => meta.access = "protected".to_string(),
                    "private" => meta.access = "private".to_string(),
                    "static" => meta.is_static = true,
                    "final" => meta.is_final = true,
                    "abstract" => meta.is_abstract = true,
                    "synchronized" => meta.is_synchronized = true,
                    "marker_annotation" | "annotation" => {
                        if let Ok(text) = mod_child.utf8_text(source.as_bytes()) {
                            // Strip leading `@`; split out args if present.
                            // `@Deprecated`          → name="Deprecated", args_text=None
                            // `@RequestMapping("/x")` → name="RequestMapping",
                            //                          args_text=Some(`("/x")`)
                            let stripped = text.trim_start_matches('@').trim();
                            let (ann_name, args_text) = match stripped.find('(') {
                                Some(idx) => (
                                    stripped[..idx].trim().to_string(),
                                    Some(stripped[idx..].trim().to_string()),
                                ),
                                None => (stripped.to_string(), None),
                            };
                            if !ann_name.is_empty() {
                                meta.annotations.push(MetadataEntry {
                                    name: ann_name,
                                    args_text,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Default access if none was set — Java defaults to package-private
    if meta.access.is_empty() {
        meta.access = "package".to_string();
    }

    // Extract return type (for method_declaration)
    if kind == "method" {
        if let Some(type_node) = node.child_by_field_name("type") {
            if let Ok(text) = type_node.utf8_text(source.as_bytes()) {
                meta.return_type = Some(text.trim().to_string());
            }
        }
    }

    // Extract parameters
    if kind == "method" {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            meta.parameters = extract_parameters(&params_node, source);
        }
    }

    // Extract throws clause
    let mut throws_cursor = node.walk();
    for child in node.children(&mut throws_cursor) {
        if child.kind() == "throws" {
            let mut tc = child.walk();
            for throw_child in child.children(&mut tc) {
                if throw_child.kind() == "type_identifier" {
                    if let Ok(text) = throw_child.utf8_text(source.as_bytes()) {
                        meta.throws.push(text.trim().to_string());
                    }
                }
            }
        }
    }

    let json = serde_json::to_string(&meta)?;
    Ok(json)
}

/// Extract parameter info from a formal_parameters node.
fn extract_parameters(params_node: &tree_sitter::Node, source: &str) -> Vec<JavaParameterInfo> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or_default()
                .to_string();

            let type_annotation = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|t| t.trim().to_string());

            // Check for final modifier on the parameter
            let mut is_final = false;
            let mut param_cursor = child.walk();
            for param_child in child.children(&mut param_cursor) {
                if param_child.kind() == "modifiers" {
                    let mut mc = param_child.walk();
                    for m in param_child.children(&mut mc) {
                        if m.kind() == "final" {
                            is_final = true;
                        }
                    }
                }
            }

            if !name.is_empty() {
                params.push(JavaParameterInfo {
                    name,
                    type_annotation,
                    is_final,
                });
            }
        }
    }

    params
}
