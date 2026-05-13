//! Rust edge extraction (R2).
//!
//! Relocated from `crate::languages::rust_lang`; per-language modules
//! retain only capture / metadata / docstring concerns. The extractor is
//! the only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated
//! the input slice from `HashMap<String, (String, u32)>` to
//! `&[Capture]` and adopted explicit per-pattern `pattern_id`s.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};
use crate::types::Symbol;

/// Rust edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/rust/edges.scm`:
/// `0` use scoped, `1` use aliased, `2` direct call, `3` scoped call,
/// `4` method call, `5` macro invocation, `6` scoped macro,
/// `7` field type ref, `8` param type ref, `9` return type ref,
/// `10` match arm struct pattern variant ref,
/// `11` match arm tuple struct pattern variant ref.
pub fn extract_rust_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let module_fn = || format!("{file_path}::__module__::function");

    match pattern {
        // Use declaration — scoped identifier (e.g. use std::io)
        // Use declaration — aliased (use ... as ...)
        0 | 1 => {
            if let Some(imported_name) = find_capture(captures, "imported_name") {
                let pattern_id = if pattern == 0 {
                    "imports.path"
                } else {
                    "imports.aliased"
                };
                edges.push(make_edge(
                    module_fn(),
                    &imported_name.text,
                    "imports",
                    pattern_id,
                    file_path,
                    imported_name.start_line,
                ));
            }
        }
        // Direct call expression (e.g. process_payment(...))
        // Scoped call expression (e.g. PaymentService::new(...))
        2 | 3 => {
            if let Some(callee) = find_capture(captures, "callee") {
                let pattern_id = if pattern == 2 {
                    "calls.function"
                } else {
                    "calls.function.scoped"
                };
                edges.push(make_edge(
                    from_fn.clone(),
                    &callee.text,
                    "calls",
                    pattern_id,
                    file_path,
                    callee.start_line,
                ));
            }
        }
        // Method call expression (e.g. self.client.charge(...))
        4 => {
            if let Some(method) = find_capture(captures, "method") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &method.text,
                    "calls",
                    "calls.method",
                    file_path,
                    method.start_line,
                ));
            }
        }
        // Macro invocation (e.g. println!(...))
        // Scoped macro invocation (e.g. std::println!(...))
        5 | 6 => {
            if let Some(macro_name) = find_capture(captures, "macro_name") {
                let pattern_id = if pattern == 5 {
                    "calls.macro"
                } else {
                    "calls.macro.scoped"
                };
                edges.push(make_edge(
                    from_fn.clone(),
                    format!("{}!", macro_name.text),
                    "calls",
                    pattern_id,
                    file_path,
                    macro_name.start_line,
                ));
            }
        }
        // Field / parameter / return type reference.
        // Skip single uppercase letters — these are almost always generic type
        // parameters (T, U, K, V, F, R, S, E, B, ...), not real type references.
        // Tree-sitter Rust treats both generic param names and concrete type
        // references as `type_identifier`, so the query cannot distinguish them
        // syntactically. Filtering here removes ~24% of false positives in
        // `references_type` on tokio.
        7..=9 => {
            if let Some(type_ref) = find_capture(captures, "type_ref") {
                if !is_likely_generic_param(&type_ref.text) {
                    let pattern_id = match pattern {
                        7 => "references_type.field",
                        8 => "references_type.param",
                        _ => "references_type.return",
                    };
                    edges.push(make_edge(
                        from_fn.clone(),
                        &type_ref.text,
                        "references_type",
                        pattern_id,
                        file_path,
                        type_ref.start_line,
                    ));
                }
            }
        }
        // Match arm — struct pattern variant ref (e.g. PaymentResult::Success { .. })
        // Match arm — tuple struct pattern variant ref (e.g. PaymentMethod::CreditCard(details))
        10 | 11 => {
            if let Some(variant_ref) = find_capture(captures, "variant_ref") {
                let variant_name = variant_ref
                    .text
                    .rsplit("::")
                    .next()
                    .unwrap_or(&variant_ref.text);
                let pattern_id = if pattern == 10 {
                    "references.match.struct"
                } else {
                    "references.match.tuple"
                };
                edges.push(make_edge(
                    from_fn.clone(),
                    variant_name,
                    "references",
                    pattern_id,
                    file_path,
                    variant_ref.start_line,
                ));
            }
        }
        _ => {}
    }

    edges
}

/// Heuristic: a single uppercase ASCII letter is almost always a Rust generic
/// type parameter, not a real type reference. Real Rust types are PascalCase
/// multi-letter identifiers (Vec, Option, HashMap). This filter trades a small
/// amount of recall (real one-letter types are vanishingly rare in idiomatic
/// Rust APIs) for a large precision win on `references_type`.
fn is_likely_generic_param(name: &str) -> bool {
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_uppercase(),
        _ => false,
    }
}

/// Extract `implements` edges from Rust `impl Trait for Type` blocks.
///
/// Relocated from `parser.rs` R2 keeps
/// EdgeKind selection inside the extractor. Walks the parsed tree for
/// `impl_item` nodes that have a `trait` field and emits one `implements`
/// `RawEdge` per such block, resolving the `from_id` against the previously
/// extracted symbols list (so the edge points at the target type's symbol,
/// not a synthetic module-level placeholder when possible).
pub fn extract_rust_trait_impl_edges(
    symbols: &[Symbol],
    tree: &tree_sitter::Tree,
    source: &str,
    file_path: &str,
) -> Vec<RawEdge> {
    let root = tree.root_node();
    let mut tree_cursor = root.walk();
    let mut edges = Vec::new();

    for child in root.children(&mut tree_cursor) {
        if child.kind() != "impl_item" {
            continue;
        }

        // Only process trait impls (`impl Trait for Type`).
        let trait_node = match child.child_by_field_name("trait") {
            Some(node) => node,
            None => continue,
        };

        let trait_name = match extract_base_type_name(&trait_node, source) {
            Some(name) => name,
            None => continue,
        };

        let target_type_name = match extract_impl_target_type(&child, source) {
            Some(name) => name,
            None => continue,
        };

        let line = child.start_position().row as u32 + 1;

        // Resolve `from_id` against the symbols list when possible. Falls back
        // to a synthetic module-level ID when the target type is defined in
        // another file (cross-file targets resolve through the R3 resolver).
        let from_id = symbols
            .iter()
            .find(|s| {
                s.file_path == file_path
                    && s.name == target_type_name
                    && (s.kind == "struct" || s.kind == "enum" || s.kind == "interface")
            })
            .map(|s| s.id.clone())
            .unwrap_or_else(|| format!("{file_path}::__module__::class"));

        edges.push(make_edge(
            from_id,
            trait_name,
            "implements",
            "implements.trait_impl_block",
            file_path,
            line,
        ));
    }

    edges
}

/// Extract the target type name from a Rust `impl_item` node.
///
/// For `impl Type { ... }`, returns `Type`. For `impl Trait for Type { ... }`,
/// returns `Type` (after `for`). For `impl<T> Type<T> { ... }`, returns `Type`
/// (strips generic params).
///
/// Exposed `pub(crate)` so `parser.rs::associate_rust_impl_methods` (which
/// does symbol parent-id management, not edge extraction) can reuse it.
pub(crate) fn extract_impl_target_type(
    impl_node: &tree_sitter::Node,
    source: &str,
) -> Option<String> {
    let type_node = impl_node.child_by_field_name("type")?;
    extract_base_type_name(&type_node, source)
}

/// Extract the base type name from a type node, stripping generic parameters.
///
/// `Foo<T>` → `Foo`; `Foo` → `Foo`; `path::Type` → `Type` (last segment).
fn extract_base_type_name(type_node: &tree_sitter::Node, source: &str) -> Option<String> {
    match type_node.kind() {
        "type_identifier" => {
            let text = type_node.utf8_text(source.as_bytes()).ok()?;
            Some(text.to_string())
        }
        "generic_type" => {
            let mut cursor = type_node.walk();
            for child in type_node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    let text = child.utf8_text(source.as_bytes()).ok()?;
                    return Some(text.to_string());
                }
            }
            None
        }
        "scoped_type_identifier" => {
            let text = type_node.utf8_text(source.as_bytes()).ok()?;
            text.rsplit("::").next().map(|s| s.to_string())
        }
        _ => {
            let text = type_node.utf8_text(source.as_bytes()).ok()?;
            Some(text.to_string())
        }
    }
}

#[cfg(test)]
mod generic_param_tests {
    use super::is_likely_generic_param;

    #[test]
    fn single_uppercase_letters_are_generic() {
        for c in ["T", "U", "K", "V", "F", "R", "S", "E", "B"] {
            assert!(is_likely_generic_param(c), "{c} should be flagged");
        }
    }

    #[test]
    fn real_types_are_not_generic() {
        for c in [
            "Vec", "Option", "HashMap", "Tt", "Future", "Item", "Output", "Self",
        ] {
            assert!(!is_likely_generic_param(c), "{c} should not be flagged");
        }
    }
}
