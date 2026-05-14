//! End-to-end integration test: spawn the `scope-audit-labeller-noop`
//! binary, pipe a v2 JSONL sample fixture through stdin, parse stdout,
//! assert every output record is v2-conformant with `labeller_id` stamped
//! and every other input field preserved verbatim.

use std::io::Write;
use std::process::{Command, Stdio};

const FIXTURE_JSONL: &str = "\
{\"schema_version\":\"2\",\"edge_id\":\"e-9f2c\",\"kind\":\"calls\",\"confidence\":\"high\",\"producer\":\"rust\",\"pattern_id\":\"rust.calls.method\",\"from\":\"crate::handlers::greet\",\"to\":\"crate::utils::format_name\",\"source_snippet\":\"format_name(&user.name)\",\"lang_version\":\"2021\",\"label\":null,\"evidence\":null,\"target_proposed\":null,\"kind_proposed\":null,\"confidence_proposed\":null,\"reasoning_text\":null,\"lang_version_evidence\":null,\"labeller_id\":null}
{\"schema_version\":\"2\",\"edge_id\":\"e-3a17\",\"kind\":\"extends\",\"confidence\":\"high\",\"producer\":\"typescript\",\"pattern_id\":\"ts.extends.class\",\"from\":\"components/Button.tsx::PrimaryButton\",\"to\":\"components/Button.tsx::BaseButton\",\"source_snippet\":\"class PrimaryButton extends BaseButton {\",\"lang_version\":\"es2022\",\"label\":null,\"evidence\":null,\"target_proposed\":null,\"kind_proposed\":null,\"confidence_proposed\":null,\"reasoning_text\":null,\"lang_version_evidence\":null,\"labeller_id\":null}
";

#[test]
fn noop_binary_stamps_labeller_id_on_every_record() {
    let bin = env!("CARGO_BIN_EXE_scope-audit-labeller-noop");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn noop binary");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(FIXTURE_JSONL.as_bytes())
        .expect("write fixture to stdin");

    let output = child.wait_with_output().expect("wait_with_output");
    assert!(
        output.status.success(),
        "noop binary exited with non-zero status: {}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected two output records, got: {stdout:?}");

    for line in &lines {
        let v: serde_json::Value =
            serde_json::from_str(line).expect("output record is valid JSON");
        assert_eq!(v["schema_version"], "2");
        assert_eq!(v["labeller_id"], "noop:reference-v0");
    }

    // Round-trip: every non-labeller_id field is preserved byte-for-byte.
    let expected_first = FIXTURE_JSONL
        .lines()
        .next()
        .unwrap()
        .replace(r#""labeller_id":null"#, r#""labeller_id":"noop:reference-v0""#);
    assert_eq!(lines[0], expected_first);
}

#[test]
fn noop_binary_rejects_unknown_schema_version() {
    let bin = env!("CARGO_BIN_EXE_scope-audit-labeller-noop");
    let bad = FIXTURE_JSONL.replace("\"schema_version\":\"2\"", "\"schema_version\":\"1\"");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(bad.as_bytes())
        .expect("write bad fixture");

    let output = child.wait_with_output().expect("wait");
    assert!(!output.status.success(), "expected non-zero exit on bad schema_version");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown schema_version"),
        "stderr did not surface the diagnostic: {stderr}"
    );
}
