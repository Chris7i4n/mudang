//! Language support.
//!
//! Per R7 (A.4): there is no `LanguagePlugin` trait and there are no
//! `*Plugin` unit structs. Every per-language decision is an exhaustive
//! match arm on [`LanguageId`] in [`id`]. To add a language, add the
//! variant; the compiler lists every site that must be updated. There is
//! no other registration step beyond appending it to
//! `register_languages!` in [`dispatch`].
//!
//! The per-language modules in this directory own the **capture** side:
//! `extract_metadata`, `extract_docstring` (for languages that override
//! the default). The **edge** side (per-`EdgeKind` decisions, `RawEdge`
//! construction) lives in [`crate::extract`] per R2 — sprint 0003 chunk 2
//! moved the `extract_*_edge` free functions and the `make_edge` /
//! `resolve_scope_id` helpers there. Per-language modules have no `impl`
//! blocks beyond on their own data structs.

pub mod csharp;
pub mod dispatch;
pub mod go_lang;
pub mod id;
pub mod java;
pub mod python;
pub mod ruby;
pub mod rust_lang;
pub mod typescript;

pub use id::LanguageId;
