//! Prompt rendering and parsing helpers.
//!
//! [`render_prompt`] converts a [`SampleRecord`] into the system + user
//! message pair sent to the model; [`Verdict::parse_response`] (in
//! `verdict.rs`) parses the model's JSON response back into structured
//! verdict fields. The system prompt is the labeller's contract with the
//! model: any prompt change must keep the output shape conformant with
//! `AUDIT-LABEL-SCHEMA.md` § Record schema labeller-fillable columns.

use scope_audit_labeller_core::SampleRecord;

/// Rendered prompt sent to the provider.
#[derive(Debug, Clone)]
pub struct Prompt {
    pub system: String,
    pub user: String,
}

/// System prompt. Defines the verdict shape the model must emit. Mirrors
/// the v2 labeller-fillable column names exactly so verdict parsing is a
/// direct `serde_json::from_str` rather than a translation step.
const SYSTEM_PROMPT: &str = r#"You are an auditor for a static-analysis tool that extracts code edges (calls, imports, instantiations, etc.) from source code. For each edge you receive, decide whether the extractor's verdict is correct.

You MUST reply with a single JSON object and nothing else — no preamble, no markdown fence. The object has these keys:

- `label`: boolean | null. `true` if the edge is correctly extracted; `false` if it is wrong; `null` if you cannot tell (abstain).
- `evidence`: object | null. Structured evidence supporting the verdict. May include `reasoning`, `referenced_symbol`, `definition_location`, etc. Schema is labeller-defined. Use `null` when the verdict is `null`.
- `target_proposed`: string | null. If `label` is `false` and you can suggest a corrected target identifier, provide it; otherwise `null`.
- `kind_proposed`: string | null. If `label` is `false` and the edge kind is itself wrong (e.g. classified `calls` but is actually `imports`), provide the corrected kind; otherwise `null`.
- `confidence_proposed`: string | null. If `label` is `false` and the extractor's `confidence` rating is itself wrong, provide one of `"high"`, `"medium"`, `"low"`; otherwise `null`.
- `reasoning_text`: string | null. Free-text explanation of the verdict. Useful as an audit trail when a `false` verdict is reviewed later.
- `lang_version_evidence`: string | null. If the edge's correctness depends on a specific language version (e.g. an import only valid under Python 3.10+), name the version; otherwise `null`.

When in doubt, abstain (`label: null`). A confident wrong answer is worse than an honest abstain."#;

/// Render one record into the prompt sent to the provider. The user
/// message holds the extractor's existing verdict (kind, from, to,
/// confidence, source snippet, language details) so the model has
/// everything it needs to judge the edge.
pub fn render_prompt(record: &SampleRecord) -> Prompt {
    let lang_version = record.lang_version.as_deref().unwrap_or("unknown");
    let user = format!(
        "Edge to audit:\n\
         - kind: {kind}\n\
         - from: {from}\n\
         - to: {to}\n\
         - extractor_confidence: {confidence}\n\
         - producer: {producer}\n\
         - pattern_id: {pattern_id}\n\
         - lang_version: {lang_version}\n\
         - source_snippet:\n```\n{snippet}\n```\n\n\
         Reply with the JSON verdict object only.",
        kind = record.kind,
        from = record.from_id,
        to = record.to_id,
        confidence = record.confidence,
        producer = record.producer,
        pattern_id = record.pattern_id,
        lang_version = lang_version,
        snippet = record.source_snippet,
    );

    Prompt {
        system: SYSTEM_PROMPT.to_string(),
        user,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_record() -> SampleRecord {
        SampleRecord {
            schema_version: "2".to_string(),
            edge_id: "e-1".to_string(),
            kind: "calls".to_string(),
            confidence: "medium".to_string(),
            producer: "rust".to_string(),
            pattern_id: "rust.calls.method".to_string(),
            from_id: "Foo::bar".to_string(),
            to_id: "Baz::quux".to_string(),
            source_snippet: "Baz::quux()".to_string(),
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
    fn user_prompt_contains_edge_fields() {
        let p = render_prompt(&template_record());
        assert!(p.user.contains("kind: calls"));
        assert!(p.user.contains("from: Foo::bar"));
        assert!(p.user.contains("to: Baz::quux"));
        assert!(p.user.contains("extractor_confidence: medium"));
        assert!(p.user.contains("pattern_id: rust.calls.method"));
        assert!(p.user.contains("lang_version: 2021"));
        assert!(p.user.contains("Baz::quux()"));
    }

    #[test]
    fn lang_version_absent_renders_unknown() {
        let mut r = template_record();
        r.lang_version = None;
        let p = render_prompt(&r);
        assert!(p.user.contains("lang_version: unknown"));
    }

    #[test]
    fn system_prompt_names_all_seven_verdict_fields() {
        let s = SYSTEM_PROMPT;
        for field in [
            "label",
            "evidence",
            "target_proposed",
            "kind_proposed",
            "confidence_proposed",
            "reasoning_text",
            "lang_version_evidence",
        ] {
            assert!(s.contains(field), "system prompt missing field {field}");
        }
    }
}
