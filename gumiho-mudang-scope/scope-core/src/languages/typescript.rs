/// TypeScript-specific metadata extraction and language plugin.
///
/// Extracts access modifiers, async, static, return type, and parameters
/// from TypeScript AST nodes. TypeScript defaults to public access.
use anyhow::Result;
use serde::Serialize;

use crate::extract::MetadataEntry;

/// Structured metadata for a TypeScript symbol.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SymbolMetadata {
    /// Access modifier: public, private, or protected. TypeScript defaults to public.
    pub access: String,
    /// Whether the symbol is async.
    pub is_async: bool,
    /// Whether the symbol is static.
    pub is_static: bool,
    /// Whether the symbol is abstract.
    pub is_abstract: bool,
    /// Whether the symbol is readonly.
    pub is_readonly: bool,
    /// Return type annotation, if present.
    pub return_type: Option<String>,
    /// Parameter list with names, types, and optionality.
    pub parameters: Vec<ParameterInfo>,
    /// Decorators applied to this symbol — reserved key per
    /// `LANGUAGE-PLAYBOOK.md` § Step 5. Empty array means "looked, found
    /// none". TypeScript's AST exposes `decorator` nodes (legacy
    /// `@Decorator(...)` form on classes and methods), so the key is
    /// always present.
    ///
    /// `template_calls` is **omitted** at this layer: the project parses
    /// `.ts` and `.tsx` files with `tree_sitter_typescript::LANGUAGE_TYPESCRIPT`
    /// which does not expose JSX AST nodes. Per the playbook
    /// (§ Step 5), absent-vs-empty distinction is meaningful, so
    /// omitting accurately records "language plugin did not implement
    /// this surface" rather than "looked, found none". When TSX parsing
    /// lands in a future sprint, the field gains a struct entry and the
    /// JSON shape opts in.
    pub decorators: Vec<MetadataEntry>,
}

/// Information about a single function/method parameter.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterInfo {
    /// Parameter name.
    pub name: String,
    /// Type annotation, if present.
    #[serde(rename = "type")]
    pub type_annotation: Option<String>,
    /// Whether the parameter is optional.
    pub optional: bool,
}

/// Extract metadata from a TypeScript AST node.
///
/// Returns a JSON string suitable for the `metadata` column.
pub fn extract_metadata(node: &tree_sitter::Node, source: &str, kind: &str) -> Result<String> {
    let mut meta = SymbolMetadata {
        access: "public".to_string(),
        ..Default::default()
    };

    // Walk direct children to find modifiers + decorators.
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        match child.kind() {
            "async" => meta.is_async = true,
            "static" => meta.is_static = true,
            "abstract" => meta.is_abstract = true,
            "readonly" => meta.is_readonly = true,
            "accessibility_modifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    meta.access = text.to_string();
                }
            }
            "decorator" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    // `@Component`              → name="Component", args_text=None
                    // `@Component({selector:…})` → name="Component",
                    //                              args_text=Some(`({selector:…})`)
                    let stripped = text.trim_start_matches('@').trim();
                    let (dec_name, args_text) = match stripped.find('(') {
                        Some(idx) => (
                            stripped[..idx].trim().to_string(),
                            Some(stripped[idx..].trim().to_string()),
                        ),
                        None => (stripped.to_string(), None),
                    };
                    if !dec_name.is_empty() {
                        meta.decorators.push(MetadataEntry {
                            name: dec_name,
                            args_text,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // tree-sitter-typescript places `decorator` nodes either as direct
    // children of the wrapped declaration (some grammar versions / some
    // shapes) OR as **adjacent preceding siblings** inside the enclosing
    // block (the more common shape for class members:
    // `class C { @Get(...) a(){} }` parses the decorator as a sibling of
    // `method_definition` inside `class_body`). The direct-child loop
    // above handles the first case; the prev-sibling walk below handles
    // the second.
    //
    // Ownership rule (mirrors the chunk-3b codex-fix for Rust
    // `attribute_item`): only **contiguous** preceding `decorator`
    // siblings attach to this node. Comment siblings (`comment`) are
    // transparent. The walk stops at the first sibling whose kind is
    // neither `decorator` nor a comment kind — guaranteeing no cross-
    // method bleed (`class C { @A a(){}; @B b(){}; c(){} }` gives
    // a → [A], b → [B], c → []). Decorators are collected in walk order
    // (newest-first while moving leftward) and reversed at the end to
    // match source order.
    let mut prev_decorators: Vec<MetadataEntry> = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        match s.kind() {
            "decorator" => {
                if let Ok(text) = s.utf8_text(source.as_bytes()) {
                    let stripped = text.trim_start_matches('@').trim();
                    let (dec_name, args_text) = match stripped.find('(') {
                        Some(idx) => (
                            stripped[..idx].trim().to_string(),
                            Some(stripped[idx..].trim().to_string()),
                        ),
                        None => (stripped.to_string(), None),
                    };
                    if !dec_name.is_empty() {
                        prev_decorators.push(MetadataEntry {
                            name: dec_name,
                            args_text,
                        });
                    }
                }
                sibling = s.prev_sibling();
            }
            "comment" => {
                // Comments are transparent — keep walking.
                sibling = s.prev_sibling();
            }
            _ => break,
        }
    }
    prev_decorators.reverse();
    meta.decorators.append(&mut prev_decorators);

    // Extract return type from type_annotation field
    if let Some(return_type_node) = node.child_by_field_name("return_type") {
        if let Ok(text) = return_type_node.utf8_text(source.as_bytes()) {
            // Strip the leading `: ` from type annotations
            let clean = text.trim_start_matches(':').trim();
            meta.return_type = Some(clean.to_string());
        }
    }

    // Extract parameters
    if kind == "function" || kind == "method" {
        if let Some(params_node) = node.child_by_field_name("parameters") {
            meta.parameters = extract_parameters(&params_node, source);
        }
    }

    let json = serde_json::to_string(&meta)?;
    Ok(json)
}

/// Extract parameter info from a formal_parameters node.
fn extract_parameters(params_node: &tree_sitter::Node, source: &str) -> Vec<ParameterInfo> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
            let optional = child.kind() == "optional_parameter";

            let name = child
                .child_by_field_name("pattern")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or_default()
                .to_string();

            let type_annotation = child
                .child_by_field_name("type")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|t| t.trim_start_matches(':').trim().to_string());

            if !name.is_empty() {
                params.push(ParameterInfo {
                    name,
                    type_annotation,
                    optional,
                });
            }
        }
    }

    params
}

#[cfg(test)]
mod ts_decorator_tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_ts(source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn find_first<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(hit) = find_first(child, kind) {
                return Some(hit);
            }
        }
        None
    }

    fn extract_for_node(source: &str, node: tree_sitter::Node, kind: &str) -> serde_json::Value {
        let json = extract_metadata(&node, source, kind).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn sibling_decorators_do_not_bleed_across_methods() {
        // Regression test for the codex-review P1 #2 fix. Each method must
        // see only the decorators belonging to it; the undecorated `c()` must
        // see none.
        let source = r#"class C {
  @Get("/a")
  a() {}

  @Post("/b")
  b() {}

  c() {}
}"#;
        let tree = parse_ts(source);
        let class = find_first(tree.root_node(), "class_body").unwrap();

        let mut decorators_per_method: Vec<(String, Vec<String>)> = Vec::new();
        let mut cursor = class.walk();
        for child in class.children(&mut cursor) {
            if child.kind() != "method_definition" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or_default()
                .to_string();
            let meta = extract_for_node(source, child, "method");
            let names: Vec<String> = meta
                .get("decorators")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.get("name").and_then(|v| v.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            decorators_per_method.push((name, names));
        }

        // Exact ownership: a→[Get], b→[Post], c→[].
        // We cannot pin grammar-version-specific decorator placement without
        // brittleness, so we assert the BLEED-PREVENTION invariant: no method
        // sees BOTH Get and Post; the undecorated `c` sees neither.
        for (name, decs) in &decorators_per_method {
            let has_get = decs.iter().any(|d| d == "Get");
            let has_post = decs.iter().any(|d| d == "Post");
            assert!(
                !(has_get && has_post),
                "method `{name}` saw both Get and Post — decorators bled across siblings: {decs:?}"
            );
            if name == "c" {
                assert!(
                    !has_get && !has_post,
                    "undecorated `c` saw decorators: {decs:?}"
                );
            }
        }
    }

    /// Class-member decorators are preceding siblings of the method
    /// inside `class_body`, not direct children. The prev-sibling
    /// walk must capture them.
    #[test]
    fn class_member_decorator_is_captured_for_owning_method() {
        let source = r#"class C {
  @Get("/a")
  a() {}

  @Post("/b")
  b() {}

  c() {}
}"#;
        let tree = parse_ts(source);
        let class = find_first(tree.root_node(), "class_body").unwrap();

        let mut decorators_per_method: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut cursor = class.walk();
        for child in class.children(&mut cursor) {
            if child.kind() != "method_definition" {
                continue;
            }
            let name = child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .unwrap_or_default()
                .to_string();
            let meta = extract_for_node(source, child, "method");
            let names: Vec<String> = meta
                .get("decorators")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|e| e.get("name").and_then(|v| v.as_str()).map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            decorators_per_method.insert(name, names);
        }

        assert_eq!(
            decorators_per_method.get("a").map(|v| v.as_slice()),
            Some(["Get".to_string()].as_slice()),
            "method `a` should see exactly [Get]; got {:?}",
            decorators_per_method.get("a")
        );
        assert_eq!(
            decorators_per_method.get("b").map(|v| v.as_slice()),
            Some(["Post".to_string()].as_slice()),
            "method `b` should see exactly [Post]; got {:?}",
            decorators_per_method.get("b")
        );
        assert_eq!(
            decorators_per_method.get("c").map(|v| v.as_slice()),
            Some([].as_slice()),
            "undecorated method `c` should see no decorators; got {:?}",
            decorators_per_method.get("c")
        );
    }

    /// Multi-decorator + args_text source-order capture on a single method.
    #[test]
    fn multiple_decorators_on_one_method_preserve_source_order() {
        let source = r#"class C {
  @Auth
  @Get("/users")
  list() {}
}"#;
        let tree = parse_ts(source);
        let class = find_first(tree.root_node(), "class_body").unwrap();
        let method = class
            .children(&mut class.walk())
            .find(|c| c.kind() == "method_definition")
            .unwrap();

        let meta = extract_for_node(source, method, "method");
        let decorators = meta.get("decorators").and_then(|v| v.as_array()).unwrap();
        let names: Vec<&str> = decorators
            .iter()
            .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
            .collect();
        assert_eq!(names, vec!["Auth", "Get"], "source order: @Auth then @Get");

        let get_args = decorators
            .iter()
            .find(|e| e.get("name").and_then(|v| v.as_str()) == Some("Get"))
            .and_then(|e| e.get("args_text"))
            .and_then(|v| v.as_str());
        assert_eq!(get_args, Some("(\"/users\")"));
    }

    /// Comment between decorator and method must be transparent.
    #[test]
    fn comment_between_decorator_and_method_is_transparent() {
        let source = r#"class C {
  @Get("/a")
  // route handler
  a() {}
}"#;
        let tree = parse_ts(source);
        let class = find_first(tree.root_node(), "class_body").unwrap();
        let method = class
            .children(&mut class.walk())
            .find(|c| c.kind() == "method_definition")
            .unwrap();

        let meta = extract_for_node(source, method, "method");
        let names: Vec<&str> = meta
            .get("decorators")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(names, vec!["Get"]);
    }

    /// Top-level class with class decorator (the direct-child path).
    #[test]
    fn class_level_decorator_still_captured_via_direct_child() {
        let source = r#"@Component({ selector: "app-root" })
class App {}"#;
        let tree = parse_ts(source);
        let class = find_first(tree.root_node(), "class_declaration").unwrap();
        let meta = extract_for_node(source, class, "class");

        let decorators = meta.get("decorators").and_then(|v| v.as_array()).unwrap();
        let names: Vec<&str> = decorators
            .iter()
            .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(
            names.contains(&"Component"),
            "class decorator should be captured (direct-child or prev-sibling), got {names:?}"
        );
    }
}
