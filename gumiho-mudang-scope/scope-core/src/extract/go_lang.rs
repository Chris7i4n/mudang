//! Go edge extraction (R2 — sprint 0003, chunk 2 + chunk 7).
//!
//! Relocated from `crate::languages::go_lang`; per-language modules
//! retain only capture / metadata / docstring concerns. The extractor is
//! the only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated
//! the input slice from `HashMap<String, (String, u32)>` to
//! `&[Capture]` and adopted explicit per-pattern `pattern_id`s.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// Go edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/go/edges.scm`:
/// `0` import spec, `1` direct call, `2` selector / method call,
/// `3` struct embedding.
pub fn extract_go_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_cls = resolve_scope_id(enclosing_scope_id, file_path, "class");

    match pattern {
        // Import spec (e.g. import "fmt")
        0 => {
            if let Some(source) = find_capture(captures, "source") {
                let clean = source.text.trim_matches('"');
                edges.push(make_edge(
                    format!("{file_path}::__module__::function"),
                    clean,
                    "imports",
                    "imports.path",
                    file_path,
                    source.start_line,
                ));
            }
        }
        // Direct function call (e.g. processPayment(...))
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
        // Selector/method call (e.g. s.Handle(), fmt.Println())
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
        // Struct embedding (e.g. type Server struct { Logger })
        3 => {
            if let Some(base_type) = find_capture(captures, "base_type") {
                edges.push(make_edge(
                    from_cls.clone(),
                    &base_type.text,
                    "extends",
                    "extends.embedding",
                    file_path,
                    base_type.start_line,
                ));
            }
        }
        _ => {}
    }

    edges
}
