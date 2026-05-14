//! [`LlmLabeller`] — wraps any [`Provider`] into a [`Labeller`].
//!
//! Per-record flow:
//!
//! 1. Render the prompt from the record (`prompt::render_prompt`).
//! 2. Call the provider; on transport error after the provider's retry
//!    policy, write a stderr diagnostic and abstain (leave fields untouched
//!    except `labeller_id`). The record still flows downstream — an
//!    abstain is signal, not corruption.
//! 3. Parse the response into a [`Verdict`]; on parse error, same abstain.
//! 4. Copy verdict fields onto the record (`Verdict::apply_to`).
//! 5. Stamp `labeller_id` as `llm:<provider_id>:<model_id>`.
//!
//! [`Labeller::Error`] is [`std::convert::Infallible`]: there is no
//! per-record error path. Every record produces a valid output record;
//! the only thing that varies is whether the labeller-fillable fields
//! were populated or the record passed through as an abstain.

use std::convert::Infallible;
use std::io::Write;

use scope_audit_labeller_core::{Labeller, SampleRecord};

use crate::prompt::render_prompt;
use crate::provider::Provider;
use crate::verdict::Verdict;

/// LLM-backed labeller. Generic over the [`Provider`] transport so that
/// the test surface can substitute [`crate::mock::MockProvider`] without
/// touching HTTP.
pub struct LlmLabeller<P: Provider, W: Write = std::io::Stderr> {
    provider: P,
    labeller_id: String,
    diagnostics: W,
}

impl<P: Provider> LlmLabeller<P, std::io::Stderr> {
    /// Construct a labeller that writes per-record diagnostic lines to
    /// stderr. The `labeller_id` is computed once at construction time
    /// from `provider_id` + `model_id`.
    pub fn new(provider: P) -> Self {
        let labeller_id = format!("llm:{}:{}", provider.provider_id(), provider.model_id());
        Self {
            provider,
            labeller_id,
            diagnostics: std::io::stderr(),
        }
    }
}

impl<P: Provider, W: Write> LlmLabeller<P, W> {
    /// Construct with a custom diagnostic sink — used by tests that want
    /// to assert on the diagnostic output instead of letting it escape to
    /// the test runner's stderr.
    pub fn with_diagnostics(provider: P, diagnostics: W) -> Self {
        let labeller_id = format!("llm:{}:{}", provider.provider_id(), provider.model_id());
        Self {
            provider,
            labeller_id,
            diagnostics,
        }
    }
}

impl<P: Provider, W: Write> Labeller for LlmLabeller<P, W> {
    type Error = Infallible;

    fn label_one(&mut self, mut record: SampleRecord) -> Result<SampleRecord, Infallible> {
        let prompt = render_prompt(&record);
        let verdict = match self.provider.complete(&prompt) {
            Ok(response) => match Verdict::parse_response(&response.text) {
                Ok(v) => v,
                Err(err) => {
                    let _ = writeln!(
                        self.diagnostics,
                        "scope-audit-labeller-llm: verdict-parse error for edge {}: {err}",
                        record.edge_id,
                    );
                    Verdict::default()
                }
            },
            Err(err) => {
                let _ = writeln!(
                    self.diagnostics,
                    "scope-audit-labeller-llm: provider error for edge {}: {err}",
                    record.edge_id,
                );
                Verdict::default()
            }
        };
        // The LLM's `labeller_id` is about to be stamped, so the seven
        // labeller-fillable columns must reflect the LLM's verdict —
        // including the all-None abstain that error paths produce.
        // Without this clear, a record carrying a prior labeller's
        // verdict would emerge with the LLM's id attributed to the
        // stale fields. Codex round 2 P2.
        verdict.apply_to(&mut record);
        record.labeller_id = Some(self.labeller_id.clone());
        Ok(record)
    }

    fn labeller_id(&self) -> &str {
        &self.labeller_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockProvider, MockResponse};

    fn template_record() -> SampleRecord {
        SampleRecord {
            schema_version: "2".to_string(),
            edge_id: "e-1".to_string(),
            kind: "calls".to_string(),
            confidence: "medium".to_string(),
            producer: "rust".to_string(),
            pattern_id: "rust.calls.method".to_string(),
            from_id: "a".to_string(),
            to_id: "b".to_string(),
            source_snippet: "a()".to_string(),
            lang_version: Some("2021".to_string()),
            label: None,
            evidence: None,
            target_proposed: None,
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: None,
            lang_version_evidence: None,
            labeller_id: None,
        }
    }

    #[test]
    fn stamps_three_segment_labeller_id() {
        let provider = MockProvider::new("mock-vendor", "mock-model-v1");
        let labeller = LlmLabeller::new(provider);
        assert_eq!(labeller.labeller_id(), "llm:mock-vendor:mock-model-v1");
    }

    #[test]
    fn happy_path_fills_verdict_fields() {
        let provider = MockProvider::new("mock", "m1").with_response(MockResponse::ok(
            r#"{"label": true, "reasoning_text": "good"}"#,
        ));
        let mut labeller = LlmLabeller::with_diagnostics(provider, Vec::new());
        let out = labeller.label_one(template_record()).unwrap();
        assert_eq!(out.label, Some(true));
        assert_eq!(out.reasoning_text.as_deref(), Some("good"));
        assert_eq!(out.labeller_id.as_deref(), Some("llm:mock:m1"));
    }

    #[test]
    fn provider_error_yields_abstain_record() {
        let provider = MockProvider::new("mock", "m1").with_response(MockResponse::err("503 down"));
        let mut diagnostics = Vec::new();
        let mut labeller = LlmLabeller::with_diagnostics(provider, &mut diagnostics);
        let input = template_record();
        let out = labeller.label_one(input).unwrap();
        // labeller_id stamped; all seven labeller-fillable fields cleared.
        assert_eq!(out.labeller_id.as_deref(), Some("llm:mock:m1"));
        assert_eq!(out.label, None);
        assert!(out.evidence.is_none());
        assert!(out.target_proposed.is_none());
        assert!(out.kind_proposed.is_none());
        assert!(out.confidence_proposed.is_none());
        assert!(out.reasoning_text.is_none());
        assert!(out.lang_version_evidence.is_none());
        // Diagnostic line written.
        let diag = String::from_utf8(diagnostics).unwrap();
        assert!(diag.contains("provider error"));
        assert!(diag.contains("e-1"));
    }

    #[test]
    fn unparseable_response_yields_abstain_record() {
        let provider =
            MockProvider::new("mock", "m1").with_response(MockResponse::ok("not json at all"));
        let mut diagnostics = Vec::new();
        let mut labeller = LlmLabeller::with_diagnostics(provider, &mut diagnostics);
        let out = labeller.label_one(template_record()).unwrap();
        assert_eq!(out.labeller_id.as_deref(), Some("llm:mock:m1"));
        assert_eq!(out.label, None);
        let diag = String::from_utf8(diagnostics).unwrap();
        assert!(diag.contains("verdict-parse error"));
    }

    #[test]
    fn provider_error_on_prelabelled_record_clears_prior_verdict() {
        // Codex round 2 P2: a record pre-filled by a prior labeller
        // must not emerge with the prior verdict attributed to the LLM
        // when the LLM's transport fails.
        let provider = MockProvider::new("mock", "m1").with_response(MockResponse::err("timeout"));
        let mut labeller = LlmLabeller::with_diagnostics(provider, Vec::new());
        let mut input = template_record();
        input.label = Some(true);
        input.evidence = Some({
            let mut m = serde_json::Map::new();
            m.insert("prior".into(), serde_json::Value::String("evidence".into()));
            m
        });
        input.target_proposed = Some("prior-target".to_string());
        input.kind_proposed = Some("prior-kind".to_string());
        input.confidence_proposed = Some("high".to_string());
        input.reasoning_text = Some("prior reasoning".to_string());
        input.lang_version_evidence = Some("2018".to_string());
        input.labeller_id = Some("prior:labeller".to_string());
        let out = labeller.label_one(input).unwrap();
        assert_eq!(out.labeller_id.as_deref(), Some("llm:mock:m1"));
        assert_eq!(out.label, None);
        assert!(out.evidence.is_none());
        assert!(out.target_proposed.is_none());
        assert!(out.kind_proposed.is_none());
        assert!(out.confidence_proposed.is_none());
        assert!(out.reasoning_text.is_none());
        assert!(out.lang_version_evidence.is_none());
    }

    #[test]
    fn unparseable_response_on_prelabelled_record_clears_prior_verdict() {
        // Codex round 2 P2 (parse-error variant): a record pre-filled
        // by a prior labeller must not emerge with the prior verdict
        // attributed to the LLM when the model's reply fails to parse.
        let provider =
            MockProvider::new("mock", "m1").with_response(MockResponse::ok("definitely not json"));
        let mut labeller = LlmLabeller::with_diagnostics(provider, Vec::new());
        let mut input = template_record();
        input.label = Some(false);
        input.target_proposed = Some("prior-target".to_string());
        input.labeller_id = Some("prior:labeller".to_string());
        let out = labeller.label_one(input).unwrap();
        assert_eq!(out.labeller_id.as_deref(), Some("llm:mock:m1"));
        assert_eq!(out.label, None);
        assert!(out.target_proposed.is_none());
    }

    #[test]
    fn preserves_unrelated_fields() {
        let provider = MockProvider::new("mock", "m1").with_response(MockResponse::ok(
            r#"{"label": false, "target_proposed": "Baz::quux"}"#,
        ));
        let mut labeller = LlmLabeller::with_diagnostics(provider, Vec::new());
        let mut input = template_record();
        input.confidence = "low".to_string();
        input.source_snippet = "complicated()".to_string();
        let out = labeller.label_one(input.clone()).unwrap();
        assert_eq!(out.edge_id, input.edge_id);
        assert_eq!(out.kind, input.kind);
        assert_eq!(out.confidence, input.confidence);
        assert_eq!(out.source_snippet, input.source_snippet);
        assert_eq!(out.target_proposed.as_deref(), Some("Baz::quux"));
    }
}
