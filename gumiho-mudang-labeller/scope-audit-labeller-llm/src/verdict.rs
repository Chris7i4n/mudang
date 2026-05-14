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

    /// Copy verdict fields onto the record. Existing `label` is
    /// overwritten only when the verdict has one (`Some(_)`); absent
    /// verdict fields leave the record's prior value alone. This means
    /// pre-filled records (e.g. emitted by `scope audit confidence
    /// --emit-sample` with `label = null`) are augmented, not erased.
    pub fn apply_to(self, record: &mut SampleRecord) {
        if self.label.is_some() {
            record.label = self.label;
        }
        if self.evidence.is_some() {
            record.evidence = self.evidence;
        }
        if self.target_proposed.is_some() {
            record.target_proposed = self.target_proposed;
        }
        if self.kind_proposed.is_some() {
            record.kind_proposed = self.kind_proposed;
        }
        if self.confidence_proposed.is_some() {
            record.confidence_proposed = self.confidence_proposed;
        }
        if self.reasoning_text.is_some() {
            record.reasoning_text = self.reasoning_text;
        }
        if self.lang_version_evidence.is_some() {
            record.lang_version_evidence = self.lang_version_evidence;
        }
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
    fn apply_to_only_overwrites_set_fields() {
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
            kind_proposed: None,
            confidence_proposed: None,
            reasoning_text: None,
            lang_version_evidence: None,
            labeller_id: None,
        };
        let v = Verdict {
            label: None,           // verdict abstains → preserve record.label
            target_proposed: None, // preserve record.target_proposed
            reasoning_text: Some("set me".to_string()),
            ..Default::default()
        };
        v.apply_to(&mut record);
        assert_eq!(record.label, Some(true)); // preserved
        assert_eq!(record.target_proposed.as_deref(), Some("prior")); // preserved
        assert_eq!(record.reasoning_text.as_deref(), Some("set me")); // set
    }
}
