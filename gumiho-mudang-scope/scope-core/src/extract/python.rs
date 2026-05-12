//! Python edge extraction (R2 — sprint 0003, chunk 2 + chunk 7).
//!
//! Relocated from `crate::languages::python`; per-language modules
//! retain only capture / metadata / docstring concerns. The extractor is
//! the only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated
//! the input slice from `HashMap<String, (String, u32)>` to
//! `&[Capture]` and adopted explicit per-pattern `pattern_id`s.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// Python edge extraction by pattern index.
///
/// Pattern indices map to the order of patterns in
/// `queries/python/edges.scm`:
/// `0` import statement, `1` from-import statement, `2` direct call,
/// `3` attribute / method call, `4` class inheritance.
pub fn extract_py_edge(
    pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_fn = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_cls = resolve_scope_id(enclosing_scope_id, file_path, "class");

    match pattern {
        // import statement (e.g. `import os`) — always module-level
        0 => {
            if let Some(imported_name) = find_capture(captures, "imported_name") {
                edges.push(make_edge(
                    format!("{file_path}::__module__::function"),
                    &imported_name.text,
                    "imports",
                    "imports.module",
                    file_path,
                    imported_name.start_line,
                ));
            }
        }
        // from-import statement (e.g. `from os.path import join`)
        1 => {
            if let (Some(imported_name), Some(source_mod)) = (
                find_capture(captures, "imported_name"),
                find_capture(captures, "source"),
            ) {
                edges.push(make_edge(
                    format!("{file_path}::__module__::function"),
                    format!("{}::{}", source_mod.text, imported_name.text),
                    "imports",
                    "imports.from",
                    file_path,
                    imported_name.start_line,
                ));
            }
        }
        // Direct function call (e.g. `foo()`)
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
        // Attribute/method call (e.g. `self.foo()`, `obj.bar()`)
        3 => {
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
        // Class inheritance (e.g. `class Foo(Bar):`)
        4 => {
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
        _ => {}
    }

    edges
}
