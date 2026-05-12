//! Human-readable output rendering shared across every Scope command.
//!
//! After R10, plain-text output goes through `impl fmt::Display` on
//! per-command view structs defined in this module and exposed by
//! [`crate::output::schema`]. Callers write `print!("{view}")` instead
//! of invoking free `print_*` functions; the `view` is constructed
//! once and feeds both the JSON envelope (when `--json` is set) and
//! the plain renderer. Procedural `println!` survives only inside the
//! `fmt::Display` bodies, where it is rewritten to `writeln!(f, …)?`.
//!
//! Rules:
//! - Separator line uses `─` (U+2500), never `-` or `=`
//! - File paths always use forward slashes, even on Windows
//! - Line ranges formatted as `start-end`
//! - Caller counts in square brackets: `[11 callers]`, `[internal]`
//! - Similarity scores always 2 decimal places: `0.91`

use std::collections::HashMap;
use std::fmt;

use crate::commands::entrypoints::EntrypointInfo;
use crate::commands::flow::FlowPath;
use crate::commands::map::{CoreSymbol, DirStats, MapStats};
use gumiho_mudang_scope::graph::{
    CallerInfo, ClassRelationships, Dependency, ImpactResult, Reference, Symbol, TraceResult,
};
use gumiho_mudang_scope::searcher::SearchResult;

/// The separator line used between header and body in all command output.
pub const SEPARATOR: &str =
    "──────────────────────────────────────────────────────────────────────────────";

/// Normalize a file path to always use forward slashes in output.
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Format a line range as `start-end`, or just `start` if start == end.
pub fn format_line_range(start: u32, end: u32) -> String {
    if start == end {
        format!("{start}")
    } else {
        format!("{start}-{end}")
    }
}

/// Write the header line `name  kind  file:line_range` followed by
/// the separator. Shared by every symbol-scoped sketch renderer.
fn write_header(symbol: &Symbol, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let path = normalize_path(&symbol.file_path);
    let line_range = format_line_range(symbol.line_start, symbol.line_end);
    writeln!(
        f,
        "{:<50}{}  {}:{}",
        symbol.name, symbol.kind, path, line_range
    )?;
    writeln!(f, "{SEPARATOR}")
}

/// Print a class sketch.
///
/// Format:
/// ```text
/// PaymentService                                    class  src/payments/service.ts:12
/// ──────────────────────────────────────────────────────────────────────────────
/// deps:     StripeClient, UserRepository, Logger
/// extends:  BaseService
/// implements: IPaymentService
///
/// methods:
///   processPayment(amount: Decimal, userId: string) → PaymentResult       [11 callers]
///   validateCard(card: CardDetails) → ValidationResult                     [internal]
///
/// fields:
///   private client: StripeClient
/// ```
/// Plain-text view for `scope sketch <class|struct>`.
pub struct ClassSketchView<'a> {
    pub symbol: &'a Symbol,
    pub methods: &'a [Symbol],
    pub caller_counts: &'a HashMap<String, usize>,
    pub relationships: &'a ClassRelationships,
    pub limit: usize,
    pub show_docs: bool,
}

impl fmt::Display for ClassSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_header(self.symbol, f)?;

        if !self.relationships.dependencies.is_empty() {
            writeln!(
                f,
                "deps:     {}",
                self.relationships.dependencies.join(", ")
            )?;
        }
        if !self.relationships.extends.is_empty() {
            writeln!(f, "extends:  {}", self.relationships.extends.join(", "))?;
        }
        if !self.relationships.implements.is_empty() {
            writeln!(
                f,
                "implements: {}",
                self.relationships.implements.join(", ")
            )?;
        }

        let (method_syms, field_syms): (Vec<&Symbol>, Vec<&Symbol>) = self
            .methods
            .iter()
            .partition(|m| m.kind == "method" || m.kind == "function");

        if !method_syms.is_empty() {
            writeln!(f)?;
            writeln!(f, "methods:")?;
            let display_methods = if method_syms.len() > self.limit {
                &method_syms[..self.limit]
            } else {
                &method_syms
            };

            for method in display_methods {
                if self.show_docs {
                    if let Some(ref doc) = method.docstring {
                        let first_line = doc.lines().next().unwrap_or("").trim();
                        let clean = first_line
                            .trim_start_matches("///")
                            .trim_start_matches("//")
                            .trim_start_matches("/**")
                            .trim_start_matches("*")
                            .trim_start_matches("*/")
                            .trim();
                        if !clean.is_empty() {
                            writeln!(f, "  /// {clean}")?;
                        }
                    }
                }

                let sig = method_display_line(method);
                let count = self.caller_counts.get(&method.id).copied().unwrap_or(0);
                let count_label = if count > 0 {
                    format!("[{count} caller{}]", if count == 1 { "" } else { "s" })
                } else {
                    "[internal]".to_string()
                };
                let padding = SEPARATOR
                    .chars()
                    .count()
                    .saturating_sub(2 + sig.chars().count() + count_label.chars().count());
                writeln!(
                    f,
                    "  {sig}{:>width$}",
                    count_label,
                    width = padding + count_label.len()
                )?;
            }

            if method_syms.len() > self.limit {
                writeln!(
                    f,
                    "  ... {} more (use --limit to show more)",
                    method_syms.len() - self.limit
                )?;
            }
        }

        let field_syms: Vec<&Symbol> = field_syms
            .into_iter()
            .filter(|s| s.kind == "property")
            .collect();

        if !field_syms.is_empty() {
            writeln!(f)?;
            writeln!(f, "fields:")?;
            for field in &field_syms {
                let sig = field.signature.as_deref().unwrap_or(&field.name);
                writeln!(f, "  {sig}")?;
            }
        }

        Ok(())
    }
}

/// Print a method/function sketch.
///
/// Format:
/// ```text
/// processPayment                        method  src/payments/service.ts:34-67
/// ──────────────────────────────────────────────────────────────────────────────
/// signature:  (amount: Decimal, userId: string) → PaymentResult
/// calls:      validateCard, repo.findUser
/// called by:  OrderController.checkout [x3]
/// ```
/// Plain-text view for `scope sketch <method|function>`.
pub struct MethodSketchView<'a> {
    pub symbol: &'a Symbol,
    pub outgoing_calls: &'a [String],
    pub incoming_callers: &'a [CallerInfo],
}

impl fmt::Display for MethodSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_header(self.symbol, f)?;

        let enrichment = extract_enrichment_prefix(&self.symbol.language, &self.symbol.metadata);
        if !enrichment.is_empty() {
            writeln!(f, "{enrichment}")?;
        }

        let modifiers = extract_modifiers(&self.symbol.metadata);
        if !modifiers.is_empty() {
            writeln!(f, "{}", modifiers.join(" "))?;
        }

        if let Some(sig) = &self.symbol.signature {
            writeln!(f, "signature:  {sig}")?;
        }

        if !self.outgoing_calls.is_empty() {
            writeln!(f, "calls:      {}", self.outgoing_calls.join(", "))?;
        }

        if !self.incoming_callers.is_empty() {
            let caller_parts: Vec<String> = self
                .incoming_callers
                .iter()
                .map(|c| {
                    if c.count > 1 {
                        format!("{} [x{}]", c.name, c.count)
                    } else {
                        c.name.clone()
                    }
                })
                .collect();
            writeln!(f, "called by:  {}", caller_parts.join(", "))?;
        }

        Ok(())
    }
}

/// Print an interface sketch.
///
/// Format:
/// ```text
/// IPaymentService                          interface  src/types/payment.ts:4
/// ──────────────────────────────────────────────────────────────────────────────
/// implemented by:  PaymentService
///
/// methods:
///   processPayment(amount: Decimal, userId: string) → Promise<PaymentResult>
/// ```
/// Plain-text view for `scope sketch <interface>`.
pub struct InterfaceSketchView<'a> {
    pub symbol: &'a Symbol,
    pub methods: &'a [Symbol],
    pub implementors: &'a [String],
    pub relationships: &'a ClassRelationships,
    pub limit: usize,
}

impl fmt::Display for InterfaceSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_header(self.symbol, f)?;

        if !self.relationships.dependencies.is_empty() {
            writeln!(
                f,
                "deps:     {}",
                self.relationships.dependencies.join(", ")
            )?;
        }
        if !self.relationships.extends.is_empty() {
            writeln!(f, "extends:  {}", self.relationships.extends.join(", "))?;
        }
        if !self.relationships.implements.is_empty() {
            writeln!(
                f,
                "implements: {}",
                self.relationships.implements.join(", ")
            )?;
        }
        if !self.implementors.is_empty() {
            writeln!(f, "implemented by:  {}", self.implementors.join(", "))?;
        }

        if !self.methods.is_empty() {
            writeln!(f)?;
            writeln!(f, "methods:")?;
            let display_methods = if self.methods.len() > self.limit {
                &self.methods[..self.limit]
            } else {
                self.methods
            };

            for method in display_methods {
                let sig = method_display_line(method);
                writeln!(f, "  {sig}")?;
            }

            if self.methods.len() > self.limit {
                writeln!(
                    f,
                    "  ... {} more (use --limit to show more)",
                    self.methods.len() - self.limit
                )?;
            }
        }

        Ok(())
    }
}

/// Print a file-level sketch.
///
/// Format:
/// ```text
/// src/payments/service.ts
/// ──────────────────────────────────────────────────────────────────────────────
///   PaymentService          class     12-89    [11 callers]
///   processPayment          method    34-67    [11 callers]
/// ```
/// Plain-text view for `scope sketch <file path>`.
pub struct FileSketchView<'a> {
    pub file_path: &'a str,
    pub symbols: &'a [Symbol],
    pub caller_counts: &'a HashMap<String, usize>,
}

impl fmt::Display for FileSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = normalize_path(self.file_path);
        writeln!(f, "{path}")?;
        writeln!(f, "{SEPARATOR}")?;

        for sym in self.symbols {
            let line_range = format_line_range(sym.line_start, sym.line_end);
            let count = self.caller_counts.get(&sym.id).copied().unwrap_or(0);
            let count_label = if count > 0 {
                format!("[{count} caller{}]", if count == 1 { "" } else { "s" })
            } else {
                "[internal]".to_string()
            };
            writeln!(
                f,
                "  {:<24}{:<10}{:<9}{}",
                sym.name, sym.kind, line_range, count_label
            )?;
        }

        Ok(())
    }
}

/// Plain-text view for `scope sketch <enum>`.
///
/// Format:
/// ```text
/// PaymentStatus                                     enum  src/payments/types.ts:1-6
/// ──────────────────────────────────────────────────────────────────────────────
/// variants:
///   Active
///   Inactive
///   Pending
///
/// [3 callers]
/// ```
pub struct EnumSketchView<'a> {
    pub symbol: &'a Symbol,
    pub variants: &'a [&'a Symbol],
    pub caller_count: usize,
}

impl fmt::Display for EnumSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_header(self.symbol, f)?;

        if !self.variants.is_empty() {
            writeln!(f, "variants:")?;
            for v in self.variants {
                let display = v.signature.as_deref().unwrap_or(&v.name);
                writeln!(f, "  {display}")?;
            }
        }

        writeln!(f)?;
        if self.caller_count > 0 {
            writeln!(
                f,
                "[{} caller{}]",
                self.caller_count,
                if self.caller_count == 1 { "" } else { "s" }
            )
        } else {
            writeln!(f, "[internal]")
        }
    }
}

/// Plain-text view for `scope sketch <const|type|struct>` — header + signature.
pub struct GenericSketchView<'a> {
    pub symbol: &'a Symbol,
}

impl fmt::Display for GenericSketchView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_header(self.symbol, f)?;
        if let Some(sig) = &self.symbol.signature {
            writeln!(f, "signature:  {sig}")?;
        }
        Ok(())
    }
}

/// Build the display string for a method in a class/interface listing.
///
/// Uses the signature if available, otherwise just the name.
/// Prepends any modifiers from metadata that are not already present in the signature.
/// Adds language-specific enrichments: Java annotations, Python decorators, Go receivers.
fn method_display_line(method: &Symbol) -> String {
    let sig = method.signature.as_deref().unwrap_or(&method.name);

    let modifiers = extract_modifiers(&method.metadata);
    let base = if modifiers.is_empty() {
        sig.to_string()
    } else {
        // Only prepend modifiers not already present in the signature text
        let missing: Vec<&str> = modifiers
            .iter()
            .filter(|m| !sig.contains(m.as_str()))
            .map(|m| m.as_str())
            .collect();

        if missing.is_empty() {
            sig.to_string()
        } else {
            format!("{} {}", missing.join(" "), sig)
        }
    };

    // Add language-specific enrichment prefix
    let enrichment = extract_enrichment_prefix(&method.language, &method.metadata);
    if enrichment.is_empty() {
        base
    } else {
        format!("{enrichment} {base}")
    }
}

/// Extract a language-specific enrichment prefix from a symbol's metadata JSON.
///
/// - **Java**: annotations (e.g. `@Override`, `@Deprecated`)
/// - **Python**: decorators (e.g. `@staticmethod`, `@property`)
/// - **Go**: receiver type (e.g. `(s *Server)`)
///
/// Returns an empty string if the language has no enrichments or parsing fails.
fn extract_enrichment_prefix(language: &str, metadata_json: &str) -> String {
    let parsed: serde_json::Value = match serde_json::from_str(metadata_json) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    match language {
        "java" => {
            // Show annotations as @Name prefixes. R2 (sprint 0003 chunk 3b)
            // migrated the per-entry shape from string to object — read
            // `.name` instead of treating each entry as a bare string.
            if let Some(annotations) = parsed.get("annotations").and_then(|v| v.as_array()) {
                let prefixes: Vec<String> = annotations
                    .iter()
                    .filter_map(|a| a.get("name").and_then(|v| v.as_str()))
                    .filter(|a| !a.is_empty())
                    .map(|a| format!("@{a}"))
                    .collect();
                if !prefixes.is_empty() {
                    return prefixes.join(" ");
                }
            }
            String::new()
        }
        "python" => {
            // Show decorators as @name prefixes. R2 (sprint 0003 chunk 3b)
            // migrated the per-entry shape from string to object — read
            // `.name` instead of treating each entry as a bare string.
            if let Some(decorators) = parsed.get("decorators").and_then(|v| v.as_array()) {
                let prefixes: Vec<String> = decorators
                    .iter()
                    .filter_map(|d| d.get("name").and_then(|v| v.as_str()))
                    .filter(|d| !d.is_empty())
                    .map(|d| format!("@{d}"))
                    .collect();
                if !prefixes.is_empty() {
                    return prefixes.join(" ");
                }
            }
            String::new()
        }
        "go" => {
            // Show receiver type as (name *Type) or (name Type) prefix
            if let Some(receiver) = parsed.get("receiver").and_then(|v| v.as_str()) {
                if !receiver.is_empty() {
                    let is_pointer = parsed
                        .get("is_pointer_receiver")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    // Extract a short variable name from the receiver type
                    let var_name = receiver
                        .chars()
                        .next()
                        .map(|c| c.to_lowercase().to_string())
                        .unwrap_or_default();
                    let type_display = if is_pointer {
                        format!("*{receiver}")
                    } else {
                        receiver.to_string()
                    };
                    return format!("({var_name} {type_display})");
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

/// Extract display-worthy modifiers from a symbol's metadata JSON.
///
/// Returns modifiers that differ from defaults (public is default, so omit it).
/// Example output: `vec!["async", "private", "static"]`
fn extract_modifiers(metadata_json: &str) -> Vec<String> {
    let parsed: serde_json::Value = match serde_json::from_str(metadata_json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut mods = Vec::new();

    // Access modifier (only show non-public)
    if let Some(access) = parsed.get("access").and_then(|v| v.as_str()) {
        match access {
            "private" | "protected" | "internal" | "protected internal" => {
                mods.push(access.to_string());
            }
            _ => {} // "public" is default, don't show
        }
    }

    if parsed
        .get("is_async")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        mods.push("async".to_string());
    }

    if parsed
        .get("is_static")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        mods.push("static".to_string());
    }

    if parsed
        .get("is_abstract")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        mods.push("abstract".to_string());
    }

    if parsed
        .get("is_virtual")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        mods.push("virtual".to_string());
    }

    if parsed
        .get("is_override")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        mods.push("override".to_string());
    }

    mods
}

/// Print references to a function or method (flat list).
///
/// Format:
/// ```text
/// processPayment — 11 references
/// ──────────────────────────────────────────────────────────────────────────────
/// src/controllers/order.ts:89       OrderController.checkout
/// src/controllers/order.ts:134      OrderController.retryPayment
/// ... 8 more (use --limit to show more)
/// ```
/// Plain-text view for a flat `scope refs <symbol>` listing.
pub struct RefsView<'a> {
    pub symbol_name: &'a str,
    pub refs: &'a [Reference],
    pub total: usize,
}

impl fmt::Display for RefsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} \u{2014} {} reference{}",
            self.symbol_name,
            self.total,
            if self.total == 1 { "" } else { "s" }
        )?;
        writeln!(f, "{SEPARATOR}")?;

        for r in self.refs {
            let path = normalize_path(&r.file_path);
            let location = if let Some(line) = r.line {
                format!("{path}:{line}")
            } else {
                path
            };
            let display_text = r.snippet_line.as_deref().unwrap_or(&r.context);
            let truncated_text = truncate_str(display_text.trim(), 80);
            writeln!(f, "{:<40}{}", location, truncated_text)?;

            if let Some(ref snippet) = r.snippet {
                write_snippet_context(snippet, r.line, f)?;
            }
        }

        if self.refs.len() < self.total {
            writeln!(
                f,
                "... {} more (use --limit to show more)",
                self.total - self.refs.len()
            )?;
        }
        Ok(())
    }
}

/// Print references to a class symbol, grouped by kind.
///
/// Format:
/// ```text
/// PaymentService — 18 references
/// ──────────────────────────────────────────────────────────────────────────────
/// instantiated (4):
///   src/controllers/order.ts:23       new PaymentService(config)
///   ...
///
/// extended (1):
///   src/payments/stripe-service.ts:4  class StripeService extends PaymentService
/// ```
/// Plain-text view for `scope refs <class>` with kind-grouped buckets.
pub struct RefsGroupedView<'a> {
    pub symbol_name: &'a str,
    pub groups: &'a [(String, Vec<Reference>)],
    pub total: usize,
}

impl fmt::Display for RefsGroupedView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} \u{2014} {} reference{}",
            self.symbol_name,
            self.total,
            if self.total == 1 { "" } else { "s" }
        )?;
        writeln!(f, "{SEPARATOR}")?;

        let mut shown = 0;
        for (kind, refs) in self.groups {
            let kind_label = humanize_edge_kind(kind);
            writeln!(f, "{kind_label} ({}):", refs.len())?;
            for r in refs {
                let path = normalize_path(&r.file_path);
                let location = if let Some(line) = r.line {
                    format!("{path}:{line}")
                } else {
                    path
                };
                let display_text = r.snippet_line.as_deref().unwrap_or(&r.context);
                let truncated_text = truncate_str(display_text.trim(), 80);
                writeln!(f, "  {:<38}{}", location, truncated_text)?;

                if let Some(ref snippet) = r.snippet {
                    write_snippet_context(snippet, r.line, f)?;
                }
            }
            shown += refs.len();
            writeln!(f)?;
        }

        if shown < self.total {
            writeln!(
                f,
                "... {} more (use --limit to show more)",
                self.total - shown
            )?;
        }
        Ok(())
    }
}

/// Print file-level references.
///
/// Same as `print_refs` but with the file path as header.
/// Plain-text view for `scope refs <file path>`.
pub struct FileRefsView<'a> {
    pub file_path: &'a str,
    pub refs: &'a [Reference],
    pub total: usize,
}

impl fmt::Display for FileRefsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = normalize_path(self.file_path);
        writeln!(
            f,
            "{} \u{2014} {} reference{}",
            path,
            self.total,
            if self.total == 1 { "" } else { "s" }
        )?;
        writeln!(f, "{SEPARATOR}")?;

        for r in self.refs {
            let rpath = normalize_path(&r.file_path);
            let location = if let Some(line) = r.line {
                format!("{rpath}:{line}")
            } else {
                rpath
            };
            let display_text = r.snippet_line.as_deref().unwrap_or(&r.context);
            let truncated_text = truncate_str(display_text.trim(), 80);
            writeln!(f, "{:<40}{}", location, truncated_text)?;

            if let Some(ref snippet) = r.snippet {
                write_snippet_context(snippet, r.line, f)?;
            }
        }

        if self.refs.len() < self.total {
            writeln!(
                f,
                "... {} more (use --limit to show more)",
                self.total - self.refs.len()
            )?;
        }
        Ok(())
    }
}

/// Print dependencies of a symbol.
///
/// Format:
/// ```text
/// PaymentService — direct dependencies
/// ──────────────────────────────────────────────────────────────────────────────
/// imports:
///   StripeClient            src/clients/stripe.ts
///   Decimal                 (external)
///
/// calls:
///   stripe.charges.create   (external)
/// ```
/// Shared body for the symbol- and file-scoped dep renderers — emits
/// the header line, separator, and kind-grouped dep listing.
fn write_deps_body(
    header_lhs: &str,
    deps: &[Dependency],
    max_depth: usize,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let depth_label = if max_depth <= 1 {
        "direct dependencies".to_string()
    } else {
        format!("transitive dependencies (depth {max_depth})")
    };

    writeln!(f, "{} \u{2014} {}", header_lhs, depth_label)?;
    writeln!(f, "{SEPARATOR}")?;

    if deps.is_empty() {
        writeln!(f, "(no dependencies found)")?;
        return Ok(());
    }

    let mut groups: Vec<(String, Vec<&Dependency>)> = Vec::new();
    for dep in deps {
        if let Some(group) = groups.iter_mut().find(|(k, _)| *k == dep.kind) {
            group.1.push(dep);
        } else {
            let kind = dep.kind.clone();
            groups.push((kind, vec![dep]));
        }
    }

    for (kind, group_deps) in &groups {
        let all_external = group_deps.iter().all(|d| d.is_external);
        let kind_label = if all_external {
            format!("{kind} (external):")
        } else {
            format!("{kind}:")
        };
        writeln!(f, "{kind_label}")?;

        for dep in group_deps {
            if dep.is_external {
                writeln!(f, "  {:<24}(external)", dep.name)?;
            } else if let Some(fp) = &dep.file_path {
                let path = normalize_path(fp);
                writeln!(f, "  {:<24}{}", dep.name, path)?;
            } else {
                writeln!(f, "  {}", dep.name)?;
            }
        }

        writeln!(f)?;
    }
    Ok(())
}

/// Plain-text view for `scope deps <symbol>`.
pub struct DepsView<'a> {
    pub symbol_name: &'a str,
    pub deps: &'a [Dependency],
    pub max_depth: usize,
}

impl fmt::Display for DepsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_deps_body(self.symbol_name, self.deps, self.max_depth, f)
    }
}

/// Plain-text view for `scope deps <file path>`.
pub struct FileDepsView<'a> {
    pub file_path: &'a str,
    pub deps: &'a [Dependency],
    pub max_depth: usize,
}

impl fmt::Display for FileDepsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = normalize_path(self.file_path);
        write_deps_body(&path, self.deps, self.max_depth, f)
    }
}

/// Print an impact analysis result.
///
/// Format:
/// ```text
/// Impact analysis: processPayment
/// ──────────────────────────────────────────────────────────────────────────────
/// Direct callers (11):
///   OrderController.checkout          src/controllers/order.ts
///   SubscriptionService.renew         src/services/subscription.ts
///   ... (9 more)
///
/// Second-degree (3):
///   src/api/routes/checkout.ts        → imports OrderController
///
/// Test files affected: 6
///   tests/unit/payment.test.ts
///   tests/unit/order.test.ts
///   ... (4 more)
/// ```
/// Plain-text view for `scope callers <symbol> --depth N` when
/// `N > 1` (uses the underlying `ImpactResult`).
pub struct ImpactView<'a> {
    pub symbol_name: &'a str,
    pub result: &'a ImpactResult,
}

impl fmt::Display for ImpactView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Impact analysis: {}", self.symbol_name)?;
        writeln!(f, "{SEPARATOR}")?;

        if self.result.nodes_by_depth.is_empty() && self.result.test_files.is_empty() {
            writeln!(f, "(no impact detected)")?;
            return Ok(());
        }

        for (depth, nodes) in &self.result.nodes_by_depth {
            let depth_label = impact_depth_label(*depth);
            writeln!(f, "{depth_label} ({}):", nodes.len())?;

            let max_display = 10;
            let display_nodes = if nodes.len() > max_display {
                &nodes[..max_display]
            } else {
                nodes.as_slice()
            };

            for node in display_nodes {
                let path = normalize_path(&node.file_path);
                writeln!(f, "  {:<40}{}", node.name, path)?;
            }

            if nodes.len() > max_display {
                writeln!(f, "  ... ({} more)", nodes.len() - max_display)?;
            }

            writeln!(f)?;
        }

        if !self.result.test_files.is_empty() {
            writeln!(f, "Test files affected: {}", self.result.test_files.len())?;

            let max_display = 10;
            let display_tests = if self.result.test_files.len() > max_display {
                &self.result.test_files[..max_display]
            } else {
                self.result.test_files.as_slice()
            };

            for node in display_tests {
                let path = normalize_path(&node.file_path);
                writeln!(f, "  {path}")?;
            }

            if self.result.test_files.len() > max_display {
                writeln!(
                    f,
                    "  ... ({} more)",
                    self.result.test_files.len() - max_display
                )?;
            }
        }

        Ok(())
    }
}

/// Print trace results showing call paths from entry points to the target.
///
/// Format:
/// ```text
/// processRenewal — 2 entry paths
/// ──────────────────────────────────────────────────────────────────────────────
/// Path 1: SubscriptionController.renewSubscription
///   └─→ SubscriptionService.processRenewal          src/services/sub.ts:72
///
/// Path 2: SubscriptionRenewalWorker.autoRenewDue
///   └─→ SubscriptionService.processRenewal          src/services/sub.ts:72
/// ```
/// Plain-text view for `scope trace <symbol>`.
pub struct TraceView<'a> {
    pub symbol_name: &'a str,
    pub result: &'a TraceResult,
    pub total: usize,
    pub truncated: bool,
}

impl fmt::Display for TraceView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_count = self.result.paths.len();
        let path_word = if path_count == 1 { "path" } else { "paths" };

        let display_count = if self.truncated {
            self.total
        } else {
            path_count
        };
        writeln!(
            f,
            "{} \u{2014} {} entry {}",
            self.symbol_name, display_count, path_word
        )?;
        writeln!(f, "{SEPARATOR}")?;

        if self.result.paths.is_empty() {
            writeln!(f, "(no entry paths found)")?;
            return Ok(());
        }

        for (i, call_path) in self.result.paths.iter().enumerate() {
            if call_path.steps.is_empty() {
                continue;
            }

            let entry = &call_path.steps[0];
            writeln!(f, "Path {}: {}", i + 1, entry.symbol_name)?;

            for (step_idx, step) in call_path.steps.iter().enumerate().skip(1) {
                let indent = "  ".repeat(step_idx);
                let path = normalize_path(&step.file_path);
                let location = format!("{path}:{}", step.line);
                writeln!(
                    f,
                    "{indent}\u{2514}\u{2500}\u{2192} {:<40}{}",
                    step.symbol_name, location
                )?;
            }

            if i < path_count - 1 {
                writeln!(f)?;
            }
        }

        if self.truncated {
            writeln!(
                f,
                "... {} more paths (use --limit to show more)",
                self.total - path_count
            )?;
        }

        Ok(())
    }
}

/// Print flow paths between two symbols.
///
/// Format:
/// ```text
/// PaymentService → OrderProcessor → NotificationService
///   src/payments/service.ts:15  →  src/orders/processor.ts:42  →  src/notifications/service.ts:22
///
/// ─ 2 paths found (depth limit: 10)
/// ```
/// Plain-text view for `scope flow <start> <end>`.
pub struct FlowView<'a> {
    pub start: &'a str,
    pub end: &'a str,
    pub paths: &'a [FlowPath],
    pub total: usize,
    pub depth_limit: usize,
}

impl fmt::Display for FlowView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.paths.is_empty() {
            writeln!(
                f,
                "No path found from {} to {} within depth {}.",
                self.start, self.end, self.depth_limit
            )?;
            return Ok(());
        }

        for (i, path) in self.paths.iter().enumerate() {
            let names: Vec<&str> = path.steps.iter().map(|s| s.name.as_str()).collect();
            writeln!(f, "{}", names.join(" \u{2192} "))?;

            let locations: Vec<String> = path
                .steps
                .iter()
                .map(|s| format!("{}:{}", normalize_path(&s.file_path), s.line_start))
                .collect();
            writeln!(f, "  {}", locations.join("  \u{2192}  "))?;

            if i < self.paths.len() - 1 {
                writeln!(f)?;
            }
        }

        let path_word = if self.total == 1 { "path" } else { "paths" };
        writeln!(
            f,
            "\n\u{2500} {} {} found (depth limit: {})",
            self.total, path_word, self.depth_limit
        )
    }
}

/// Print entry points grouped by type.
///
/// Format:
/// ```text
/// Entrypoints — 8 across 6 files
/// ──────────────────────────────────────────────────────────────────────────────
/// API Controllers:
///   PaymentController              src/Api/Controllers/PaymentController.cs       → 3 methods
///   SubscriptionController         src/Api/Controllers/SubscriptionController.cs  → 2 methods
///
/// Background Workers:
///   PaymentRetryWorker             src/Infrastructure/Workers/PaymentRetryWorker.cs
/// ```
/// Plain-text view for `scope entrypoints`.
pub struct EntrypointsView<'a> {
    pub groups: &'a [(String, Vec<EntrypointInfo>)],
    pub total: usize,
    pub file_count: usize,
}

impl fmt::Display for EntrypointsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_word = if self.file_count == 1 { "file" } else { "files" };
        writeln!(
            f,
            "Entrypoints \u{2014} {} across {} {}",
            self.total, self.file_count, file_word
        )?;
        writeln!(f, "{SEPARATOR}")?;

        if self.groups.is_empty() {
            writeln!(f, "(no entry points found)")?;
            return Ok(());
        }

        for (i, (group_name, entries)) in self.groups.iter().enumerate() {
            writeln!(f, "{group_name}:")?;

            let max_name_len = entries
                .iter()
                .map(|e| e.name.chars().count())
                .max()
                .unwrap_or(0);
            let name_width = max_name_len.max(20) + 2;

            for entry in entries {
                let path = normalize_path(&entry.file_path);
                let suffix = if entry.method_count > 0 {
                    format!(
                        "   \u{2192} {} method{}",
                        entry.method_count,
                        if entry.method_count == 1 { "" } else { "s" }
                    )
                } else {
                    String::new()
                };
                writeln!(
                    f,
                    "  {:<width$}{}{}",
                    entry.name,
                    path,
                    suffix,
                    width = name_width
                )?;
            }

            if i < self.groups.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

/// Print a structural map of the entire repository.
///
/// Format:
/// ```text
/// project-name — 181 files, 1,147 symbols, 1,409 edges
/// ──────────────────────────────────────────────────────────────────────────────
/// Languages: C#
///
/// Entry points:
///   PaymentController              Api/Controllers/                → 3 methods
///   SubscriptionController         Api/Controllers/                → 2 methods
///
/// Core symbols (by caller count):
///   ProcessPayment                 7 callers    Infrastructure/Services/PaymentService.cs
///
/// Architecture:
///   Api/                    7 files    62 symbols
///   Application/            22 files   145 symbols
/// ```
/// Plain-text view for `scope map`.
pub struct MapView<'a> {
    pub project_name: &'a str,
    pub stats: &'a MapStats,
    pub entrypoints: &'a [(String, Vec<EntrypointInfo>)],
    pub core_symbols: &'a [CoreSymbol],
    pub directories: &'a [DirStats],
}

impl fmt::Display for MapView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} \u{2014} {} files, {} symbols, {} edges",
            self.project_name,
            format_number(self.stats.file_count),
            format_number(self.stats.symbol_count),
            format_number(self.stats.edge_count),
        )?;
        writeln!(f, "{SEPARATOR}")?;

        if !self.stats.languages.is_empty() {
            writeln!(f, "Languages: {}", self.stats.languages.join(", "))?;
        }

        let mut ep_count = 0usize;
        let mut ep_lines: Vec<String> = Vec::new();

        for (_group_name, entries) in self.entrypoints {
            for entry in entries {
                let path = normalize_path(&entry.file_path);
                let dir = if let Some(pos) = path.rfind('/') {
                    format!("{}/", &path[..pos])
                } else {
                    String::new()
                };

                let display_dir = dir.strip_prefix("src/").unwrap_or(&dir).to_string();

                let suffix = if entry.method_count > 0 {
                    format!(
                        "   \u{2192} {} method{}",
                        entry.method_count,
                        if entry.method_count == 1 { "" } else { "s" }
                    )
                } else {
                    String::new()
                };

                ep_lines.push(format!("  {:<32}{:<32}{}", entry.name, display_dir, suffix));
                ep_count += 1;
            }
        }

        if !ep_lines.is_empty() {
            writeln!(f)?;
            writeln!(f, "Entry points:")?;
            let max_display = 8;
            for line in ep_lines.iter().take(max_display) {
                writeln!(f, "{line}")?;
            }
            if ep_count > max_display {
                writeln!(f, "  ... {} more", ep_count - max_display)?;
            }
        }

        if !self.core_symbols.is_empty() {
            writeln!(f)?;
            writeln!(f, "Core symbols (by caller count):")?;
            for sym in self.core_symbols {
                let path = normalize_path(&sym.file_path);
                let display_path = path.strip_prefix("src/").unwrap_or(&path).to_string();

                let caller_label = format!(
                    "{} caller{}",
                    sym.caller_count,
                    if sym.caller_count == 1 { "" } else { "s" }
                );
                writeln!(
                    f,
                    "  {:<32}{:<14}{}",
                    sym.name, caller_label, display_path
                )?;
            }
        }

        if !self.directories.is_empty() {
            writeln!(f)?;
            writeln!(f, "Architecture:")?;
            for dir in self.directories {
                let file_label = format!(
                    "{} file{}",
                    dir.file_count,
                    if dir.file_count == 1 { "" } else { "s" }
                );
                let sym_label = format!(
                    "{} symbol{}",
                    dir.symbol_count,
                    if dir.symbol_count == 1 { "" } else { "s" }
                );
                writeln!(f, "  {:<24}{:<14}{}", dir.directory, file_label, sym_label)?;
            }
        }
        Ok(())
    }
}

/// Human-readable label for an impact depth level.
fn impact_depth_label(depth: usize) -> &'static str {
    match depth {
        1 => "Direct callers",
        2 => "Second-degree",
        3 => "Third-degree",
        _ => "Further impact",
    }
}

/// Print incremental indexing results.
///
/// Format:
/// ```text
/// 3 files changed. Re-indexing...
///   Modified: src/payments/processor.ts
///   Added:    src/payments/refund.ts
/// Updated in 0.3s.
/// ```
/// Plain-text view for the incremental-index summary. The caller
/// writes this view to **stderr** via `eprint!("{view}")` (it is a
/// progress message, not output data).
pub struct IncrementalResultView<'a> {
    pub modified: &'a [String],
    pub added: &'a [String],
    pub deleted: &'a [String],
    pub duration_secs: f64,
}

impl fmt::Display for IncrementalResultView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.modified.len() + self.added.len() + self.deleted.len();
        writeln!(
            f,
            "{} file{} changed. Re-indexing...",
            total,
            if total == 1 { "" } else { "s" }
        )?;

        for path in self.modified {
            writeln!(f, "  Modified: {}", normalize_path(path))?;
        }
        for path in self.added {
            writeln!(f, "  Added:    {}", normalize_path(path))?;
        }
        for path in self.deleted {
            writeln!(f, "  Deleted:  {}", normalize_path(path))?;
        }

        writeln!(f, "Updated in {:.1}s.", self.duration_secs)
    }
}

/// Print search results from `scope find`.
///
/// Format:
/// ```text
/// Results for: "handles authentication errors"
/// ──────────────────────────────────────────────────────────────────────────────
/// 0.91  AuthMiddleware.handleUnauthorized    src/middleware/auth.ts:34      method
/// 0.88  errorHandler (auth branch)           src/api/middleware/errors.ts:67  function
/// 0.85  TokenValidator.onExpired             src/auth/token.ts:112          method
/// ```
/// Plain-text view for `scope find <query>`.
pub struct FindResultsView<'a> {
    pub query: &'a str,
    pub results: &'a [SearchResult],
}

impl fmt::Display for FindResultsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Results for: \"{}\"", self.query)?;
        writeln!(f, "{SEPARATOR}")?;

        if self.results.is_empty() {
            writeln!(f, "(no results found)")?;
            return Ok(());
        }

        for result in self.results {
            let path = normalize_path(&result.file_path);
            let location = format!("{path}:{}", result.line_start);
            writeln!(
                f,
                "{:.2}  {:<40}{:<36}  {}",
                result.score, result.name, location, result.kind
            )?;
        }
        Ok(())
    }
}

/// Print index status.
///
/// Format:
/// ```text
/// Index status: up to date
///   Symbols:    6,764
///   Files:      847
///   Edges:      12,340
///   Last index: 2 minutes ago
/// ```
/// Plain-text view for `scope status`.
pub struct StatusView<'a> {
    pub status_label: &'a str,
    pub symbol_count: usize,
    pub file_count: usize,
    pub edge_count: usize,
    pub last_indexed: Option<&'a str>,
}

impl fmt::Display for StatusView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Index status: {}", self.status_label)?;
        writeln!(f, "  Symbols:    {}", format_number(self.symbol_count))?;
        writeln!(f, "  Files:      {}", format_number(self.file_count))?;
        writeln!(f, "  Edges:      {}", format_number(self.edge_count))?;
        match self.last_indexed {
            Some(relative) => writeln!(f, "  Last index: {relative}"),
            None => writeln!(f, "  Last index: never"),
        }
    }
}

/// Format a number with comma separators (e.g. 6764 -> "6,764").
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len <= 3 {
        return s;
    }

    let mut result = String::with_capacity(len + len / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result
}

/// Truncate a string to a maximum character width, adding "..." if truncated.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

/// Write multi-line snippet context with line numbers.
///
/// Marks the reference line with `>` and other lines with a space.
fn write_snippet_context(
    snippet: &[String],
    ref_line: Option<i64>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let Some(line_num) = ref_line else {
        return Ok(());
    };
    let ref_idx_in_snippet = snippet.len() / 2;
    let start_line = (line_num as usize).saturating_sub(ref_idx_in_snippet);

    for (i, code) in snippet.iter().enumerate() {
        let current_line = start_line + i;
        let marker = if current_line == line_num as usize {
            ">"
        } else {
            " "
        };
        writeln!(f, "  {marker} {current_line:>4} | {code}")?;
    }
    Ok(())
}

/// Print workspace member list in human-readable format.
///
/// Shows each member's name, path, index status, file count, and symbol count.
/// Plain-text view for `scope workspace list`.
pub struct WorkspaceListView<'a> {
    pub workspace_name: &'a str,
    pub members: &'a [crate::commands::workspace::MemberStatus],
}

impl fmt::Display for WorkspaceListView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Workspace: {}", self.workspace_name)?;
        writeln!(f, "{SEPARATOR}")?;

        if self.members.is_empty() {
            writeln!(f, "  (no members)")?;
            return Ok(());
        }

        let max_name = self
            .members
            .iter()
            .map(|m| m.name.len())
            .max()
            .unwrap_or(4)
            .max(4);
        let max_path = self
            .members
            .iter()
            .map(|m| m.path.len())
            .max()
            .unwrap_or(4)
            .max(4);

        writeln!(
            f,
            "  {:<name_w$}  {:<path_w$}  {:<15}  {:>5}  {:>7}",
            "Name",
            "Path",
            "Status",
            "Files",
            "Symbols",
            name_w = max_name,
            path_w = max_path,
        )?;

        for member in self.members {
            writeln!(
                f,
                "  {:<name_w$}  {:<path_w$}  {:<15}  {:>5}  {:>7}",
                member.name,
                normalize_path(&member.path),
                member.status,
                if member.file_count > 0 {
                    format_number(member.file_count)
                } else {
                    "─".to_string()
                },
                if member.symbol_count > 0 {
                    format_number(member.symbol_count)
                } else {
                    "─".to_string()
                },
                name_w = max_name,
                path_w = max_path,
            )?;
        }
        Ok(())
    }
}

/// Convert an edge kind string to a human-readable label for grouped output.
fn humanize_edge_kind(kind: &str) -> &str {
    match kind {
        "instantiates" => "instantiated",
        "extends" => "extended",
        "implements" => "implemented",
        "references_type" => "used as type",
        "imports" => "imported",
        "calls" => "called",
        "references" => "referenced",
        _ => kind,
    }
}

/// Print workspace status showing per-member status and aggregate totals.
/// Plain-text view for `scope status --workspace`.
pub struct WorkspaceStatusView<'a> {
    pub workspace_name: &'a str,
    pub members: &'a [crate::commands::status::MemberStatusData],
    pub total_symbols: usize,
    pub total_files: usize,
    pub total_edges: usize,
}

impl fmt::Display for WorkspaceStatusView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Workspace: {}", self.workspace_name)?;
        writeln!(f, "{SEPARATOR}")?;

        for m in self.members {
            let status_label = if m.status.index_exists {
                if m.status.symbol_count == 0 {
                    "empty"
                } else {
                    "indexed"
                }
            } else {
                "not indexed"
            };
            let last = m.status.last_indexed_relative.as_deref().unwrap_or("never");
            writeln!(
                f,
                "  {:<16}{:<14}{:>6} files  {:>7} symbols  {:>7} edges  {}",
                m.name,
                status_label,
                format_number(m.status.file_count),
                format_number(m.status.symbol_count),
                format_number(m.status.edge_count),
                last,
            )?;
        }

        writeln!(f, "{SEPARATOR}")?;
        writeln!(
            f,
            "  {:<16}{:<14}{:>6} files  {:>7} symbols  {:>7} edges",
            "Total",
            "",
            format_number(self.total_files),
            format_number(self.total_symbols),
            format_number(self.total_edges),
        )
    }
}

/// Print workspace refs: references tagged with project names.
/// Plain-text view for workspace-wide `scope refs --workspace <symbol>`.
pub struct WorkspaceRefsView<'a> {
    pub symbol_name: &'a str,
    pub refs: &'a [gumiho_mudang_scope::workspace_graph::WorkspaceRef],
    pub total: usize,
}

impl fmt::Display for WorkspaceRefsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} \u{2014} {} reference{} (workspace)",
            self.symbol_name,
            self.total,
            if self.total == 1 { "" } else { "s" }
        )?;
        writeln!(f, "{SEPARATOR}")?;

        for wr in self.refs {
            let r = &wr.reference;
            let path = normalize_path(&r.file_path);
            let location = if let Some(line) = r.line {
                format!("{path}:{line}")
            } else {
                path
            };
            let display_text = r.snippet_line.as_deref().unwrap_or(&r.context);
            let truncated_text = truncate_str(display_text.trim(), 70);
            writeln!(
                f,
                "[{:<12}] {:<36}{}",
                wr.project, location, truncated_text
            )?;
        }
        Ok(())
    }
}

/// Print workspace find results with project labels.
/// Plain-text view for workspace-wide `scope find --workspace <query>`.
pub struct WorkspaceFindResultsView<'a> {
    pub query: &'a str,
    pub results: &'a [crate::commands::find::WorkspaceSearchResult],
}

impl fmt::Display for WorkspaceFindResultsView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "find \"{}\" \u{2014} {} result{}",
            self.query,
            self.results.len(),
            if self.results.len() == 1 { "" } else { "s" }
        )?;
        writeln!(f, "{SEPARATOR}")?;

        for r in self.results {
            let path = normalize_path(&r.result.file_path);
            let line_range = format_line_range(r.result.line_start, r.result.line_end);
            writeln!(
                f,
                "[{:<12}] {:<32}{:<8}  {path}:{line_range}  ({:.2})",
                r.project, r.result.name, r.result.kind, r.result.score
            )?;
        }
        Ok(())
    }
}
