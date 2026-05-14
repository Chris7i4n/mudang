//! Concrete [`Provider`] implementations, each behind its own cargo
//! feature so an installation only pulls the SDK / HTTP stack for the
//! providers the operator opts in to.
//!
//! Current providers:
//!
//! - [`deepseek`] — DeepSeek chat-completions (OpenAI-compatible).
//!   Feature: `deepseek`.
//!
//! Future providers (sprints beyond 0010): `anthropic`, `openai`,
//! `gemini`, `local` (e.g. llama.cpp / ollama). Each lands as its own
//! cargo feature; existing builds are unaffected.
//!
//! [`Provider`]: crate::provider::Provider

#[cfg(feature = "deepseek")]
pub mod deepseek;
