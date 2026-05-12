/// tree-sitter parsing and symbol/edge extraction.
///
/// Per R7 (A.4): a single `LanguageId` enum drives every per-language
/// decision via exhaustive inherent methods. Each registered language
/// holds its compiled queries in a `LanguageEntry`; the indexer routes
/// files through `dispatch::dispatch_extension` (compile-time table) and
/// dispatches behaviour through `LanguageId::*` calls.
use anyhow::{Context, Result};
use std::path::Path;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

use crate::extract::Capture;
use crate::languages::dispatch;
use crate::languages::id::LanguageId;
use crate::edge::RawEdge;
use crate::types::Symbol;

/// A registered language with its compiled queries.
struct LanguageEntry {
    lang: LanguageId,
    /// Compiled query for extracting symbol definitions.
    symbol_query: Query,
    /// Compiled query for extracting edges (calls, imports, etc.).
    edge_query: Query,
}

/// The code parser that uses tree-sitter to extract symbols and edges.
pub struct CodeParser {
    parser: Parser,
    entries: Vec<LanguageEntry>,
}

impl CodeParser {
    /// Create a new parser with every registered language pre-compiled.
    ///
    /// The registry is the const-fn list from
    /// [`crate::languages::dispatch::REGISTERED`]; no separate registration
    /// step exists.
    pub fn new() -> Result<Self> {
        let parser = Parser::new();
        let mut entries = Vec::with_capacity(dispatch::REGISTERED.len());

        for &lang in dispatch::REGISTERED {
            let ts_lang = lang.ts_language();
            let symbol_query = Query::new(&ts_lang, lang.symbol_query_source())
                .with_context(|| format!("Failed to compile {lang} symbol query"))?;
            let edge_query = Query::new(&ts_lang, lang.edge_query_source())
                .with_context(|| format!("Failed to compile {lang} edge query"))?;

            entries.push(LanguageEntry {
                lang,
                symbol_query,
                edge_query,
            });
        }

        Ok(Self { parser, entries })
    }

    fn find_entry(&self, lang: LanguageId) -> Option<&LanguageEntry> {
        self.entries.iter().find(|e| e.lang == lang)
    }

    /// Detect the language of a file based on its extension.
    ///
    /// Delegates to the compile-time dispatch table in `dispatch.rs`;
    /// there is no separate match block to keep in sync.
    pub fn detect_language(path: &Path) -> Result<LanguageId> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension: {}", path.display()))?;

        dispatch::dispatch_extension(ext)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: .{ext}"))
    }

    /// Check if a file extension is supported for parsing (has a loaded grammar).
    pub fn is_supported(&self, path: &Path) -> bool {
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            return false;
        };
        dispatch::dispatch_extension(ext).is_some()
    }

    /// Extract symbol definitions from a source file.
    pub fn extract_symbols(
        &mut self,
        file_path: &str,
        source: &str,
        lang: LanguageId,
    ) -> Result<Vec<Symbol>> {
        let entry = self
            .find_entry(lang)
            .ok_or_else(|| anyhow::anyhow!("Language {lang} not loaded"))?;

        let ts_lang = entry.lang.ts_language();

        self.parser
            .set_language(&ts_lang)
            .context("Failed to set parser language")?;

        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Parse failed for {file_path}"))?;

        let mut cursor = QueryCursor::new();

        // Re-borrow entry for iteration after mutable use of self.parser above.
        let entry = self
            .find_entry(lang)
            .ok_or_else(|| anyhow::anyhow!("Language {lang} not loaded"))?;

        let mut matches = cursor.matches(&entry.symbol_query, tree.root_node(), source.as_bytes());

        let mut symbols = Vec::new();
        let capture_names = entry.symbol_query.capture_names();

        while let Some(m) = matches.next() {
            let mut name_text: Option<String> = None;
            let mut def_node = None;
            let mut _params_text: Option<String> = None;
            let mut _return_type_text: Option<String> = None;

            for capture in m.captures {
                let capture_name = &capture_names[capture.index as usize];
                let text = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .unwrap_or_default();

                match &**capture_name {
                    "name" => name_text = Some(text.to_string()),
                    "definition" => def_node = Some(capture.node),
                    "params" => _params_text = Some(text.to_string()),
                    "return_type" => _return_type_text = Some(text.to_string()),
                    _ => {}
                }
            }

            let Some(mut name) = name_text else { continue };
            let Some(def) = def_node else { continue };

            let kind = lang.symbol_kind_for_node(def.kind()).to_string();
            if lang == LanguageId::Ruby && matches!(kind.as_str(), "class" | "interface") {
                name = qualify_ruby_decl_name(&def, &name, source);
            }
            let line = def.start_position().row as u32 + 1;
            let id = format!("{file_path}::{name}::{kind}::{line}");

            // Extract metadata using language-specific logic. Phase B
            // passes a NoopWorkspaceContext (R4); R2/R3 (sprint 0003)
            // thread a real context.
            let ctx = crate::workspace_context::NoopWorkspaceContext::default();
            let metadata = lang.extract_metadata(&def, source, &kind, &ctx)?;

            let signature = extract_signature(&def, source);
            let docstring = lang.extract_docstring(&def, source);

            // Determine parent_id for methods inside classes
            let parent_id = if kind == "method" || kind == "property" || kind == "variant" {
                find_parent_class(&def, source, file_path, lang)
            } else {
                None
            };

            symbols.push(Symbol {
                id,
                name,
                kind,
                file_path: file_path.to_string(),
                line_start: def.start_position().row as u32 + 1,
                line_end: def.end_position().row as u32 + 1,
                signature,
                docstring,
                parent_id,
                language: lang.as_str().to_string(),
                metadata,
            });
        }

        if lang == LanguageId::Rust {
            associate_rust_impl_methods(&mut symbols, &tree, source, file_path);
        }

        Ok(symbols)
    }

    /// Extract edges (relationships) from a source file.
    pub fn extract_edges(
        &mut self,
        file_path: &str,
        source: &str,
        lang: LanguageId,
    ) -> Result<Vec<RawEdge>> {
        let entry = self
            .find_entry(lang)
            .ok_or_else(|| anyhow::anyhow!("Language {lang} not loaded"))?;

        let ts_lang = entry.lang.ts_language();

        self.parser
            .set_language(&ts_lang)
            .context("Failed to set parser language")?;

        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Parse failed for {file_path}"))?;

        let mut cursor = QueryCursor::new();

        let entry = self
            .find_entry(lang)
            .ok_or_else(|| anyhow::anyhow!("Language {lang} not loaded"))?;

        let mut matches = cursor.matches(&entry.edge_query, tree.root_node(), source.as_bytes());

        let mut edges = Vec::new();
        let capture_names = entry.edge_query.capture_names();

        while let Some(m) = matches.next() {
            let pattern = m.pattern_index;
            let mut typed_captures: Vec<Capture> = Vec::with_capacity(m.captures.len());
            // Pick the smallest captured node as the scope anchor. A whole-class
            // capture like `(class ...) @extends.node` would otherwise become the
            // anchor and `find_enclosing_scope` would walk past the class itself
            // — attributing inheritance to the outer namespace.
            let mut representative_node: Option<tree_sitter::Node> = None;

            for capture in m.captures {
                let capture_name = capture_names[capture.index as usize].to_string();
                let text = capture
                    .node
                    .utf8_text(source.as_bytes())
                    .unwrap_or_default()
                    .to_string();
                let start = capture.node.start_position();
                let end = capture.node.end_position();
                let node_len = capture.node.end_byte() - capture.node.start_byte();
                let replace = match representative_node {
                    None => true,
                    Some(existing) => node_len < (existing.end_byte() - existing.start_byte()),
                };
                if replace {
                    representative_node = Some(capture.node);
                }
                typed_captures.push(Capture {
                    name: capture_name,
                    node_kind: capture.node.kind().to_string(),
                    text,
                    start_byte: capture.node.start_byte() as u32,
                    end_byte: capture.node.end_byte() as u32,
                    start_line: start.row as u32 + 1,
                    end_line: end.row as u32 + 1,
                    start_column: start.column as u32,
                    end_column: end.column as u32,
                });
            }

            let enclosing_scope_id = representative_node
                .as_ref()
                .and_then(|n| find_enclosing_scope(n, source, file_path, lang));

            let extracted = crate::extract::extract_edges_for_match(
                lang,
                pattern,
                &typed_captures,
                file_path,
                enclosing_scope_id.as_deref(),
            );
            edges.extend(extracted);
        }

        Ok(edges)
    }

    /// Collect every skipped region for this file.
    ///
    /// Concatenates:
    /// 1. Plugin-driven skips ([`LanguageId::plugin_skipped_ranges`]).
    /// 2. Tree-sitter parser-recovery skips
    ///    ([`crate::extract::scan_tree_sitter_errors`]).
    ///
    /// Both streams are forwarded verbatim per Charter §3 invariant 5; the
    /// only post-processing is a `sort_by_key(start_line, end_line)` so
    /// consumers downstream of `file_hashes.skipped_ranges` see source order.
    pub fn collect_skipped_ranges(
        &mut self,
        file_path: &str,
        source: &str,
        lang: LanguageId,
    ) -> Result<Vec<crate::extract::SkippedRange>> {
        let entry = self
            .find_entry(lang)
            .ok_or_else(|| anyhow::anyhow!("Language {lang} not loaded"))?;

        let ts_lang = entry.lang.ts_language();

        self.parser
            .set_language(&ts_lang)
            .context("Failed to set parser language")?;

        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Parse failed for {file_path}"))?;

        let root = tree.root_node();
        let mut ranges = lang.plugin_skipped_ranges(&root, source);
        ranges.extend(crate::extract::scan_tree_sitter_errors(&root, source));
        ranges.sort_by_key(|r| (r.start_line, r.end_line));
        Ok(ranges)
    }

    /// Extract Rust trait implementation edges (`impl Trait for Type`).
    ///
    /// Must be called after `extract_symbols` so that the symbols list is available
    /// to resolve the correct `from_id` for the target type.
    pub fn extract_rust_impl_trait_edges(
        &mut self,
        file_path: &str,
        source: &str,
        symbols: &[Symbol],
    ) -> Result<Vec<RawEdge>> {
        let entry = self
            .find_entry(LanguageId::Rust)
            .ok_or_else(|| anyhow::anyhow!("Rust language not loaded"))?;

        let ts_lang = entry.lang.ts_language();

        self.parser
            .set_language(&ts_lang)
            .context("Failed to set parser language")?;

        let tree = self
            .parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Parse failed for {file_path}"))?;

        Ok(crate::extract::rust_lang::extract_rust_trait_impl_edges(
            symbols, &tree, source, file_path,
        ))
    }
}

/// Extract the signature — first line of the definition up to `{` or end of the line.
///
/// For enum variants, the full text is used (including struct body like `{ field: Type }`)
/// so that data shapes are preserved in the signature.
fn extract_signature(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let start = node.start_byte();
    let end = node.end_byte();
    let text = &source[start..end];

    // Enum variants: preserve the full data shape (struct fields, tuple types)
    // e.g. "Success { tx_id: String }" or "Error(String)" or "Pending"
    if node.kind() == "enum_variant" {
        // Take up to the first newline, preserving struct bodies
        let sig = if let Some(nl_pos) = text.find('\n') {
            // For multi-line struct variants, collapse to a single line
            let full = text.trim();
            if full.contains('{') && full.contains('}') {
                // Single-line or collapsible struct variant
                let collapsed: String =
                    full.lines().map(|l| l.trim()).collect::<Vec<_>>().join(" ");
                return if collapsed.is_empty() {
                    None
                } else {
                    // Strip trailing comma from variant
                    Some(collapsed.trim_end_matches(',').trim().to_string())
                };
            }
            text[..nl_pos].trim()
        } else {
            text.trim()
        };
        // Strip trailing comma from variant
        let sig = sig.trim_end_matches(',').trim();
        return if sig.is_empty() {
            None
        } else {
            Some(sig.to_string())
        };
    }

    // Take up to the first `{` or newline, whichever comes first
    let sig = if let Some(brace_pos) = text.find('{') {
        text[..brace_pos].trim()
    } else if let Some(nl_pos) = text.find('\n') {
        text[..nl_pos].trim()
    } else {
        text.trim()
    };

    if sig.is_empty() {
        None
    } else {
        Some(sig.to_string())
    }
}

/// Walk up the AST from `node` to find the nearest enclosing scope (function, method, class).
/// Returns the symbol ID of that scope, or `None` if at module level.
fn find_enclosing_scope(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    lang: LanguageId,
) -> Option<String> {
    let mut current = node.parent();

    let scope_types = lang.scope_node_types();

    while let Some(parent) = current {
        if scope_types.contains(&parent.kind()) {
            // Ruby `call` nodes are scope boundaries only for assigned proc/lambda
            // blocks. Treating every call as a scope would create synthetic
            // from_ids such as `::new::function` for ordinary member calls.
            if lang == LanguageId::Ruby && parent.kind() == "call" {
                if let Some(scope_id) = ruby_assigned_call_scope(&parent, source, file_path, lang) {
                    return Some(scope_id);
                }
                current = parent.parent();
                continue;
            }

            if lang == LanguageId::Ruby && parent.kind() == "lambda" {
                if let Some(scope_id) = ruby_assigned_lambda_scope(&parent, source, file_path) {
                    return Some(scope_id);
                }
                current = parent.parent();
                continue;
            }

            // For arrow functions / function expressions assigned to variables,
            // walk up to the variable_declarator to get a meaningful name.
            if parent.kind() == "arrow_function" || parent.kind() == "function_expression" {
                if let Some(grandparent) = parent.parent() {
                    if grandparent.kind() == "variable_declarator" {
                        if let Some(name_node) = grandparent.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = grandparent.start_position().row as u32 + 1;
                                return Some(format!("{file_path}::{name}::function::{line}"));
                            }
                        }
                    }
                }
                current = parent.parent();
                continue;
            }

            // Named scope — get its name and build the ID
            if let Some(name_node) = parent.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    let mut kind = lang.symbol_kind_for_node(parent.kind());

                    // For Rust: function_item inside an impl_item is a "method"
                    if kind == "function"
                        && lang == LanguageId::Rust
                        && parent.kind() == "function_item"
                    {
                        if let Some(grandparent) = parent.parent() {
                            if grandparent.kind() == "declaration_list" {
                                if let Some(great_grandparent) = grandparent.parent() {
                                    if great_grandparent.kind() == "impl_item" {
                                        kind = "method";
                                    }
                                }
                            }
                        }
                    }

                    let name = if lang == LanguageId::Ruby && matches!(kind, "class" | "interface")
                    {
                        qualify_ruby_decl_name(&parent, name, source)
                    } else {
                        name.to_string()
                    };
                    let line = parent.start_position().row as u32 + 1;
                    return Some(format!("{file_path}::{name}::{kind}::{line}"));
                }
            }
        }
        current = parent.parent();
    }

    None
}

fn ruby_assigned_lambda_scope(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
) -> Option<String> {
    let assignment = node.parent()?;
    if assignment.kind() != "assignment" {
        return None;
    }

    let name_node = assignment.child_by_field_name("left")?;
    if name_node.kind() != "identifier" {
        return None;
    }

    let name = name_node.utf8_text(source.as_bytes()).ok()?;
    let line = assignment.start_position().row as u32 + 1;
    Some(format!("{file_path}::{name}::function::{line}"))
}

fn ruby_assigned_call_scope(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    lang: LanguageId,
) -> Option<String> {
    let method = node.child_by_field_name("method")?;
    let method_name = method.utf8_text(source.as_bytes()).ok()?;
    if method_name != "proc" && method_name != "lambda" {
        return None;
    }

    let assignment = node.parent()?;
    if assignment.kind() != "assignment" {
        return None;
    }

    let name_node = assignment.child_by_field_name("left")?;
    if name_node.kind() != "identifier" {
        return None;
    }

    let name = name_node.utf8_text(source.as_bytes()).ok()?;
    let kind = lang.symbol_kind_for_node(node.kind());
    let line = assignment.start_position().row as u32 + 1;
    Some(format!("{file_path}::{name}::{kind}::{line}"))
}

/// Find the parent class for a method or property node.
fn find_parent_class(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    lang: LanguageId,
) -> Option<String> {
    let class_body_nodes = lang.class_body_node_types();
    let class_decl_nodes = lang.class_decl_node_types();

    let mut current = node.parent();
    while let Some(parent) = current {
        if class_body_nodes.contains(&parent.kind()) {
            if let Some(class_node) = parent.parent() {
                if class_decl_nodes.contains(&class_node.kind()) {
                    if let Some(name_node) = class_node.child_by_field_name("name") {
                        let class_name = name_node.utf8_text(source.as_bytes()).ok()?;
                        let kind = lang.symbol_kind_for_node(class_node.kind());
                        let class_name = if lang == LanguageId::Ruby {
                            qualify_ruby_decl_name(&class_node, class_name, source)
                        } else {
                            class_name.to_string()
                        };
                        let class_line = class_node.start_position().row as u32 + 1;
                        return Some(format!("{file_path}::{class_name}::{kind}::{class_line}"));
                    }
                }
            }
        }
        current = parent.parent();
    }
    None
}

fn qualify_ruby_decl_name(node: &tree_sitter::Node, name: &str, source: &str) -> String {
    let clean_name = name.trim().trim_start_matches("::");
    if clean_name.is_empty() || clean_name.contains("::") {
        return clean_name.to_string();
    }

    let mut namespaces = Vec::new();
    let mut current = node.parent();

    while let Some(parent) = current {
        if matches!(parent.kind(), "class" | "module") {
            if let Some(name_node) = parent.child_by_field_name("name") {
                if let Ok(parent_name) = name_node.utf8_text(source.as_bytes()) {
                    let parent_name = parent_name.trim().trim_start_matches("::");
                    if !parent_name.is_empty() {
                        namespaces.push(parent_name.to_string());
                    }
                }
            }
        }
        current = parent.parent();
    }

    if namespaces.is_empty() {
        clean_name.to_string()
    } else {
        namespaces.reverse();
        namespaces.push(clean_name.to_string());
        namespaces.join("::")
    }
}

/// Associate Rust methods inside `impl` blocks with their target struct/enum.
///
/// Rust defines methods in separate `impl Type { ... }` blocks, not inside the
/// struct/enum body. After extracting all symbols from a file, this function
/// walks the AST for `impl_item` nodes and sets `parent_id` on each function
/// inside the impl's `declaration_list` to point to the target struct/enum
/// symbol in the same file.
///
/// Handles both `impl Type { ... }` and `impl Trait for Type { ... }`.
/// Generic type parameters (e.g., `impl<T> Foo<T>`) are handled by extracting
/// only the base type name. Methods targeting types defined in other files
/// (i.e., no matching struct/enum in the current file's symbols) are skipped.
fn associate_rust_impl_methods(
    symbols: &mut [Symbol],
    tree: &tree_sitter::Tree,
    source: &str,
    file_path: &str,
) {
    let root = tree.root_node();
    let mut tree_cursor = root.walk();

    // Collect impl block info: (target_type_name, Vec<(method_name, method_line_start)>)
    let mut impl_associations: Vec<(String, Vec<(String, u32)>)> = Vec::new();

    for child in root.children(&mut tree_cursor) {
        if child.kind() != "impl_item" {
            continue;
        }

        let target_type_name =
            match crate::extract::rust_lang::extract_impl_target_type(&child, source) {
                Some(name) => name,
                None => continue,
            };

        // Collect function_item children inside the declaration_list
        let mut methods = Vec::new();
        let mut impl_cursor = child.walk();
        for impl_child in child.children(&mut impl_cursor) {
            if impl_child.kind() == "declaration_list" {
                let mut decl_cursor = impl_child.walk();
                for decl_child in impl_child.children(&mut decl_cursor) {
                    if decl_child.kind() == "function_item" {
                        if let Some(name_node) = decl_child.child_by_field_name("name") {
                            if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                                let line = decl_child.start_position().row as u32 + 1;
                                methods.push((name.to_string(), line));
                            }
                        }
                    }
                }
            }
        }

        if !methods.is_empty() {
            impl_associations.push((target_type_name, methods));
        }
    }

    // Now match extracted symbols to their target types
    for (target_type_name, methods) in &impl_associations {
        // Find the target struct/enum symbol in the same file
        let target_id = symbols
            .iter()
            .find(|s| {
                s.file_path == file_path
                    && s.name == *target_type_name
                    && (s.kind == "struct" || s.kind == "enum" || s.kind == "interface")
            })
            .map(|s| s.id.clone());

        let Some(target_id) = target_id else {
            // Target type not in this file — skip
            continue;
        };

        // Set parent_id on each method symbol
        for (method_name, method_line) in methods {
            if let Some(sym) = symbols.iter_mut().find(|s| {
                s.file_path == file_path
                    && s.name == *method_name
                    && s.line_start == *method_line
                    && s.kind == "function"
                    && s.parent_id.is_none()
            }) {
                sym.parent_id = Some(target_id.clone());
                sym.kind = "method".to_string();
                // Update ID to reflect the new kind
                sym.id = format!(
                    "{}::{}::method::{}",
                    sym.file_path, sym.name, sym.line_start
                );
            }
        }
    }
}
