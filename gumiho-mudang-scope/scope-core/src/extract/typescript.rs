//! TypeScript edge extraction (R2).
//!
//! Relocated from `crate::languages::typescript`; per-language modules
//! retain only capture / metadata / docstring concerns. The extractor is
//! the only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated
//! the input slice from `HashMap<String, (String, u32)>` to
//! `&[Capture]` and adopted explicit per-pattern `pattern_id`s in place
//! of the `legacy.<kind>` placeholder.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// TypeScript edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/typescript/edges.scm`:
/// `0` import, `1` direct call, `2` member call, `3` chained member
/// call, `4` new expression, `5` extends, `6` implements, `7`
/// `this.method()` call, `8` type reference.
pub fn extract_ts_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_cls = resolve_scope_id(enclosing_scope_id, file_path, "class");

    match pattern {
        // Import statement — always module-level, use __module__ synthetic ID
        0 => {
            if let (Some(imported_name), Some(source)) = (
                find_capture(captures, "imported_name"),
                find_capture(captures, "source"),
            ) {
                let source_clean = source.text.trim_matches(|c| c == '\'' || c == '"');
                edges.push(make_edge(
                    format!("{file_path}::__module__::function"),
                    format!("{source_clean}::{}", imported_name.text),
                    "imports",
                    "imports.named",
                    file_path,
                    imported_name.start_line,
                ));
            }
        }
        // Direct call expression
        1 => {
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
        // Member call expression (obj.method())
        2 => {
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
        // Chained member access call (a.b.method() / this.svc.method())
        3 => {
            if let (Some(object), Some(method)) = (
                find_capture(captures, "object"),
                find_capture(captures, "method"),
            ) {
                edges.push(make_edge(
                    from_fn.clone(),
                    format!("{}.{}", object.text, method.text),
                    "calls",
                    "calls.method.chained",
                    file_path,
                    object.start_line,
                ));
            }
        }
        // New expression (instantiation)
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
        // Extends clause
        5 => {
            if let Some(base_class) = find_capture(captures, "base_class") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_class.text,
                    "extends",
                    "extends.class",
                    file_path,
                    base_class.start_line,
                ));
            }
        }
        // Implements clause
        6 => {
            if let Some(iface_name) = find_capture(captures, "interface_name") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &iface_name.text,
                    "implements",
                    "implements.interface",
                    file_path,
                    iface_name.start_line,
                ));
            }
        }
        // this.method() call — captures method name only
        7 => {
            if let Some(method) = find_capture(captures, "method") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &method.text,
                    "calls",
                    "calls.method.this",
                    file_path,
                    method.start_line,
                ));
            }
        }
        // Type reference
        8 => {
            if let Some(type_ref) = find_capture(captures, "type_ref") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &type_ref.text,
                    "references_type",
                    "references_type.annotation",
                    file_path,
                    type_ref.start_line,
                ));
            }
        }
        _ => {}
    }

    edges
}
