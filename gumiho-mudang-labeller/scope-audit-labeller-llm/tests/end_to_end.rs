//! End-to-end mock-provider integration test.
//!
//! Verifies the full [`LlmLabeller`] pipeline:
//!
//! - Reads a v2 JSONL fixture from a [`std::io::BufRead`].
//! - Runs each record through an [`LlmLabeller`] backed by a
//!   [`MockProvider`] returning canned responses in order.
//! - Asserts the resulting JSONL stream is v2-conformant and carries the
//!   three-segment `labeller_id` plus the verdict fields the mock
//!   prescribed.
//!
//! Also includes a binary smoke test for the `no-API-key` error path —
//! the only branch of the shipped binary's main() that can run without
//! network access. The live-API path is exercised by `live_deepseek.rs`
//! behind the `live-deepseek-tests` cargo feature.

use std::io::Cursor;
use std::process::{Command, Stdio};

use scope_audit_labeller_core::{run_labeller, SampleRecord};
use scope_audit_labeller_llm::{
    mock::{MockProvider, MockResponse},
    LlmLabeller,
};

/// Minimal v2 record with all 18 fields. Mutate in callers to fill the
/// per-fixture variations.
fn base_record(edge_id: &str, kind: &str, from: &str, to: &str) -> SampleRecord {
    SampleRecord {
        schema_version: "2".to_string(),
        edge_id: edge_id.to_string(),
        kind: kind.to_string(),
        confidence: "medium".to_string(),
        producer: "rust".to_string(),
        pattern_id: format!("rust.{kind}.example"),
        from_id: from.to_string(),
        to_id: to.to_string(),
        source_snippet: format!("{from}::{to}();"),
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
fn three_record_pipeline_with_mixed_verdicts() {
    let r1 = base_record("e-1", "calls", "Foo::bar", "Baz::quux");
    let r2 = base_record("e-2", "imports", "main", "std::io");
    let r3 = base_record("e-3", "calls", "alpha", "beta");

    let mut fixture = String::new();
    for r in [&r1, &r2, &r3] {
        fixture.push_str(&serde_json::to_string(r).unwrap());
        fixture.push('\n');
    }

    let provider = MockProvider::new("mock", "m1").with_responses(vec![
        MockResponse::ok(r#"{"label": true, "evidence": {"reasoning": "exact match"}, "reasoning_text": "correctly extracted"}"#),
        MockResponse::ok(r#"{"label": null, "reasoning_text": "cannot decide without imports table"}"#),
        MockResponse::ok(r#"{"label": false, "target_proposed": "gamma", "kind_proposed": "instantiates", "reasoning_text": "wrong kind and target"}"#),
    ]);
    let mut labeller = LlmLabeller::with_diagnostics(provider, Vec::new());

    let mut output = Vec::new();
    let stats = run_labeller(&mut labeller, Cursor::new(fixture.as_bytes()), &mut output).unwrap();
    assert_eq!(stats.records_labelled, 3);

    let out_str = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = out_str.trim_end().lines().collect();
    assert_eq!(lines.len(), 3);

    let parsed: Vec<SampleRecord> = lines
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    // labeller_id stamped on every record.
    for r in &parsed {
        assert_eq!(r.labeller_id.as_deref(), Some("llm:mock:m1"));
        assert_eq!(r.schema_version, "2");
    }

    // Per-record verdicts match what the mock returned.
    assert_eq!(parsed[0].label, Some(true));
    assert!(parsed[0].evidence.is_some());
    assert_eq!(
        parsed[0].reasoning_text.as_deref(),
        Some("correctly extracted")
    );

    assert_eq!(parsed[1].label, None);
    assert_eq!(
        parsed[1].reasoning_text.as_deref(),
        Some("cannot decide without imports table")
    );

    assert_eq!(parsed[2].label, Some(false));
    assert_eq!(parsed[2].target_proposed.as_deref(), Some("gamma"));
    assert_eq!(parsed[2].kind_proposed.as_deref(), Some("instantiates"));
}

#[test]
fn unparseable_response_in_middle_yields_abstain_and_pipeline_continues() {
    let r1 = base_record("e-1", "calls", "a", "b");
    let r2 = base_record("e-2", "calls", "c", "d");
    let r3 = base_record("e-3", "calls", "e", "f");

    let mut fixture = String::new();
    for r in [&r1, &r2, &r3] {
        fixture.push_str(&serde_json::to_string(r).unwrap());
        fixture.push('\n');
    }

    let provider = MockProvider::new("mock", "m1").with_responses(vec![
        MockResponse::ok(r#"{"label": true}"#),
        MockResponse::ok("definitely not json"),
        MockResponse::ok(r#"{"label": false}"#),
    ]);
    let mut diagnostics = Vec::new();
    let mut labeller = LlmLabeller::with_diagnostics(provider, &mut diagnostics);

    let mut output = Vec::new();
    let stats = run_labeller(&mut labeller, Cursor::new(fixture.as_bytes()), &mut output).unwrap();
    assert_eq!(stats.records_labelled, 3);

    let out_str = String::from_utf8(output).unwrap();
    let parsed: Vec<SampleRecord> = out_str
        .trim_end()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(parsed[0].label, Some(true));
    assert_eq!(parsed[1].label, None); // abstain on parse error
    assert_eq!(parsed[2].label, Some(false));
    // All three still carry labeller_id.
    assert!(parsed.iter().all(|r| r.labeller_id.is_some()));

    let diag = String::from_utf8(diagnostics).unwrap();
    assert!(diag.contains("verdict-parse error"));
    assert!(diag.contains("e-2"));
}

#[test]
fn transport_error_yields_abstain_and_pipeline_continues() {
    let r1 = base_record("e-1", "calls", "a", "b");
    let r2 = base_record("e-2", "calls", "c", "d");

    let mut fixture = String::new();
    for r in [&r1, &r2] {
        fixture.push_str(&serde_json::to_string(r).unwrap());
        fixture.push('\n');
    }

    let provider = MockProvider::new("mock", "m1").with_responses(vec![
        MockResponse::err("503 down"),
        MockResponse::ok(r#"{"label": true}"#),
    ]);
    let mut diagnostics = Vec::new();
    let mut labeller = LlmLabeller::with_diagnostics(provider, &mut diagnostics);

    let mut output = Vec::new();
    run_labeller(&mut labeller, Cursor::new(fixture.as_bytes()), &mut output).unwrap();

    let parsed: Vec<SampleRecord> = String::from_utf8(output)
        .unwrap()
        .trim_end()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(parsed[0].label, None); // abstain
    assert_eq!(parsed[1].label, Some(true));

    let diag = String::from_utf8(diagnostics).unwrap();
    assert!(diag.contains("provider error"));
    assert!(diag.contains("e-1"));
}

#[cfg(feature = "deepseek")]
#[test]
fn binary_errors_without_api_key() {
    use std::collections::HashMap;

    let binary = env!("CARGO_BIN_EXE_scope-audit-labeller-llm");

    // Clear DEEPSEEK_API_KEY from the spawned process env so the
    // assertion holds even when the developer has the key set locally.
    let mut env: HashMap<String, String> = std::env::vars().collect();
    env.remove("DEEPSEEK_API_KEY");

    let output = Command::new(binary)
        .env_clear()
        .envs(&env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn binary");

    assert!(
        !output.status.success(),
        "binary should not succeed without DEEPSEEK_API_KEY"
    );
    let code = output.status.code().expect("binary exited via signal");
    assert_eq!(code, 2, "expected exit code 2 for missing api key");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("DEEPSEEK_API_KEY"),
        "stderr should mention the missing env var; got: {stderr}"
    );
}
