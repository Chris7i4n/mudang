//! In-process [`Provider`] mock for tests.
//!
//! Lives in the crate's public surface (rather than `cfg(test)`-only) so
//! downstream integration tests can construct an [`LlmLabeller`] backed
//! by canned responses without an HTTP fake. The mock has its own
//! `provider_id` / `model_id` strings so `labeller_id` assertions can
//! still verify the three-segment shape.

use std::cell::RefCell;

use crate::prompt::Prompt;
use crate::provider::{Provider, ProviderResponse};

/// Canned response for one call to the mock.
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Successful provider response with the given raw text.
    Ok(String),
    /// Transport error with the given message — surfaces as
    /// [`MockProviderError`] from the provider trait.
    Err(String),
}

impl MockResponse {
    pub fn ok(text: impl Into<String>) -> Self {
        Self::Ok(text.into())
    }
    pub fn err(msg: impl Into<String>) -> Self {
        Self::Err(msg.into())
    }
}

/// [`Provider`] impl that returns canned responses in order.
///
/// Construct with `MockProvider::new(provider_id, model_id)` then add
/// canned responses via [`MockProvider::with_response`]. Responses are
/// consumed in order; running out of canned responses on the next call
/// returns an error.
#[derive(Debug)]
pub struct MockProvider {
    provider_id: String,
    model_id: String,
    responses: RefCell<Vec<MockResponse>>,
    calls: RefCell<usize>,
}

impl MockProvider {
    pub fn new(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            responses: RefCell::new(Vec::new()),
            calls: RefCell::new(0),
        }
    }

    /// Queue one canned response. Builder-style for one-liner test setup.
    pub fn with_response(self, response: MockResponse) -> Self {
        self.responses.borrow_mut().push(response);
        self
    }

    /// Queue many canned responses at once.
    pub fn with_responses(self, responses: impl IntoIterator<Item = MockResponse>) -> Self {
        self.responses.borrow_mut().extend(responses);
        self
    }

    /// Number of times [`Provider::complete`] has been called. Useful for
    /// assertions on call counts in tests.
    pub fn call_count(&self) -> usize {
        *self.calls.borrow()
    }
}

/// Error surfaced by [`MockProvider`].
#[derive(Debug, thiserror::Error)]
pub enum MockProviderError {
    #[error("mock transport error: {0}")]
    Transport(String),

    #[error("mock provider has no canned response left")]
    Exhausted,
}

impl Provider for MockProvider {
    type Error = MockProviderError;

    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn complete(&self, _prompt: &Prompt) -> Result<ProviderResponse, Self::Error> {
        let mut responses = self.responses.borrow_mut();
        let mut calls = self.calls.borrow_mut();
        *calls += 1;
        if responses.is_empty() {
            return Err(MockProviderError::Exhausted);
        }
        match responses.remove(0) {
            MockResponse::Ok(text) => Ok(ProviderResponse {
                text,
                latency_ms: 0,
            }),
            MockResponse::Err(msg) => Err(MockProviderError::Transport(msg)),
        }
    }
}
