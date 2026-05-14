//! Live-API integration test against the real DeepSeek endpoint.
//!
//! Off by default. To exercise it locally:
//!
//! ```text
//! DEEPSEEK_API_KEY=sk-... cargo test \
//!     -p scope-audit-labeller-llm --features live-deepseek-tests \
//!     --test live_deepseek
//! ```
//!
//! Two-layer opt-in: the `live-deepseek-tests` cargo feature gates
//! compilation, and the test additionally no-ops when `DEEPSEEK_API_KEY`
//! is unset. Default `cargo test --workspace` therefore never reaches
//! the network.

#![cfg(feature = "live-deepseek-tests")]

use std::io::Cursor;

use scope_audit_labeller_core::{run_labeller, SampleRecord};
use scope_audit_labeller_llm::{providers::deepseek::DeepSeekProvider, LlmLabeller};

fn base_record() -> SampleRecord {
    SampleRecord {
        schema_version: "2".to_string(),
        edge_id: "e-live-1".to_string(),
        kind: "calls".to_string(),
        confidence: "medium".to_string(),
        producer: "rust".to_string(),
        pattern_id: "rust.calls.method".to_string(),
        from_id: "main".to_string(),
        to_id: "println".to_string(),
        source_snippet: "println!(\"hello\");".to_string(),
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
fn round_trip_one_record_against_live_deepseek() {
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("DEEPSEEK_API_KEY unset; skipping live-deepseek test");
        return;
    }
    let provider = DeepSeekProvider::from_env().expect("api key present");
    let mut labeller = LlmLabeller::new(provider);

    let record = base_record();
    let fixture = format!("{}\n", serde_json::to_string(&record).unwrap());
    let mut output = Vec::new();
    let stats = run_labeller(&mut labeller, Cursor::new(fixture.as_bytes()), &mut output)
        .expect("run_labeller against live endpoint");
    assert_eq!(stats.records_labelled, 1);

    let parsed: SampleRecord =
        serde_json::from_str(String::from_utf8(output).unwrap().trim_end()).unwrap();
    assert_eq!(parsed.schema_version, "2");
    assert_eq!(parsed.edge_id, "e-live-1");
    assert_eq!(
        parsed.labeller_id.as_deref(),
        Some("llm:deepseek:deepseek-chat")
    );
}
