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

#[test]
fn test_audit_confidence_emit_then_label_round_trips() {
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label every record true (placeholder labeller — replaces null with true).
    let raw = std::fs::read_to_string(&sample).unwrap();
    let labelled: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.replace("\"label\":null", "\"label\":true"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&sample, format!("{labelled}\n")).unwrap();

    // --label is chunk-4-complete: it parses + drift-checks then bails
    // with the chunk-5/6 pointer. Until chunks 5-6 land we assert that
    // the parser accepts the round-tripped file (no schema_version /
    // drift errors).
    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure(); // chunks 5-6 not yet wired => intentional bail
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();
    assert!(
        stderr.contains("Precision report") || stderr.contains("chunks 5-6"),
        "expected chunk-5-6 pointer in stderr; got: {stderr}"
    );
    assert!(
        !stderr.contains("source drift"),
        "no drift expected; got: {stderr}"
    );
    assert!(
        !stderr.contains("unknown schema_version"),
        "no schema mismatch expected; got: {stderr}"
    );
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
