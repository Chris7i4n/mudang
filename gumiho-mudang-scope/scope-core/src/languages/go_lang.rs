/// Go-specific metadata extraction and language plugin.
///
/// Extracts exported status (capitalized name = exported), method receiver info
/// (type, pointer vs value), return types (Go supports multiple returns),
/// and parameters from Go AST nodes.
///
/// Go methods are top-level `func (receiver) Name()` declarations — they are NOT
/// lexically nested inside structs. The receiver type is stored in metadata rather
/// than using `find_parent_class`, since there is no lexical nesting to walk.
use anyhow::Result;
use serde::Serialize;

/// Structured metadata for a Go symbol.
#[derive(Debug, Clone, Serialize, Default)]
pub struct GoMetadata {
    /// Whether the symbol name starts with an uppercase letter (exported).
    pub exported: bool,
    /// Method receiver type name (e.g. "Server", "*Server").
    pub receiver: Option<String>,
    /// Whether the method receiver is a pointer receiver.
    pub is_pointer_receiver: bool,
    /// Return types (Go supports multiple return values).
    pub return_types: Vec<String>,
    /// Parameter list with names and type annotations.
    pub parameters: Vec<GoParameterInfo>,
    /// The refined kind for type_spec nodes: "struct", "interface", or "type".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_kind: Option<String>,
}

/// Information about a single Go function/method parameter.
#[derive(Debug, Clone, Serialize)]
pub struct GoParameterInfo {
    /// Parameter name.
    pub name: String,
    /// Type annotation, if present.
    #[serde(rename = "type")]
    pub type_annotation: Option<String>,
}

/// Extract metadata from a Go AST node.
///
/// Returns a JSON string suitable for the `metadata` column.
pub fn extract_metadata(node: &tree_sitter::Node, source: &str, kind: &str) -> Result<String> {
    let mut meta = GoMetadata::default();

    // Determine exported status from the symbol name
    let name = extract_name_text(node, source);
    if let Some(ref n) = name {
        meta.exported = n.starts_with(|c: char| c.is_uppercase());
    }

    match kind {
        "method" => {
            extract_receiver_info(node, source, &mut meta);
            extract_parameters_from_func(node, source, &mut meta);
            extract_return_types(node, source, &mut meta);
        }
        "function" => {
            extract_parameters_from_func(node, source, &mut meta);
            extract_return_types(node, source, &mut meta);
        }
        "struct" => {
            // Refine struct vs interface vs type alias
            if let Some(type_child) = find_type_child(node) {
                match type_child.kind() {
                    "struct_type" => meta.type_kind = Some("struct".to_string()),
                    "interface_type" => meta.type_kind = Some("interface".to_string()),
                    _ => meta.type_kind = Some("type".to_string()),
                }
            }
        }
        _ => {}
    }

    let json = serde_json::to_string(&meta)?;
    Ok(json)
}

/// Extract the symbol name from a node.
fn extract_name_text(node: &tree_sitter::Node, source: &str) -> Option<String> {
    // For function_declaration: child_by_field_name("name")
    // For method_declaration: child_by_field_name("name")
    // For type_spec: child_by_field_name("name")
    // For const_spec: child_by_field_name("name")
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Find the type child of a type_spec node (struct_type, interface_type, etc.).
fn find_type_child<'a>(node: &'a tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
    // type_spec has a "type" field for the actual type (struct_type, interface_type, etc.)
    node.child_by_field_name("type")
}

/// Extract method receiver info from a method_declaration node.
fn extract_receiver_info(node: &tree_sitter::Node, source: &str, meta: &mut GoMetadata) {
    // method_declaration has a "receiver" field containing parameter_list
    if let Some(receiver_node) = node.child_by_field_name("receiver") {
        // The parameter_list contains a parameter_declaration
        let mut cursor = receiver_node.walk();
        for child in receiver_node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                // Extract the type from the parameter_declaration
                if let Some(type_node) = child.child_by_field_name("type") {
                    if let Ok(type_text) = type_node.utf8_text(source.as_bytes()) {
                        let type_text = type_text.trim();
                        if let Some(stripped) = type_text.strip_prefix('*') {
                            meta.receiver = Some(stripped.to_string());
                            meta.is_pointer_receiver = true;
                        } else {
                            meta.receiver = Some(type_text.to_string());
                            meta.is_pointer_receiver = false;
                        }
                    }
                }
            }
        }
    }
}

/// Extract parameters from a function_declaration or method_declaration.
fn extract_parameters_from_func(node: &tree_sitter::Node, source: &str, meta: &mut GoMetadata) {
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                // In Go, multiple names can share a type: (a, b int)
                // Extract all names and the type
                let type_text = child
                    .child_by_field_name("type")
                    .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.trim().to_string());

                let mut names = Vec::new();
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "identifier" {
                        if let Ok(name) = inner.utf8_text(source.as_bytes()) {
                            names.push(name.to_string());
                        }
                    }
                }

                if names.is_empty() {
                    // Unnamed parameter (just a type)
                    meta.parameters.push(GoParameterInfo {
                        name: String::new(),
                        type_annotation: type_text,
                    });
                } else {
                    for name in names {
                        meta.parameters.push(GoParameterInfo {
                            name,
                            type_annotation: type_text.clone(),
                        });
                    }
                }
            }
        }
    }
}

/// Extract return types from a function/method declaration.
fn extract_return_types(node: &tree_sitter::Node, source: &str, meta: &mut GoMetadata) {
    if let Some(result_node) = node.child_by_field_name("result") {
        match result_node.kind() {
            // Single return type (e.g. `func foo() error`)
            "type_identifier" | "pointer_type" | "slice_type" | "map_type" | "channel_type"
            | "qualified_type" | "array_type" | "interface_type" | "struct_type"
            | "function_type" => {
                if let Ok(text) = result_node.utf8_text(source.as_bytes()) {
                    meta.return_types.push(text.trim().to_string());
                }
            }
            // Multiple return types in a parameter_list (e.g. `func foo() (int, error)`)
            "parameter_list" => {
                let mut cursor = result_node.walk();
                for child in result_node.children(&mut cursor) {
                    if child.kind() == "parameter_declaration" {
                        // Named return: (result int, err error)
                        if let Some(type_node) = child.child_by_field_name("type") {
                            if let Ok(text) = type_node.utf8_text(source.as_bytes()) {
                                meta.return_types.push(text.trim().to_string());
                            }
                        }
                    } else if child.kind() == "type_identifier"
                        || child.kind() == "pointer_type"
                        || child.kind() == "slice_type"
                        || child.kind() == "qualified_type"
                    {
                        // Unnamed return: (int, error)
                        if let Ok(text) = child.utf8_text(source.as_bytes()) {
                            meta.return_types.push(text.trim().to_string());
                        }
                    }
                }
            }
            _ => {
                // Fallback: capture the raw text
                if let Ok(text) = result_node.utf8_text(source.as_bytes()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        meta.return_types.push(trimmed.to_string());
                    }
                }
            }
        }
    }
}
