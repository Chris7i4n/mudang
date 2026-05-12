//! Tree-sitter error scanner (R6 — sprint 0003, chunk 3a).
//!
//! Walks a parsed `tree_sitter::Tree` and collects every node where
//! `node.is_error()` or `node.is_missing()` into a flat
//! [`SkippedRange`] list.
//!
//! ## Conventions
//!
//! - `reason = "tree_sitter_error:syntax_error"` for `ERROR` nodes
//! - `reason = "tree_sitter_error:missing_node"` for `MISSING` nodes
//!
//! Both share the `tree_sitter_error:` prefix that the sprint 0008 R6
//! malformed-source harness greps to distinguish parser-recovery skips
//! from plugin-driven skips (the latter use `plugin_skip:` per
//! [`SkippedRange::reason`] convention). These two `tree_sitter_error:*`
//! subkinds plus `plugin_skip:<plugin>:<rationale>` are the **only**
//! reason families the indexer emits — `is_error()` and `is_missing()`
//! are tree-sitter's only error-node accessors, so there is no separate
//! "unrecoverable" category; a MISSING node is the closest analog and
//! is already captured by `tree_sitter_error:missing_node`.
//!
//! ## Charter alignment
//!
//! - Charter §3 invariant 5 (tree-sitter resilience): the scanner emits
//!   one entry per error node — no merging, no reordering, no filtering.
//!   That extends to nested error nodes: a `MISSING` node beneath an
//!   `ERROR` parent produces both entries, not just the parent. The
//!   indexer (via [`crate::parser::CodeParser::collect_skipped_ranges`])
//!   owns source-order presentation; the scanner stays a raw stream.
//! - Charter §3 invariant 1 (read-only against AST): the scanner makes no
//!   filesystem call and never mutates the tree.

use super::SkippedRange;

/// Walk the parsed tree and collect every ERROR / MISSING node as a
/// [`SkippedRange`]. Returns an empty vec if the tree is clean.
///
/// Lines are 1-based to match the `SkippedRange` contract used by
/// `file_hashes.skipped_ranges` (consistent with `Symbol.line` and
/// `Edge.line` across the codebase).
///
/// **Per Charter §3 invariant 5**, the scanner is a raw stream — every
/// `ERROR` / `MISSING` descendant is emitted, including those nested
/// beneath other error nodes. Presentation-order sorting happens at the
/// collection layer ([`crate::parser::CodeParser::collect_skipped_ranges`]).
pub fn scan_tree_sitter_errors(root: &tree_sitter::Node, _source: &str) -> Vec<SkippedRange> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    let mut stack: Vec<tree_sitter::Node> = vec![*root];

    while let Some(node) = stack.pop() {
        if node.is_error() {
            out.push(node_to_range(&node, "tree_sitter_error:syntax_error"));
        } else if node.is_missing() {
            out.push(node_to_range(&node, "tree_sitter_error:missing_node"));
        }

        // Always recurse: a MISSING descendant under an ERROR parent must
        // surface independently. Filtering would violate the no-merge / no-
        // filter half of Charter §3 invariant 5.
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }

    out
}

fn node_to_range(node: &tree_sitter::Node, reason: &str) -> SkippedRange {
    let start_line = node.start_position().row as u32 + 1;
    let end_line = node.end_position().row as u32 + 1;
    SkippedRange {
        start_line,
        end_line,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse(lang: tree_sitter::Language, source: &str) -> tree_sitter::Tree {
        let mut parser = Parser::new();
        parser.set_language(&lang).expect("set language");
        parser.parse(source, None).expect("parse")
    }

    #[test]
    fn clean_source_emits_no_ranges() {
        let tree = parse(
            tree_sitter_python::LANGUAGE.into(),
            "def hello():\n    return 42\n",
        );
        let ranges = scan_tree_sitter_errors(&tree.root_node(), "");
        assert!(ranges.is_empty(), "clean source emitted: {ranges:?}");
    }

    #[test]
    fn syntax_error_produces_tree_sitter_error_range() {
        // Unbalanced bracket — guaranteed ERROR node from tree-sitter-python.
        let source = "def broken(:\n    return\n";
        let tree = parse(tree_sitter_python::LANGUAGE.into(), source);
        let ranges = scan_tree_sitter_errors(&tree.root_node(), source);

        assert!(
            !ranges.is_empty(),
            "expected at least one skipped range for malformed source"
        );
        assert!(
            ranges
                .iter()
                .all(|r| r.reason.starts_with("tree_sitter_error:")),
            "every range must carry the tree_sitter_error: prefix; got {ranges:?}"
        );
        assert!(
            ranges.iter().all(|r| r.start_line >= 1 && r.end_line >= 1),
            "lines must be 1-based; got {ranges:?}"
        );
    }

    #[test]
    fn nested_missing_under_error_is_not_filtered() {
        // Pattern designed to land a MISSING descendant beneath an ERROR
        // parent: an unclosed function header trailed by a stray colon.
        // The exact AST shape varies per grammar version, but the test
        // contract is invariant: if the scanner sees both an ERROR and a
        // MISSING descendant, BOTH appear in the output. Per Charter §3
        // invariant 5, the scanner never filters one out as "covered by
        // the parent".
        let source = "def broken(:\n    pass\n";
        let tree = parse(tree_sitter_python::LANGUAGE.into(), source);
        let ranges = scan_tree_sitter_errors(&tree.root_node(), source);

        // Walk the tree ourselves to count expected entries — the test
        // asserts the scanner returns AT LEAST that many. This pins the
        // no-filter contract without depending on grammar-version-specific
        // node counts.
        let mut expected = 0usize;
        let mut cursor = tree.root_node().walk();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.is_error() || node.is_missing() {
                expected += 1;
            }
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }
        assert_eq!(
            ranges.len(),
            expected,
            "scanner must emit one range per ERROR / MISSING node \
             (Charter §3 inv 5 — no filter); got {ranges:?}, expected {expected}"
        );
    }
}
