//! Language identifier — single type of truth for every per-language decision.
//!
//! Per R7 (A.4): no trait, no `*Plugin` unit structs. Behaviour lives in
//! inherent methods on `LanguageId` with exhaustive `match self` arms; the
//! compiler refuses to build when a variant is added without touching every
//! decision site. This is the make-invalid-state-unrepresentable foundation
//! the rest of Phase B builds on.
//!
//! `as_str()` returns the historical DB slug verbatim (R7 § B.3 — zero schema
//! impact on `symbols.language`). The regression test in
//! `tests/language_id_db_slug.rs` pins this contract.
//!
//! # Negative trait shape (R11 + R12 — mechanically enforced)
//!
//! The post-A.4 plugin surface (this `impl LanguageId` block + the per-language
//! modules under `scope-core/src/languages/` + the per-language extractors
//! under `scope-core/src/extract/`) carries an exhaustive **negative**
//! shape: no method or free function may be named with any of the following
//! prefixes:
//!
//! - `infer_*` — would imply type-system inference (forbidden by
//!   LANGUAGE-PLAYBOOK.md A1 + CHARTER.md § 5 hard limit "No live type
//!   inference").
//! - `evaluate_*` — would imply expression / constant evaluation.
//! - `solve_*` — would imply constraint solving (A2).
//! - `narrow_*` — would imply type narrowing.
//! - `resolve_overload_*` — would imply method-dispatch / overload
//!   resolution (A3 + B2 + CHARTER.md § 5 "No reflection / dynamic
//!   dispatch resolution").
//! - `expand_*` — would imply macro / template expansion (R11 + C1 +
//!   CHARTER.md § 5 "Runtime macro expansion").
//!
//! `scripts/audit_trait_shape.sh` (gate `ci-trait-shape`) greps the scanned
//! paths for `fn <prefix>...` definitions and fails the build on a match.
//! The name itself is the contract — there is no allowlist tag. A function
//! whose work does not actually do inference / evaluation / narrowing /
//! overload resolution / expansion must be named accordingly (e.g.
//! `symbol_kind_for_node` not `infer_symbol_kind`; `visibility_for_node`
//! not `infer_visibility`; `access_from_name` not `infer_access`).
//!
//! Companion gates: `scripts/audit_immutable.sh` (R9 — no `&mut` on source
//! types), `scripts/audit_no_spawn.sh` (R12 — no `Command::new` in audited
//! paths), `scripts/audit_no_network.sh` (R12 — no network symbols).

use anyhow::Result;
use tree_sitter::Language;

use crate::workspace_context::LanguageWorkspaceContext;

/// Languages indexable by `scope`.
///
/// Every method on this type is exhaustive over the variants. To add a
/// language, add the variant and the compiler will list every site that
/// must be updated. To remove one, delete the variant and follow the
/// errors. There is no other registration step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    /// TypeScript (.ts, .tsx)
    TypeScript,
    /// C# (.cs)
    CSharp,
    /// Python (.py)
    Python,
    /// Go (.go)
    Go,
    /// Java (.java)
    Java,
    /// Rust (.rs)
    Rust,
    /// Ruby (.rb)
    Ruby,
}

impl LanguageId {
    /// DB slug — preserved verbatim across the R7 rename per B.3.
    ///
    /// Persisted to `symbols.language`. Changing the return value of any
    /// arm is a schema break; the regression test pins each variant to its
    /// historical slug.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::CSharp => "csharp",
            Self::Python => "python",
            Self::Go => "go",
            Self::Java => "java",
            Self::Rust => "rust",
            Self::Ruby => "ruby",
        }
    }

    /// Reverse of `as_str` — used by call sites that hold a DB slug (e.g.
    /// `symbols.language` rows in scope-search) and need typed dispatch.
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "typescript" => Some(Self::TypeScript),
            "csharp" => Some(Self::CSharp),
            "python" => Some(Self::Python),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "rust" => Some(Self::Rust),
            "ruby" => Some(Self::Ruby),
            _ => None,
        }
    }

    /// File extensions this language owns. Const-callable so the dispatch
    /// table in `dispatch.rs` is built at compile time.
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::TypeScript => &["ts", "tsx"],
            Self::CSharp => &["cs"],
            Self::Python => &["py"],
            Self::Go => &["go"],
            Self::Java => &["java"],
            Self::Rust => &["rs"],
            Self::Ruby => &["rb"],
        }
    }

    /// Interpreter tokens this language responds to in a shebang line
    /// (e.g. `#!/usr/bin/env python3` → `"python3"`).
    ///
    /// Currently unused by the indexer (extension dispatch covers every
    /// indexed file). Reserved for the cheap-path detection extension
    /// noted in BACKLOG.md.
    pub const fn shebangs(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["python", "python3"],
            Self::Ruby => &["ruby"],
            Self::TypeScript | Self::CSharp | Self::Go | Self::Java | Self::Rust => &[],
        }
    }

    /// tree-sitter grammar for this language.
    pub fn ts_language(self) -> Language {
        match self {
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        }
    }

    pub fn symbol_query_source(self) -> &'static str {
        match self {
            Self::TypeScript => include_str!("../queries/typescript/symbols.scm"),
            Self::CSharp => include_str!("../queries/csharp/symbols.scm"),
            Self::Python => include_str!("../queries/python/symbols.scm"),
            Self::Go => include_str!("../queries/go/symbols.scm"),
            Self::Java => include_str!("../queries/java/symbols.scm"),
            Self::Rust => include_str!("../queries/rust/symbols.scm"),
            Self::Ruby => include_str!("../queries/ruby/symbols.scm"),
        }
    }

    pub fn edge_query_source(self) -> &'static str {
        match self {
            Self::TypeScript => include_str!("../queries/typescript/edges.scm"),
            Self::CSharp => include_str!("../queries/csharp/edges.scm"),
            Self::Python => include_str!("../queries/python/edges.scm"),
            Self::Go => include_str!("../queries/go/edges.scm"),
            Self::Java => include_str!("../queries/java/edges.scm"),
            Self::Rust => include_str!("../queries/rust/edges.scm"),
            Self::Ruby => include_str!("../queries/ruby/edges.scm"),
        }
    }

    pub fn symbol_kind_for_node(self, node_kind: &str) -> &'static str {
        match self {
            Self::TypeScript => match node_kind {
                "function_declaration" => "function",
                "class_declaration" => "class",
                "method_definition" => "method",
                "interface_declaration" => "interface",
                "enum_declaration" => "enum",
                "type_alias_declaration" => "type",
                "public_field_definition" => "property",
                "lexical_declaration" | "arrow_function" | "function_expression" => "function",
                "enum_assignment" | "property_identifier" => "variant",
                _ => "function",
            },
            Self::CSharp => match node_kind {
                "class_declaration" => "class",
                "method_declaration" => "method",
                "constructor_declaration" => "method",
                "property_declaration" => "property",
                "interface_declaration" => "interface",
                "enum_declaration" => "enum",
                "struct_declaration" => "struct",
                "record_declaration" => "class",
                "delegate_declaration" => "type",
                "enum_member_declaration" => "variant",
                _ => "function",
            },
            Self::Python => match node_kind {
                "function_definition" => "function",
                "class_definition" => "class",
                _ => "function",
            },
            Self::Go => match node_kind {
                "function_declaration" => "function",
                "method_declaration" => "method",
                "type_spec" => "struct",
                "const_spec" => "const",
                _ => "function",
            },
            Self::Java => match node_kind {
                "class_declaration" => "class",
                "interface_declaration" => "interface",
                "enum_declaration" => "enum",
                "record_declaration" => "class",
                "method_declaration" => "method",
                "constructor_declaration" => "method",
                "field_declaration" => "property",
                "annotation_type_declaration" => "type",
                "enum_constant" => "variant",
                _ => "function",
            },
            Self::Rust => match node_kind {
                "function_item" => "function",
                "struct_item" => "struct",
                "enum_item" => "enum",
                "trait_item" => "interface",
                "type_item" => "type",
                "const_item" | "static_item" => "const",
                "enum_variant" => "variant",
                _ => "function",
            },
            Self::Ruby => match node_kind {
                "class" => "class",
                "module" => "interface",
                "method" | "singleton_method" => "method",
                "assignment" => "const",
                "lambda" | "call" => "function",
                _ => "class",
            },
        }
    }

    pub fn scope_node_types(self) -> &'static [&'static str] {
        match self {
            Self::TypeScript => &[
                "function_declaration",
                "method_definition",
                "arrow_function",
                "function_expression",
                "class_declaration",
                "interface_declaration",
            ],
            Self::CSharp => &[
                "method_declaration",
                "constructor_declaration",
                "class_declaration",
                "struct_declaration",
                "interface_declaration",
                "record_declaration",
            ],
            Self::Python => &[
                "function_definition",
                "class_definition",
                "decorated_definition",
                "module",
            ],
            Self::Go => &["function_declaration", "method_declaration", "func_literal"],
            Self::Java => &[
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
                "method_declaration",
                "constructor_declaration",
                "lambda_expression",
            ],
            Self::Rust => &["function_item", "impl_item", "trait_item", "mod_item"],
            Self::Ruby => &[
                "class",
                "module",
                "method",
                "singleton_method",
                "lambda",
                "call",
            ],
        }
    }

    pub fn class_body_node_types(self) -> &'static [&'static str] {
        match self {
            Self::TypeScript => &["class_body", "enum_body"],
            Self::CSharp => &["declaration_list", "enum_member_declaration_list"],
            Self::Python => &["block"],
            Self::Go => &["field_declaration_list"],
            Self::Java => &["class_body", "interface_body", "enum_body"],
            // Rust impl blocks contain a `declaration_list` body, but `impl_item`
            // is not stored as a symbol. The standard `find_parent_class` would
            // generate a parent_id referencing a non-existent symbol, causing FK
            // constraint errors. Only `enum_variant_list` is included so enum
            // variants get their parent enum.
            Self::Rust => &["enum_variant_list"],
            Self::Ruby => &["body_statement"],
        }
    }

    pub fn class_decl_node_types(self) -> &'static [&'static str] {
        match self {
            Self::TypeScript => &["class_declaration", "enum_declaration"],
            Self::CSharp => &[
                "class_declaration",
                "struct_declaration",
                "interface_declaration",
                "record_declaration",
                "enum_declaration",
            ],
            Self::Python => &["class_definition"],
            Self::Go => &["type_declaration"],
            Self::Java => &[
                "class_declaration",
                "interface_declaration",
                "enum_declaration",
            ],
            Self::Rust => &["enum_item"],
            Self::Ruby => &["class", "module"],
        }
    }

    /// Per R4, `ctx` carries typed workspace state. Phase B plugins ignore
    /// it; R2 wires the first real consumer (Python package
    /// resolution via `__init__.py`).
    pub fn extract_metadata(
        self,
        node: &tree_sitter::Node,
        source: &str,
        kind: &str,
        _ctx: &dyn LanguageWorkspaceContext,
    ) -> Result<String> {
        match self {
            Self::TypeScript => crate::languages::typescript::extract_metadata(node, source, kind),
            Self::CSharp => crate::languages::csharp::extract_metadata(node, source, kind),
            Self::Python => crate::languages::python::extract_metadata(node, source, kind),
            Self::Go => crate::languages::go_lang::extract_metadata(node, source, kind),
            Self::Java => crate::languages::java::extract_metadata(node, source, kind),
            Self::Rust => crate::languages::rust_lang::extract_metadata(node, source, kind),
            Self::Ruby => crate::languages::ruby::extract_metadata(node, source, kind),
        }
    }

    /// Plugin-driven skipped ranges (R2 / R6).
    ///
    /// Sub-trees the plugin chose **not** to analyse — e.g., a macro body the
    /// plugin cannot interpret meaningfully. Each entry is propagated verbatim
    /// per Charter §3 invariant 5; the indexer concatenates these with
    /// tree-sitter-error skips into `file_hashes.skipped_ranges`.
    ///
    /// Default for every language is "no plugin-driven skips". Languages opt in
    /// later as concrete cases land (e.g., Rust unparseable `macro_rules!`
    /// bodies in a future sprint). Adding a new `LanguageId` variant forces
    /// the author to pick an arm here — the compiler refuses to build
    /// otherwise.
    pub fn plugin_skipped_ranges(
        self,
        _root: &tree_sitter::Node,
        _source: &str,
    ) -> Vec<crate::extract::SkippedRange> {
        match self {
            Self::TypeScript
            | Self::CSharp
            | Self::Python
            | Self::Go
            | Self::Java
            | Self::Rust
            | Self::Ruby => Vec::new(),
        }
    }

    /// Default is "previous sibling comment node, trimmed". Python and Ruby
    /// override (docstring is the first body statement). Go merges runs of
    /// consecutive `//` comments. Java accepts `block_comment` too.
    pub fn extract_docstring(self, node: &tree_sitter::Node, source: &str) -> Option<String> {
        match self {
            Self::Python => crate::languages::python::extract_docstring(node, source),
            Self::Ruby => crate::languages::ruby::extract_docstring(node, source),
            Self::Go => extract_go_docstring(node, source),
            Self::Java => extract_java_docstring(node, source),
            Self::TypeScript | Self::CSharp | Self::Rust => {
                default_prev_comment_docstring(node, source)
            }
        }
    }

    /// Symbol names too generic to boost in FTS5 search ranking.
    ///
    /// Exhaustive over the enum — replaces the silent `_ => &[]` fallback
    /// that used to live in `stopwords_for_language(&str)` (C.1).
    pub const fn generic_name_stopwords(self) -> &'static [&'static str] {
        match self {
            Self::TypeScript => &["constructor", "toString", "valueOf", "render", "default"],
            Self::CSharp => &[
                "ToString",
                "GetHashCode",
                "Equals",
                "Dispose",
                "GetType",
                "Main",
            ],
            Self::Python => &[
                "__init__", "__str__", "__repr__", "__eq__", "__hash__", "__len__", "__iter__",
                "__next__",
            ],
            Self::Go => &[
                "String", "Error", "Close", "Read", "Write", "New", "Init", "Run",
            ],
            Self::Java => &[
                "toString", "hashCode", "equals", "get", "set", "of", "main", "run", "close",
            ],
            Self::Rust => &[
                "new", "default", "from", "into", "run", "build", "try_from", "fmt", "clone",
                "drop",
            ],
            Self::Ruby => &[
                "new",
                "initialize",
                "call",
                "to_s",
                "inspect",
                "class",
                "module",
            ],
        }
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::TypeScript => "TypeScript",
            Self::CSharp => "C#",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::Rust => "Rust",
            Self::Ruby => "Ruby",
        };
        f.write_str(name)
    }
}

fn default_prev_comment_docstring(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() == "comment" {
        let text = prev.utf8_text(source.as_bytes()).ok()?;
        Some(text.trim().to_string())
    } else {
        None
    }
}

/// Go merges consecutive `//` lines into one docstring.
fn extract_go_docstring(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let mut lines = Vec::new();
    let mut current = node.prev_sibling();

    while let Some(prev) = current {
        if prev.kind() == "comment" {
            if let Ok(text) = prev.utf8_text(source.as_bytes()) {
                let cleaned = text.trim().trim_start_matches("//").trim();
                lines.push(cleaned.to_string());
            }
            current = prev.prev_sibling();
        } else {
            break;
        }
    }

    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

/// Java accepts both line and block comments for Javadoc.
fn extract_java_docstring(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let prev = node.prev_sibling()?;
    match prev.kind() {
        "block_comment" | "line_comment" => {
            let text = prev.utf8_text(source.as_bytes()).ok()?;
            Some(text.trim().to_string())
        }
        _ => None,
    }
}
