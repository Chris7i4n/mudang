//! Ruby edge extraction (R2 — sprint 0003, chunk 2 + chunk 7).
//!
//! Relocated from `crate::languages::ruby`; per-language modules retain
//! only capture / metadata / docstring concerns. The extractor is the
//! only `EdgeKind`-aware site (R2 target state). Chunk 7 migrated the
//! input slice from `HashMap<String, (String, u32)>` to `&[Capture]`
//! and adopted explicit per-pattern `pattern_id`s.
//!
//! Ruby dispatches by semantic capture name (e.g. `import.method`,
//! `call.receiver`) rather than by `pattern_index`; the `_pattern`
//! argument is ignored. Edges differentiate via the
//! constants/predicates below.

use crate::edge::RawEdge;
use crate::extract::{find_capture, make_edge, resolve_scope_id, Capture};

/// Ruby edge extraction by semantic capture name.
///
/// The extractor validates call names explicitly instead of relying only on
/// tree-sitter query predicates. That keeps command output stable if a broad
/// structural capture also matches a Ruby metaprogramming or receiver-call form.
pub fn extract_ruby_edge(
    _pattern: usize,
    captures: &[Capture],
    file_path: &str,
    enclosing_scope_id: Option<&str>,
) -> Vec<RawEdge> {
    let mut edges = Vec::new();

    let from_scope = resolve_scope_id(enclosing_scope_id, file_path, "function");
    let from_class = resolve_scope_id(enclosing_scope_id, file_path, "class");
    let from_module = format!("{file_path}::__module__::function");

    // require/require_relative/load/autoload — always module-level imports.
    if let (Some(method), Some(path), Some(call_node)) = (
        find_capture(captures, "import.method"),
        find_capture(captures, "import.path"),
        find_capture(captures, "import.call"),
    ) {
        if matches!(
            method.text.as_str(),
            "require" | "require_relative" | "load" | "autoload"
        ) && is_plain_ruby_call(&call_node.text, &method.text)
        {
            let pattern_id = if method.text == "autoload" {
                "imports.autoload"
            } else {
                "imports.require"
            };
            edges.push(make_edge(
                from_module,
                clean_ruby_literal(&path.text),
                "imports",
                pattern_id,
                file_path,
                path.start_line,
            ));
        }
        return edges;
    }

    // Instantiation via `.new`.
    if let (Some(class_node), Some(method)) = (
        find_capture(captures, "instantiate.class"),
        find_capture(captures, "instantiate.method"),
    ) {
        let class_name = clean_ruby_edge_name(&class_node.text);
        if method.text == "new" && !class_name.is_empty() {
            edges.push(make_edge(
                from_scope,
                class_name,
                "instantiates",
                "instantiates.new",
                file_path,
                class_node.start_line,
            ));
        }
        return edges;
    }

    // Class inheritance.
    if let Some(parent) = find_capture(captures, "extends.parent") {
        let parent_name = clean_ruby_edge_name(&parent.text);
        if !parent_name.is_empty() {
            edges.push(make_edge(
                from_class,
                parent_name,
                "extends",
                "extends.class",
                file_path,
                parent.start_line,
            ));
        }
        return edges;
    }

    // Mixins: include/prepend/extend.
    if let (Some(method), Some(module_node), Some(call_node)) = (
        find_capture(captures, "implements.method"),
        find_capture(captures, "implements.module"),
        find_capture(captures, "implements.call"),
    ) {
        if matches!(method.text.as_str(), "include" | "prepend" | "extend")
            && is_plain_ruby_call(&call_node.text, &method.text)
        {
            let pattern_id = match method.text.as_str() {
                "include" => "implements.mixin.include",
                "prepend" => "implements.mixin.prepend",
                _ => "implements.mixin.extend",
            };
            edges.push(make_edge(
                from_class,
                clean_ruby_edge_name(&module_node.text),
                "implements",
                pattern_id,
                file_path,
                module_node.start_line,
            ));
        }
        return edges;
    }

    // Constant/type references in selected expression positions.
    if let Some(type_node) = find_capture(captures, "type.name") {
        let type_name = clean_ruby_edge_name(&type_node.text);
        if !type_name.is_empty() {
            edges.push(make_edge(
                from_scope,
                type_name,
                "references_type",
                "references_type.constant",
                file_path,
                type_node.start_line,
            ));
        }
        return edges;
    }

    // Literal send/public_send/define_method/const_get.
    if let (Some(method), Some(literal_node)) = (
        find_capture(captures, "meta.method"),
        find_capture(captures, "meta.literal"),
    ) {
        let literal = clean_ruby_literal(&literal_node.text);
        if literal.is_empty() || is_dynamic_ruby_literal(&literal) {
            return edges;
        }

        match method.text.as_str() {
            "send" | "public_send" | "__send__" => {
                edges.push(make_edge(
                    from_scope,
                    literal,
                    "calls",
                    "calls.meta.send",
                    file_path,
                    method.start_line,
                ));
            }
            "define_method" => {
                edges.push(make_edge(
                    from_scope,
                    literal,
                    "references",
                    "references.meta.define_method",
                    file_path,
                    method.start_line,
                ));
            }
            "const_get" => {
                edges.push(make_edge(
                    from_scope,
                    literal,
                    "references_type",
                    "references_type.meta.const_get",
                    file_path,
                    method.start_line,
                ));
            }
            _ => {}
        }
        return edges;
    }

    // super
    if let Some(super_cap) = find_capture(captures, "ruby.super") {
        edges.push(make_edge(
            from_scope,
            "super",
            "calls",
            "calls.super",
            file_path,
            super_cap.start_line,
        ));
        return edges;
    }

    // yield
    if let Some(yield_cap) = find_capture(captures, "ruby.yield") {
        edges.push(make_edge(
            from_scope,
            "yield",
            "calls",
            "calls.yield",
            file_path,
            yield_cap.start_line,
        ));
        return edges;
    }

    // Receiver call.
    if let (Some(receiver_cap), Some(name_cap), Some(call_node)) = (
        find_capture(captures, "call.receiver"),
        find_capture(captures, "call.name"),
        find_capture(captures, "call.node"),
    ) {
        let receiver = clean_ruby_edge_name(&receiver_cap.text);
        let method = clean_ruby_edge_name(&name_cap.text);
        if !receiver.is_empty()
            && !method.is_empty()
            && method != "new"
            && !is_reserved_edge_call(&method)
            && !is_plain_ruby_call(&call_node.text, &method)
        {
            edges.push(make_edge(
                from_scope,
                format!("{receiver}.{method}"),
                "calls",
                "calls.method",
                file_path,
                receiver_cap.start_line,
            ));
        }
        return edges;
    }

    // Direct call. The query also structurally matches receiver calls, so
    // validate the full call text and skip cases handled by narrower patterns.
    if let (Some(name_cap), Some(call_node)) = (
        find_capture(captures, "call.name"),
        find_capture(captures, "call.node"),
    ) {
        let callee = clean_ruby_edge_name(&name_cap.text);
        if !callee.is_empty()
            && !is_reserved_edge_call(&callee)
            && is_plain_ruby_call(&call_node.text, &callee)
        {
            edges.push(make_edge(
                from_scope,
                callee,
                "calls",
                "calls.function",
                file_path,
                name_cap.start_line,
            ));
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
