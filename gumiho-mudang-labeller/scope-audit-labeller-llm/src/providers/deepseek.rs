//! DeepSeek chat-completions [`Provider`].
//!
//! DeepSeek exposes an OpenAI-compatible endpoint at
//! `https://api.deepseek.com/chat/completions`. Authentication is a
//! bearer token (the user's `DEEPSEEK_API_KEY`). The provider asks for
//! `response_format = { type: "json_object" }` so the model's reply is
//! direct JSON — no markdown fence stripping needed on our side.
//!
//! Transport: synchronous `ureq`. Retry policy: bounded exponential
//! backoff on 429 / 5xx / network errors; after the bound, the provider
//! surfaces a [`DeepSeekError`] which [`LlmLabeller`] catches and turns
//! into an abstain record.
//!
//! [`LlmLabeller`]: crate::labeller::LlmLabeller

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::prompt::Prompt;
use crate::provider::{Provider, ProviderResponse};

/// Default chat-completions endpoint. Override via
/// [`DeepSeekProvider::with_endpoint`] for the live test feature.
pub const DEFAULT_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

/// Default model. DeepSeek's general-purpose chat model.
pub const DEFAULT_MODEL: &str = "deepseek-chat";

/// Provider identifier — middle segment of `labeller_id`.
pub const PROVIDER_ID: &str = "deepseek";

/// DeepSeek chat-completions provider.
#[derive(Debug, Clone)]
pub struct DeepSeekProvider {
    api_key: String,
    model: String,
    endpoint: String,
    max_retries: u32,
    base_backoff: Duration,
    request_timeout: Duration,
}

impl DeepSeekProvider {
    /// Construct with the user's API key. Defaults to model
    /// [`DEFAULT_MODEL`] and endpoint [`DEFAULT_ENDPOINT`].
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            endpoint: DEFAULT_ENDPOINT.to_string(),
            max_retries: 4,
            base_backoff: Duration::from_millis(500),
            request_timeout: Duration::from_secs(60),
        }
    }

    /// Construct from the `DEEPSEEK_API_KEY` env var; returns
    /// [`DeepSeekError::MissingApiKey`] if unset.
    pub fn from_env() -> Result<Self, DeepSeekError> {
        let key = std::env::var("DEEPSEEK_API_KEY").map_err(|_| DeepSeekError::MissingApiKey)?;
        Ok(Self::new(key))
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_base_backoff(mut self, base_backoff: Duration) -> Self {
        self.base_backoff = base_backoff;
        self
    }
}

#[derive(Debug, Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct Request<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    response_format: ResponseFormat,
    temperature: f32,
}

/// Errors the DeepSeek provider can surface. The labeller catches these
/// and emits an abstain record; nothing else in the crate inspects the
/// variants — they exist for the diagnostic stderr line.
#[derive(Debug, thiserror::Error)]
pub enum DeepSeekError {
    #[error("DEEPSEEK_API_KEY env var unset")]
    MissingApiKey,

    #[error("HTTP transport error after {attempts} attempts: {source}")]
    Transport {
        attempts: u32,
        // Boxed to keep `DeepSeekError`'s size off the `Result`-large-err
        // clippy threshold; `ureq::Error` carries an embedded response
        // buffer and bulks the variant past 250 bytes on its own.
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("DeepSeek returned status {status} after {attempts} attempts: {body}")]
    Status {
        status: u16,
        attempts: u32,
        body: String,
    },

    #[error("response body read error: {0}")]
    BodyRead(#[from] std::io::Error),

    #[error("response JSON did not contain choices[0].message.content")]
    NoContent,

    #[error("response JSON failed to parse: {0}")]
    InvalidResponseJson(#[from] serde_json::Error),
}

impl Provider for DeepSeekProvider {
    type Error = DeepSeekError;

    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    fn complete(&self, prompt: &Prompt) -> Result<ProviderResponse, Self::Error> {
        let started = Instant::now();
        let body = Request {
            model: &self.model,
            messages: vec![
                Message {
                    role: "system",
                    content: &prompt.system,
                },
                Message {
                    role: "user",
                    content: &prompt.user,
                },
            ],
            response_format: ResponseFormat {
                kind: "json_object",
            },
            temperature: 0.0,
        };

        let mut attempts: u32 = 0;
        loop {
            attempts += 1;
            let agent = ureq::AgentBuilder::new()
                .timeout(self.request_timeout)
                .build();
            let bearer = format!("Bearer {}", self.api_key);
            let result = agent
                .post(&self.endpoint)
                .set("Authorization", &bearer)
                .set("Content-Type", "application/json")
                .send_json(serde_json::to_value(&body)?);

            match result {
                Ok(response) => {
                    let text = response.into_string()?;
                    let parsed: serde_json::Value = serde_json::from_str(&text)?;
                    let content = parsed
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .ok_or(DeepSeekError::NoContent)?;
                    return Ok(ProviderResponse {
                        text: content.to_string(),
                        latency_ms: started.elapsed().as_millis() as u64,
                    });
                }
                Err(ureq::Error::Status(status, response)) => {
                    let retriable = status == 429 || (500..600).contains(&status);
                    if !retriable || attempts > self.max_retries {
                        let body = response
                            .into_string()
                            .unwrap_or_else(|_| "<unreadable body>".to_string());
                        return Err(DeepSeekError::Status {
                            status,
                            attempts,
                            body,
                        });
                    }
                }
                Err(other) => {
                    if attempts > self.max_retries {
                        return Err(DeepSeekError::Transport {
                            attempts,
                            source: Box::new(other),
                        });
                    }
                }
            }

            let backoff = self.base_backoff * (1 << (attempts - 1).min(6));
            std::thread::sleep(backoff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_is_constant() {
        let p = DeepSeekProvider::new("k");
        assert_eq!(p.provider_id(), "deepseek");
    }

    #[test]
    fn default_model_is_deepseek_chat() {
        let p = DeepSeekProvider::new("k");
        assert_eq!(p.model_id(), "deepseek-chat");
    }

    #[test]
    fn with_model_overrides() {
        let p = DeepSeekProvider::new("k").with_model("deepseek-reasoner");
        assert_eq!(p.model_id(), "deepseek-reasoner");
    }

    #[test]
    fn from_env_errors_without_key() {
        // Ensure variable is unset for this assertion. The test process
        // does not normally have DEEPSEEK_API_KEY in scope; if it does
        // (live-test environment), we skip.
        if std::env::var("DEEPSEEK_API_KEY").is_ok() {
            return;
        }
        let err = DeepSeekProvider::from_env().unwrap_err();
        assert!(matches!(err, DeepSeekError::MissingApiKey));
    }
}
