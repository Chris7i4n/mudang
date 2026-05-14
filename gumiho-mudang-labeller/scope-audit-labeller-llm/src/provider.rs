//! Transport trait every concrete LLM provider implements.
//!
//! A provider is the thin layer that turns a rendered [`Prompt`] into a
//! raw model response. Parsing the response into a [`Verdict`] and mapping
//! the verdict onto [`SampleRecord`] fields lives one layer up in
//! [`LlmLabeller`] — providers should not know about the v2 schema.
//!
//! [`Verdict`]: crate::verdict::Verdict
//! [`Prompt`]: crate::prompt::Prompt
//! [`SampleRecord`]: scope_audit_labeller_core::SampleRecord
//! [`LlmLabeller`]: crate::labeller::LlmLabeller

use crate::prompt::Prompt;

/// Raw response from a [`Provider::complete`] call.
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// Raw model output. Conventionally a JSON object the verdict parser
    /// can deserialise; the provider does not interpret it.
    pub text: String,
    /// Wall-clock latency of the underlying transport call, including
    /// retry backoff. Surfaced for the operator's diagnostics.
    pub latency_ms: u64,
}

/// One LLM transport. Implementors are typically a thin HTTP client over a
/// chat-completions API. Retry / rate-limit handling is the provider's
/// responsibility: by the time `complete` returns, transient failures
/// must already have been retried within the provider's bounded policy.
pub trait Provider {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Stable provider identifier — used as the middle segment of
    /// `labeller_id` (`llm:<provider_id>:<model_id>`). Conventionally
    /// the host or vendor name in lowercase (`"deepseek"`,
    /// `"anthropic"`, `"openai"`).
    fn provider_id(&self) -> &str;

    /// Model identifier this provider routes calls to (e.g.
    /// `"deepseek-chat"`). Used as the trailing segment of `labeller_id`.
    fn model_id(&self) -> &str;

    /// Send the prompt to the model and return the raw response.
    fn complete(&self, prompt: &Prompt) -> Result<ProviderResponse, Self::Error>;
}
