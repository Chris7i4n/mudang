//! `SymbolSketch<'a>` — sum type over the six `scope sketch` outputs.
//!
//! `scope sketch <symbol>` dispatches on symbol kind:
//! - class / struct → [`ClassSketch`]
//! - method / function → [`MethodSketch`]
//! - interface → [`InterfaceSketch`]
//! - enum → [`EnumSketch`]
//! - const / type / other → [`GenericSketch`]
//! - file path → [`FileSketch`]
//!
//! Each variant carries exactly the fields its renderer needs. The
//! sum-type shape makes it a compile-time error to construct a class
//! sketch with method-only fields (R10's "make illegal states
//! unrepresentable").
//!
//! The `SymbolSketch` enum serializes with `serde(tag = "sketch_kind")`
//! so JSON consumers can discriminate without parsing nested fields.

use std::collections::HashMap;

use gumiho_mudang_scope::graph::{CallerInfo, ClassRelationships, Symbol};
use serde::Serialize;

use super::CompactSymbol;

/// The whole-symbol view: either the full `Symbol` or a borrowed
/// `CompactSymbol`. `--compact` selects the latter; default JSON
/// selects the former.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SketchSymbol<'a> {
    /// Full symbol (default JSON shape).
    Full(&'a Symbol),
    /// Compact projection (--compact JSON shape).
    Compact(CompactSymbol<'a>),
}

impl<'a> SketchSymbol<'a> {
    /// Construct a `SketchSymbol::Compact` from a borrowed `Symbol`.
    pub fn compact(symbol: &'a Symbol) -> Self {
        Self::Compact(CompactSymbol::from(symbol))
    }

    /// Construct a `SketchSymbol::Full` from a borrowed `Symbol`.
    pub fn full(symbol: &'a Symbol) -> Self {
        Self::Full(symbol)
    }

    /// Pick `Full` or `Compact` based on the `--compact` flag.
    pub fn pick(symbol: &'a Symbol, compact: bool) -> Self {
        if compact {
            Self::compact(symbol)
        } else {
            Self::full(symbol)
        }
    }
}

/// A field view — used inside `ClassSketch`.
#[derive(Debug, Clone, Serialize)]
pub struct FieldView<'a> {
    /// Field name.
    pub name: &'a str,
    /// Type signature where available. Serialises as JSON `null` when
    /// absent (pre-R10 wire-shape compat; codex P1, sprint 0006).
    pub signature: Option<&'a str>,
    /// First line of the field declaration.
    pub line_start: u32,
}

impl<'a> From<&'a Symbol> for FieldView<'a> {
    fn from(symbol: &'a Symbol) -> Self {
        Self {
            name: &symbol.name,
            signature: symbol.signature.as_deref(),
            line_start: symbol.line_start,
        }
    }
}

/// An enum variant view — used inside `EnumSketch`.
#[derive(Debug, Clone, Serialize)]
pub struct EnumVariantView<'a> {
    /// Variant name.
    pub name: &'a str,
    /// Variant signature (parameters, payload) where available.
    /// Serialises as JSON `null` when absent (pre-R10 wire-shape compat;
    /// codex P1, sprint 0006).
    pub signature: Option<&'a str>,
    /// First line of the variant.
    pub line_start: u32,
    /// Last line of the variant.
    pub line_end: u32,
}

impl<'a> From<&'a Symbol> for EnumVariantView<'a> {
    fn from(symbol: &'a Symbol) -> Self {
        Self {
            name: &symbol.name,
            signature: symbol.signature.as_deref(),
            line_start: symbol.line_start,
            line_end: symbol.line_end,
        }
    }
}

/// Class / struct sketch — symbol + methods + fields + relationships.
#[derive(Debug, Clone, Serialize)]
pub struct ClassSketch<'a> {
    pub symbol: SketchSymbol<'a>,
    pub methods: Vec<SketchSymbol<'a>>,
    pub fields: Vec<FieldView<'a>>,
    pub caller_counts: HashMap<String, usize>,
    pub relationships: &'a ClassRelationships,
}

/// Method / function sketch — symbol + outgoing calls + incoming callers.
///
/// `calls` is a list of callee names (`graph.get_outgoing_calls`
/// returns symbol-name strings; resolver-tier symbol structs are not
/// materialised here).
#[derive(Debug, Clone, Serialize)]
pub struct MethodSketch<'a> {
    pub symbol: SketchSymbol<'a>,
    pub calls: &'a [String],
    pub called_by: &'a [CallerInfo],
}

/// Interface sketch — symbol + methods + implementors + relationships.
///
/// `implementors` is a list of implementor symbol-name strings
/// (`graph.get_implementors` shape).
#[derive(Debug, Clone, Serialize)]
pub struct InterfaceSketch<'a> {
    pub symbol: SketchSymbol<'a>,
    pub methods: Vec<SketchSymbol<'a>>,
    pub implementors: &'a [String],
    pub relationships: &'a ClassRelationships,
}

/// Enum sketch — symbol + variants + caller count.
#[derive(Debug, Clone, Serialize)]
pub struct EnumSketch<'a> {
    pub symbol: SketchSymbol<'a>,
    pub variants: Vec<EnumVariantView<'a>>,
    pub caller_count: usize,
}

/// Generic sketch — symbol only (consts, types, anything kind-agnostic).
#[derive(Debug, Clone, Serialize)]
pub struct GenericSketch<'a> {
    pub symbol: SketchSymbol<'a>,
}

/// File sketch — every symbol in a file.
#[derive(Debug, Clone, Serialize)]
pub struct FileSketch<'a> {
    pub file_path: &'a str,
    pub symbols: Vec<SketchSymbol<'a>>,
    pub caller_counts: HashMap<String, usize>,
}

/// `scope sketch` output — sum type over the six variants. JSON
/// consumers distinguish variants via the `sketch_kind` tag.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "sketch_kind", rename_all = "lowercase")]
pub enum SymbolSketch<'a> {
    Class(ClassSketch<'a>),
    Method(MethodSketch<'a>),
    Interface(InterfaceSketch<'a>),
    Enum(EnumSketch<'a>),
    Generic(GenericSketch<'a>),
    File(FileSketch<'a>),
}
