//! Reference no-op labeller.
//!
//! Implements [`Labeller`] by passing every record through unchanged and
//! stamping the canonical `labeller_id`. Proves the
//! `scope-audit-labeller-core` trait + JSONL IO loop end-to-end before
//! the concrete labellers (LLM / LSP / hybrid) land in sprints 0010-0012.
//!
//! Stamping `labeller_id` is a write the trait commits to (every labeller
//! must identify itself); the seven other labeller-fillable fields
//! (`evidence`, `target_proposed`, `kind_proposed`, `confidence_proposed`,
//! `reasoning_text`, `lang_version_evidence`, plus the verdict `label`)
//! are passed through verbatim. The noop has no opinion to overwrite.

use std::convert::Infallible;

use scope_audit_labeller_core::{Labeller, SampleRecord};

/// The `labeller_id` this labeller writes into every record.
pub const NOOP_LABELLER_ID: &str = "noop:reference-v0";

/// Stateless reference labeller. Construct with `NoopLabeller::default()`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopLabeller;

impl Labeller for NoopLabeller {
    type Error = Infallible;

    fn label_one(&mut self, mut record: SampleRecord) -> Result<SampleRecord, Infallible> {
        record.labeller_id = Some(NOOP_LABELLER_ID.to_string());
        Ok(record)
    }

    fn labeller_id(&self) -> &str {
        NOOP_LABELLER_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_record() -> SampleRecord {
        SampleRecord {
            schema_version: "2".to_string(),
            edge_id: "e-test".to_string(),
            kind: "calls".to_string(),
            confidence: "high".to_string(),
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
    fn stamps_canonical_labeller_id() {
        let mut labeller = NoopLabeller;
        let out = labeller.label_one(template_record()).unwrap();
        assert_eq!(out.labeller_id.as_deref(), Some(NOOP_LABELLER_ID));
    }

    #[test]
    fn preserves_other_fields() {
        let mut labeller = NoopLabeller;
        let input = template_record();
        let out = labeller.label_one(input.clone()).unwrap();
        // Every field except labeller_id must match the input.
        assert_eq!(out.schema_version, input.schema_version);
        assert_eq!(out.edge_id, input.edge_id);
        assert_eq!(out.from_id, input.from_id);
        assert_eq!(out.to_id, input.to_id);
        assert_eq!(out.label, input.label);
        assert_eq!(out.evidence, input.evidence);
        assert_eq!(out.target_proposed, input.target_proposed);
    }

    #[test]
    fn overwrites_existing_labeller_id() {
        let mut labeller = NoopLabeller;
        let mut input = template_record();
        input.labeller_id = Some("prior:labeller".to_string());
        let out = labeller.label_one(input).unwrap();
        assert_eq!(out.labeller_id.as_deref(), Some(NOOP_LABELLER_ID));
    }
}
