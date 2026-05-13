//! C# edge extraction (R2).
//!
//! Relocated from `crate::languages::csharp`; per-language modules
//! retain only capture / metadata / docstring concerns. The extractor is
//! the only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated
//! the input slice from `HashMap<String, (String, u32)>` to
//! `&[Capture]` and adopted explicit per-pattern `pattern_id`s.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// C# edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/csharp/edges.scm`:
/// `0` using (identifier), `1` using (qualified), `2` member call,
/// `3` direct call, `4` new expression, `5` `this.Method()` call,
/// `6` base list (identifier), `7` base list (qualified),
/// `8` `base.Method()` call, `9` switch case member-access variant.
pub fn extract_cs_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_cls = resolve_scope_id(enclosing_scope_id, file_path, "class");
    let module_fn = || format!("{file_path}::__module__::function");

    match pattern {
        // Using directive — identifier or qualified-name — always module-level.
        0 | 1 => {
            if let Some(imported_name) = find_capture(captures, "imported_name") {
                let pattern_id = if pattern == 0 {
                    "imports.identifier"
                } else {
                    "imports.qualified"
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
        // Member access call (e.g. _logger.Info(...))
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
        // Direct call (e.g. DoSomething(...))
        3 => {
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
        // Object creation (new ...)
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
        // this.Method() / base.Method() — captures method name only.
        5 | 8 => {
            if let Some(method) = find_capture(captures, "method") {
                let pattern_id = if pattern == 5 {
                    "calls.method.this"
                } else {
                    "calls.method.base"
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
        // Base list (identifier / qualified-name) — implements / extends without disambiguation.
        6 | 7 => {
            if let Some(base_type) = find_capture(captures, "base_type") {
                let pattern_id = if pattern == 6 {
                    "implements.base_list"
                } else {
                    "implements.base_list.qualified"
                };
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_type.text,
                    "implements",
                    pattern_id,
                    file_path,
                    base_type.start_line,
                ));
            }
        }
        // Switch case with member access variant ref (e.g. case PaymentStatus.Pending:)
        9 => {
            if let Some(variant_ref) = find_capture(captures, "variant_ref") {
                edges.push(make_edge(
                    from_fn.clone(),
                    &variant_ref.text,
                    "references",
                    "references.switch.member",
                    file_path,
                    variant_ref.start_line,
                ));
            }
        }
        _ => {}
    }

    edges
}
