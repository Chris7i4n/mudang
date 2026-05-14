//! Structured verdict the model is expected to emit.
//!
//! [`Verdict`] is the wire shape of the JSON object the system prompt
//! commits the model to. Field names match `AUDIT-LABEL-SCHEMA.md`
//! labeller-fillable columns exactly so [`apply_to`] is a straight
//! field-for-field copy onto a [`SampleRecord`].
//!
//! [`apply_to`]: Verdict::apply_to

use scope_audit_labeller_core::SampleRecord;
use serde::Deserialize;

/// Parsed model verdict. All fields optional — a confident model fills
/// some; an abstaining model returns the all-null shape (`label = null`,
/// every proposed-* field null).
///
/// `evidence` is typed as `serde_json::Map<String, serde_json::Value>` to
/// match the v2 schema's `object | null` shape — the same type-level
/// enforcement that [`SampleRecord::evidence`] uses. A model that
/// returns `evidence: "free text"` is rejected at parse time, not
/// silently coerced.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct Verdict {
    #[serde(default)]
    pub label: Option<bool>,
    #[serde(default)]
    pub evidence: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub target_proposed: Option<String>,
    #[serde(default)]
    pub kind_proposed: Option<String>,
    #[serde(default)]
    pub confidence_proposed: Option<String>,
    #[serde(default)]
    pub reasoning_text: Option<String>,
    #[serde(default)]
    pub lang_version_evidence: Option<String>,
}

/// Errors encountered parsing the model's raw response into a [`Verdict`].
#[derive(Debug, thiserror::Error)]
pub enum VerdictParseError {
    #[error("model response is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("model response is not a JSON object")]
    NotAnObject,
}

impl Verdict {
    /// Parse the provider's raw response text. The response must be a
    /// single JSON object; any other shape (array, string, scalar) is
    /// rejected — that's a sign the model ignored the system prompt and
    /// emitting an abstain record is the right move upstream.
    pub fn parse_response(text: &str) -> Result<Self, VerdictParseError> {
        let value: serde_json::Value = serde_json::from_str(text.trim())?;
        if !value.is_object() {
            return Err(VerdictParseError::NotAnObject);
        }
        let verdict = serde_json::from_value(value)?;
        Ok(verdict)
    }

    /// Copy verdict fields onto the record, unconditionally. The seven
    /// labeller-fillable columns belong to the labeller named by the
    /// record's `labeller_id` — when the LLM stamps its id, it also
    /// owns those columns. A verdict field of `None` (either because
    /// the model omitted the key or because it returned an explicit
    /// `null`) clears any prior value; this prevents a downstream
    /// reader from seeing `labeller_id = llm:…` paired with a
    /// `label`/`target_proposed`/… written by a previous labeller in a
    /// composed or rerun pipeline. Aggregating partial verdicts across
    /// labellers is the aggregator's job (sprint 0006 (i)), not the
    /// per-labeller code's.
    pub fn apply_to(self, record: &mut SampleRecord) {
        record.label = self.label;
        record.evidence = self.evidence;
        record.target_proposed = self.target_proposed;
        record.kind_proposed = self.kind_proposed;
        record.confidence_proposed = self.confidence_proposed;
        record.reasoning_text = self.reasoning_text;
        record.lang_version_evidence = self.lang_version_evidence;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_verdict() {
        let text = r#"{
            "label": true,
            "evidence": {"reasoning": "exact match"},
            "target_proposed": null,
            "kind_proposed": null,
            "confidence_proposed": null,
            "reasoning_text": "extractor is correct",
            "lang_version_evidence": null
        }"#;
        let v = Verdict::parse_response(text).unwrap();
        assert_eq!(v.label, Some(true));
        assert_eq!(v.reasoning_text.as_deref(), Some("extractor is correct"));
        assert!(v.evidence.is_some());
    }

    #[test]
    fn parses_abstain_verdict() {
        let text = r#"{"label": null}"#;
        let v = Verdict::parse_response(text).unwrap();
        assert_eq!(v.label, None);
        assert!(v.evidence.is_none());
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        let text = "\n  {\"label\": false}  \n";
        let v = Verdict::parse_response(text).unwrap();
        assert_eq!(v.label, Some(false));
    }

    #[test]
    fn rejects_non_object_response() {
        let err = Verdict::parse_response("\"just a string\"").unwrap_err();
        assert!(matches!(err, VerdictParseError::NotAnObject));

        let err = Verdict::parse_response("[1, 2, 3]").unwrap_err();
        assert!(matches!(err, VerdictParseError::NotAnObject));
    }

    #[test]
    fn rejects_non_object_evidence() {
        let text = r#"{"evidence": "not an object"}"#;
        let err = Verdict::parse_response(text).unwrap_err();
        assert!(matches!(err, VerdictParseError::InvalidJson(_)));
    }

    #[test]
    fn apply_to_overwrites_every_labeller_fillable_field() {
        // Codex round 1 P2: verdict fields are owned by the labeller
        // whose `labeller_id` ends up on the record. An LLM abstain
        // (`label = None`) on a record pre-filled by another labeller
        // must clear that prior labeller's verdict, not silently inherit
        // it under the LLM's id. Aggregation across labellers is the
        // aggregator's responsibility (sprint 0006 (i)).
        let mut record = SampleRecord {
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
            label: Some(true),
            evidence: None,
            target_proposed: Some("prior".to_string()),
            kind_proposed: Some("prior-kind".to_string()),
            confidence_proposed: Some("high".to_string()),
            reasoning_text: Some("prior-reasoning".to_string()),
            lang_version_evidence: Some("2018".to_string()),
            labeller_id: Some("prior:labeller".to_string()),
        };
        // Verdict: LLM abstains, supplies one explicit reasoning.
        let v = Verdict {
            label: None,
            evidence: None,
            target_proposed: None,
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: Some("I cannot tell from this snippet".to_string()),
            lang_version_evidence: None,
        };
        v.apply_to(&mut record);
        // Every prior labeller-fillable field cleared / overwritten.
        assert_eq!(record.label, None);
        assert!(record.evidence.is_none());
        assert!(record.target_proposed.is_none());
        assert!(record.kind_proposed.is_none());
        assert!(record.confidence_proposed.is_none());
        assert_eq!(
            record.reasoning_text.as_deref(),
            Some("I cannot tell from this snippet")
        );
        assert!(record.lang_version_evidence.is_none());
        // `labeller_id` is set by the labeller, not `apply_to`; should
        // still hold the prior value until `LlmLabeller::label_one`
        // stamps it.
        assert_eq!(record.labeller_id.as_deref(), Some("prior:labeller"));
    }

    #[test]
    fn explicit_null_in_response_clears_prior_value() {
        // Round-trip the wire shape: model returns explicit nulls; the
        // resulting `Verdict` must clear, not preserve.
        let text = r#"{"label": null, "target_proposed": null, "reasoning_text": null}"#;
        let v = Verdict::parse_response(text).unwrap();
        let mut record = SampleRecord {
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
            label: Some(false),
            evidence: None,
            target_proposed: Some("prior".to_string()),
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: Some("prior".to_string()),
            lang_version_evidence: None,
            labeller_id: None,
        };
        v.apply_to(&mut record);
        assert_eq!(record.label, None);
        assert!(record.target_proposed.is_none());
        assert!(record.reasoning_text.is_none());
    }
}
