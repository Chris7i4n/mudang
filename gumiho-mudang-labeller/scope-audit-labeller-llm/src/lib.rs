//! LLM-backed Scope audit labeller.
//!
//! Wraps any [`Provider`] (transport to a chat-completions API) and turns
//! it into a [`Labeller`] that fills the seven labeller-fillable v2 fields
//! from the model's response. The trait split — transport on one side,
//! per-record labelling on the other — is the seam that keeps the
//! labeller provider-agnostic and the test surface mockable without an
//! HTTP fake.
//!
//! See `AUDIT-LABEL-SCHEMA.md` § Record schema for the wire contract and
//! `gumiho-mudang-scope/docs/SELF-CORRECTION-CYCLE.md` § Labeller workspace
//! for the role this crate plays in the loop.

pub mod labeller;
pub mod mock;
pub mod prompt;
pub mod provider;
pub mod verdict;

pub mod providers;

pub use labeller::LlmLabeller;
pub use mock::MockProvider;
pub use prompt::{render_prompt, Prompt};
pub use provider::{Provider, ProviderResponse};
pub use verdict::Verdict;
