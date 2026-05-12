/// Integration tests for `scope audit confidence` (R8 / sprint 0007).
///
/// Chunk 4 surface: `--emit-sample <PATH>` writes a JSONL sample,
/// `--label <PATH>` reads it back. Both flows hard-abort on source
/// drift (auditor immutability rule — see
/// `gumiho-mudang-scope/docs/AUDIT-LABEL-SCHEMA.md`).
use assert_cmd::Command;
use predicates::str::contains;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const TS_FIXTURE: &str = "tests/fixtures/typescript-simple";

fn copy_dir_all(src: &Path, dest: &Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dest_path);
        } else {
            std::fs::copy(&src_path, &dest_path).unwrap();
        }
    }
}

fn setup_indexed_fixture() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    copy_dir_all(Path::new(TS_FIXTURE), dir.path());

    Command::cargo_bin("mudang")
        .unwrap()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["index", "--full"])
        .current_dir(dir.path())
        .assert()
        .success();

    let root = dir.path().to_path_buf();
    (dir, root)
}

#[test]
fn test_audit_confidence_emit_sample_writes_jsonl() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let content = std::fs::read_to_string(&sample).unwrap();
    assert!(!content.is_empty(), "sample file should not be empty");

    // Every line must be a valid JSON record with schema_version "1"
    // and the contract-mandated fields per docs/AUDIT-LABEL-SCHEMA.md.
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} not valid JSON: {e}\n{line}", i + 1));
        assert_eq!(v["schema_version"], "1");
        for field in [
            "edge_id",
            "kind",
            "confidence",
            "producer",
            "pattern_id",
            "from",
            "to",
            "source_snippet",
            "lang_version",
            "label",
        ] {
            assert!(
                v.get(field).is_some(),
                "line {} missing field {field}: {line}",
                i + 1
            );
        }
        assert!(v["label"].is_null(), "emitted label must be null");
    }
}

/// Helper: emit a sample then label every record `true`.
fn emit_and_label_all_true(root: &Path) -> PathBuf {
    let sample = root.join("sample.jsonl");
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let labelled: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.replace("\"label\":null", "\"label\":true"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&sample, format!("{labelled}\n")).unwrap();
    sample
}

#[test]
fn test_audit_confidence_label_emits_json_report_by_default() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = emit_and_label_all_true(&root);

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();

    // Parse the report and verify its shape matches the pre-Phase-D
    // ambiguity #4 contract.
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout not valid JSON: {e}\n{stdout}"));
    assert_eq!(report["schema_version"], "1");
    assert!(report["disclaimer"]
        .as_str()
        .unwrap()
        .contains("precision report"));
    let rows = report["report"].as_array().expect("report array");
    assert!(!rows.is_empty(), "expected at least one report row");
    for row in rows {
        for field in [
            "kind",
            "tier",
            "producer",
            "pattern_id",
            "sample_size",
            "correct_count",
            "precision",
        ] {
            assert!(
                row.get(field).is_some(),
                "row missing field {field}: {row}"
            );
        }
        // Labeller said true everywhere => precision must be 1.0.
        assert_eq!(row["precision"].as_f64().unwrap(), 1.0);
        assert_eq!(row["sample_size"], row["correct_count"]);
    }
}

#[test]
fn test_audit_confidence_label_format_tsv() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = emit_and_label_all_true(&root);

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .args(["--format", "tsv"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();

    // Preamble (chunk 8): two `#`-prefixed lines carrying the
    // precision-only disclaimer and the sample-file schema pointer.
    let mut lines = stdout.lines();
    let p1 = lines.next().expect("preamble line 1");
    let p2 = lines.next().expect("preamble line 2");
    assert!(p1.starts_with("# "), "preamble must be #-prefixed: {p1:?}");
    assert!(p2.starts_with("# "), "preamble must be #-prefixed: {p2:?}");
    assert!(
        p1.contains("precision report") && p1.contains("recall"),
        "first preamble line must carry the precision-only disclaimer: {p1:?}"
    );
    assert!(
        p2.contains("docs/AUDIT-LABEL-SCHEMA.md"),
        "second preamble line must point to the sample-file schema doc: {p2:?}"
    );

    let header = lines.next().expect("header line");
    assert_eq!(
        header,
        "kind\ttier\tproducer\tpattern_id\tsample_size\tcorrect_count\tprecision"
    );
    let body: Vec<_> = lines.collect();
    assert!(!body.is_empty(), "expected at least one body row");
    for row in body {
        let cols: Vec<&str> = row.split('\t').collect();
        assert_eq!(cols.len(), 7, "expected 7 tab-separated columns: {row}");
        // All-true labelling => precision rendered as 1.0000.
        assert_eq!(cols[6], "1.0000", "row: {row}");
    }
}

#[test]
fn test_audit_confidence_label_json_carries_sample_schema_doc() {
    // Chunk 8: external labeller authors must discover the sample-file
    // contract directly from the report header. Verify the JSON shape
    // includes `sample_schema_doc` pointing at docs/AUDIT-LABEL-SCHEMA.md.
    let (_dir, root) = setup_indexed_fixture();
    let sample = emit_and_label_all_true(&root);

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let doc = report["sample_schema_doc"]
        .as_str()
        .expect("sample_schema_doc must be a string");
    assert!(
        doc.contains("docs/AUDIT-LABEL-SCHEMA.md"),
        "expected AUDIT-LABEL-SCHEMA.md pointer: {doc:?}"
    );
    assert!(
        doc.contains("schema_version"),
        "doc pointer must cite schema_version: {doc:?}"
    );
}

#[test]
fn test_audit_confidence_tier_gate_fails_on_low_precision_high_tier() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label every record `false`: every high-tier group's precision = 0,
    // which is far below the 95% target. Tier gate must fail the run.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let labelled: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.replace("\"label\":null", "\"label\":false"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&sample, format!("{labelled}\n")).unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure();

    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();

    // Report still printed (so the operator sees every offender).
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must still carry the JSON report");
    assert_eq!(report["schema_version"], "1");

    // Tier gate error printed to stderr with target percentages and remediation.
    assert!(
        stderr.contains("tier gate"),
        "stderr must mention tier gate; got: {stderr}"
    );
    assert!(
        stderr.contains("95%") || stderr.contains("0.9500"),
        "stderr must reference the high-tier target; got: {stderr}"
    );
    assert!(
        stderr.contains("Remediation"),
        "stderr must include remediation; got: {stderr}"
    );
}

#[test]
fn test_audit_confidence_label_rejects_null_labels() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();
    // Don't fill any labels: every record still has label=null.

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("label=null"))
        .stderr(contains("complete labelling"));
}

#[test]
fn test_audit_confidence_emit_aborts_on_source_drift() {
    let (_dir, root) = setup_indexed_fixture();

    // Edit a source file between `scope index` and `--emit-sample`.
    let service = root.join("src/payments/service.ts");
    let original = std::fs::read_to_string(&service).unwrap();
    std::fs::write(&service, format!("{original}\n// drift\n")).unwrap();

    let sample = root.join("sample.jsonl");
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("source drift detected"))
        .stderr(contains("scope index"))
        .stderr(contains("no `--allow-drift` escape"));

    // No partial JSONL must be left behind on drift abort. We accept
    // either "file not created" or "file is empty" — both satisfy the
    // contract that drift aborts before any write commits.
    if sample.exists() {
        let content = std::fs::read_to_string(&sample).unwrap();
        assert!(
            content.trim().is_empty(),
            "drift abort must not leave partial JSONL; got: {content:?}"
        );
    }
}

#[test]
fn test_audit_confidence_label_rejects_unknown_schema_version() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    // Emit a valid sample first so the drift gate stays green.
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Mutate the first record's schema_version to a future value.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let bumped = raw.replacen("\"schema_version\":\"1\"", "\"schema_version\":\"99\"", 1);
    std::fs::write(&sample, bumped).unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("unknown schema_version"))
        .stderr(contains("Re-emit"));
}

#[test]
fn test_audit_confidence_emit_and_label_are_mutually_exclusive() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .arg("--label")
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("cannot be used with"));
}
