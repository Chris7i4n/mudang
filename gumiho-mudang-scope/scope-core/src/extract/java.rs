//! Java edge extraction (R2).
//!
//! Relocated from `crate::languages::java`; per-language modules retain
//! only capture / metadata / docstring concerns. The extractor is the
//! only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated the
//! input slice from `HashMap<String, (String, u32)>` to `&[Capture]`
//! and adopted explicit per-pattern `pattern_id`s.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// Java edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/java/edges.scm`:
/// `0` import declaration, `1` member method call, `2` direct method
/// call, `3` `this.method()`, `4` object creation (new), `5` extends
/// (superclass), `6` class implements, `7` interface extends, `8`
/// field type ref, `9` param type ref, `10` `super.method()`, `11`
/// switch-case enum constant.
pub fn extract_java_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_cls = resolve_scope_id(enclosing_scope_id, file_path, "class");

    match pattern {
        // Import declaration
        0 => {
            if let Some(imported_name) = find_capture(captures, "imported_name") {
                edges.push(make_edge(
                    format!("{file_path}::__module__::function"),
                    &imported_name.text,
                    "imports",
                    "imports.scoped",
                    file_path,
                    imported_name.start_line,
                ));
            }
        }
        // Member method invocation (e.g. service.processPayment())
        1 => {
            if let (Some(object), Some(method)) = (
                find_capture(captures, "object"),
                find_capture(captures, "method"),
            ) {
                edges.push(make_edge(
                    from_fn.clone(),
                    format!("{}.{}", object.text, method.text),
                    "calls",
                    "calls.method",
                    file_path,
                    object.start_line,
                ));
            }
        }
        // Direct method invocation (e.g. processPayment())
        2 => {
            if let Some(callee) = find_capture(captures, "callee") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &callee.text,
                    "calls",
                    "calls.function",
                    file_path,
                    callee.start_line,
                ));
            }
        }
        // this.method() / super.method() — captures method name only.
        3 | 10 => {
            if let Some(method) = find_capture(captures, "method") {
                let pattern_id = if pattern == 3 {
                    "calls.method.this"
                } else {
                    "calls.method.super"
                };
                edges.push(make_edge(
                    from_fn.clone(),
                    &method.text,
                    "calls",
                    pattern_id,
                    file_path,
                    method.start_line,
                ));
            }
        }
        // Object creation (new Foo())
        4 => {
            if let Some(class_name) = find_capture(captures, "class_name") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &class_name.text,
                    "instantiates",
                    "instantiates.class",
                    file_path,
                    class_name.start_line,
                ));
            }
        }
        // Superclass (extends)
        5 => {
            if let Some(base_type) = find_capture(captures, "base_type") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_type.text,
                    "extends",
                    "extends.class",
                    file_path,
                    base_type.start_line,
                ));
            }
        }
        // Class implements
        6 => {
            if let Some(base_type) = find_capture(captures, "base_type") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_type.text,
                    "implements",
                    "implements.interface",
                    file_path,
                    base_type.start_line,
                ));
            }
        }
        // Interface extends
        7 => {
            if let Some(base_type) = find_capture(captures, "base_type") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_type.text,
                    "extends",
                    "extends.interface",
                    file_path,
                    base_type.start_line,
                ));
            }
        }
        // Field / parameter type reference
        8 | 9 => {
            if let Some(type_ref) = find_capture(captures, "type_ref") {
                let pattern_id = if pattern == 8 {
                    "references_type.field"
                } else {
                    "references_type.param"
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
        // Switch case label referencing an enum constant (e.g. case SUCCESS:)
        11 => {
            if let Some(variant_ref) = find_capture(captures, "variant_ref") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &variant_ref.text,
                    "references",
                    "references.switch.enum",
                    file_path,
                    variant_ref.start_line,
                ));
            }
        }
        _ => {}
    }

    edges
}
