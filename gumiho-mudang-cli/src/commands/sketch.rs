/// `scope sketch <symbol>` — show structural overview of a symbol.
///
/// Returns the class/function signature, dependencies, methods with caller counts,
/// and type information. Use this before `scope source` to understand structure first.
///
/// Examples:
///   scope sketch PaymentService              — sketch a class
///   scope sketch PaymentService.processPayment  — sketch a method
///   scope sketch src/payments/service.ts     — sketch a whole file
use anyhow::{bail, Result};
use clap::Args;
use std::path::Path;

use crate::commands::warn_if_stale;
use crate::output::formatter;
use crate::output::json::JsonOutput;
use crate::output::schema::{
    ClassSketch, EnumSketch, EnumVariantView, FieldView, FileSketch, GenericSketch,
    InterfaceSketch, MethodSketch, SketchSymbol, SymbolSketch,
};
use gumiho_mudang_scope::graph::{Graph, Symbol};

/// Arguments for the `scope sketch` command.
#[derive(Args, Debug)]
pub struct SketchArgs {
    /// Symbol name or file path to sketch.
    ///
    /// Pass a class name to see its methods, deps, and inheritance.
    /// Pass a method name to see its signature, callers, and callees.
    /// Pass Class.method for qualified lookup.
    /// Pass a file path to see all symbols in that file.
    ///
    /// Examples: PaymentService, PaymentService.processPayment, src/payments/service.ts
    pub symbol: String,

    /// Output as JSON instead of human-readable format
    #[arg(long, short = 'j')]
    pub json: bool,

    /// Maximum number of methods to show (default: all)
    #[arg(long, default_value = "50")]
    pub limit: usize,

    /// Suppress docstring display in sketch output
    #[arg(long)]
    pub no_docs: bool,

    /// Treat the argument as a file path (sketch all symbols in the file).
    ///
    /// Useful when the path doesn't contain `/` and would otherwise be
    /// treated as a symbol name.
    #[arg(long)]
    pub file: bool,

    /// Emit compact JSON (strips internal IDs, raw metadata, language).
    ///
    /// Reduces token cost by ~70% for LLM agents that only need
    /// name, kind, signature, and line numbers. Implies --json.
    #[arg(long)]
    pub compact: bool,
}

/// Returns true if the input looks like a file path rather than a symbol name.
use super::looks_like_file_path;

/// Run the `scope sketch` command.
pub fn run(args: &SketchArgs, project_root: &Path) -> Result<()> {
    let scope_dir = project_root.join(".scope");

    if !scope_dir.exists() {
        bail!("No .scope/ directory found. Run 'scope init' first.");
    }

    let db_path = scope_dir.join("graph.db");
    if !db_path.exists() {
        bail!("No index found. Run 'scope index' to build one first.");
    }

    let graph = Graph::open(&db_path)?;
    warn_if_stale(&graph, project_root);

    if args.file || looks_like_file_path(&args.symbol) {
        return run_file_sketch(args, &graph);
    }

    run_symbol_sketch(args, &graph)
}

/// Emit a `JsonOutput<SymbolSketch>` to stdout.
fn emit_json(
    symbol_name: String,
    data: SymbolSketch<'_>,
    truncated: bool,
    total: usize,
) -> Result<()> {
    let output = JsonOutput {
        command: "sketch",
        symbol: Some(symbol_name),
        data,
        truncated,
        total,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Sketch a single symbol (class, method, interface, etc.).
fn run_symbol_sketch(args: &SketchArgs, graph: &Graph) -> Result<()> {
    let symbol = graph.find_symbol(&args.symbol)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Symbol '{}' not found in index.\n\
             Tip: Check spelling, or use 'scope find \"{}\"' for semantic search.",
            args.symbol,
            args.symbol
        )
    })?;

    match symbol.kind.as_str() {
        "class" | "struct" => sketch_class(args, graph, &symbol),
        "method" | "function" => sketch_method(args, graph, &symbol),
        "interface" => sketch_interface(args, graph, &symbol),
        "enum" => sketch_enum(args, graph, &symbol),
        _ => sketch_generic(args, &symbol),
    }
}

/// Sketch a class or struct.
fn sketch_class(args: &SketchArgs, graph: &Graph, symbol: &Symbol) -> Result<()> {
    let methods = graph.get_methods(&symbol.id)?;
    let relationships = graph.get_class_relationships(&symbol.id)?;

    let method_ids: Vec<&str> = methods.iter().map(|m| m.id.as_str()).collect();
    let caller_counts = graph.get_caller_counts(&method_ids)?;

    if args.json || args.compact {
        let (field_syms, non_field_syms): (Vec<&Symbol>, Vec<&Symbol>) =
            methods.iter().partition(|m| m.kind == "property");
        let total = non_field_syms.len();
        let truncated = total > args.limit;

        let methods_view: Vec<SketchSymbol<'_>> = non_field_syms
            .into_iter()
            .take(args.limit)
            .map(|m| SketchSymbol::pick(m, args.compact))
            .collect();
        let fields: Vec<FieldView<'_>> = field_syms.into_iter().map(FieldView::from).collect();

        let sketch = ClassSketch {
            symbol: SketchSymbol::pick(symbol, args.compact),
            methods: methods_view,
            fields,
            caller_counts,
            relationships: &relationships,
        };

        emit_json(symbol.name.clone(), SymbolSketch::Class(sketch), truncated, total)?;
    } else {
        print!(
            "{}",
            formatter::ClassSketchView {
                symbol,
                methods: &methods,
                caller_counts: &caller_counts,
                relationships: &relationships,
                limit: args.limit,
                show_docs: !args.no_docs,
            }
        );
    }

    Ok(())
}

/// Sketch a method or function.
fn sketch_method(args: &SketchArgs, graph: &Graph, symbol: &Symbol) -> Result<()> {
    let outgoing_calls = graph.get_outgoing_calls(&symbol.id)?;
    let incoming_callers = graph.get_incoming_callers(&symbol.id)?;

    if args.json || args.compact {
        let sketch = MethodSketch {
            symbol: SketchSymbol::pick(symbol, args.compact),
            calls: &outgoing_calls,
            called_by: &incoming_callers,
        };
        emit_json(symbol.name.clone(), SymbolSketch::Method(sketch), false, 1)?;
    } else {
        print!(
            "{}",
            formatter::MethodSketchView {
                symbol,
                outgoing_calls: &outgoing_calls,
                incoming_callers: &incoming_callers,
            }
        );
    }

    Ok(())
}

/// Sketch an interface.
fn sketch_interface(args: &SketchArgs, graph: &Graph, symbol: &Symbol) -> Result<()> {
    let methods = graph.get_methods(&symbol.id)?;
    let implementors = graph.get_implementors(&symbol.id)?;
    let relationships = graph.get_class_relationships(&symbol.id)?;

    if args.json || args.compact {
        let total = methods.len();
        let truncated = total > args.limit;
        let methods_view: Vec<SketchSymbol<'_>> = methods
            .iter()
            .take(args.limit)
            .map(|m| SketchSymbol::pick(m, args.compact))
            .collect();

        let sketch = InterfaceSketch {
            symbol: SketchSymbol::pick(symbol, args.compact),
            methods: methods_view,
            implementors: &implementors,
            relationships: &relationships,
        };

        emit_json(
            symbol.name.clone(),
            SymbolSketch::Interface(sketch),
            truncated,
            total,
        )?;
    } else {
        print!(
            "{}",
            formatter::InterfaceSketchView {
                symbol,
                methods: &methods,
                implementors: &implementors,
                relationships: &relationships,
                limit: args.limit,
            }
        );
    }

    Ok(())
}

/// Sketch an enum — shows variants and caller count.
fn sketch_enum(args: &SketchArgs, graph: &Graph, symbol: &Symbol) -> Result<()> {
    let children = graph.get_methods(&symbol.id)?;
    let variants: Vec<&Symbol> = children.iter().filter(|c| c.kind == "variant").collect();
    let caller_count = graph.get_caller_count(&symbol.id)?;

    if args.json || args.compact {
        let variant_views: Vec<EnumVariantView<'_>> =
            variants.iter().map(|v| EnumVariantView::from(*v)).collect();

        let sketch = EnumSketch {
            symbol: SketchSymbol::pick(symbol, args.compact),
            variants: variant_views,
            caller_count,
        };

        emit_json(symbol.name.clone(), SymbolSketch::Enum(sketch), false, 1)?;
    } else {
        print!(
            "{}",
            formatter::EnumSketchView {
                symbol,
                variants: &variants,
                caller_count,
            }
        );
    }

    Ok(())
}

/// Sketch a generic symbol (const, type).
fn sketch_generic(args: &SketchArgs, symbol: &Symbol) -> Result<()> {
    if args.json || args.compact {
        let sketch = GenericSketch {
            symbol: SketchSymbol::pick(symbol, args.compact),
        };
        emit_json(symbol.name.clone(), SymbolSketch::Generic(sketch), false, 1)?;
    } else {
        print!("{}", formatter::GenericSketchView { symbol });
    }

    Ok(())
}

/// Sketch all symbols in a file.
fn run_file_sketch(args: &SketchArgs, graph: &Graph) -> Result<()> {
    let file_path = formatter::normalize_path(&args.symbol);
    let symbols = graph.get_file_symbols(&file_path)?;

    if symbols.is_empty() {
        bail!(
            "No symbols found for file '{}'.\n\
             Tip: Check the path is relative to the project root. Run 'scope index' if the file is new.",
            file_path
        );
    }

    let symbol_ids: Vec<&str> = symbols.iter().map(|s| s.id.as_str()).collect();
    let caller_counts = graph.get_caller_counts(&symbol_ids)?;

    if args.json || args.compact {
        let total = symbols.len();
        let symbols_view: Vec<SketchSymbol<'_>> = symbols
            .iter()
            .map(|s| SketchSymbol::pick(s, args.compact))
            .collect();

        let sketch = FileSketch {
            file_path: &file_path,
            symbols: symbols_view,
            caller_counts,
        };

        emit_json(file_path.clone(), SymbolSketch::File(sketch), false, total)?;
    } else {
        print!(
            "{}",
            formatter::FileSketchView {
                file_path: &file_path,
                symbols: &symbols,
                caller_counts: &caller_counts,
            }
        );
    }

    Ok(())
}
