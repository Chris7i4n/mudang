/// Integration tests for `scope audit history` (R8 read-side surface
/// over `edge_audit_history`; sprint 0004 BACKLOG (j)).
///
/// Three forms ship in sprint 0004:
/// - default dashboard (no subcommand)
/// - `edge <edge_id>` drill
/// - `pattern <pattern_id>` drill
///
/// Tests cover the empty-state path (no `--label` runs yet) and the
/// populated path (one full labelling pass against the TS fixture).
use assert_cmd::Command;
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

/// Emit a sample and apply a mixed verdict distribution so every
/// audit-history shape (correct / incorrect / skipped) is exercised in
/// one run. Returns the sample path + per-label counts.
fn emit_and_label_mixed(root: &Path) -> (PathBuf, (usize, usize, usize)) {
    let sample = root.join("sample.jsonl");
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(root)
        .assert()
        .success();
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut correct = 0usize;
    let mut incorrect = 0usize;
    let mut skipped = 0usize;
    for (i, l) in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .enumerate()
    {
        if i % 3 == 0 {
            skipped += 1;
            out_lines.push(l.to_string());
        } else if i % 2 == 1 {
            correct += 1;
            out_lines.push(l.replace("\"label\":null", "\"label\":true"));
        } else {
            incorrect += 1;
            out_lines.push(l.replace("\"label\":null", "\"label\":false"));
        }
    }
    std::fs::write(&sample, format!("{}\n", out_lines.join("\n"))).unwrap();
    let _ = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(root)
        .assert();
    (sample, (correct, incorrect, skipped))
}

// ─────────────────────────────────────────────────────────────
// Empty-state path: no `--label` runs yet → dashboard + drills exit 0
// with explanatory messages.

#[test]
fn test_audit_history_empty_state_dashboard_returns_ok() {
    let (_dir, root) = setup_indexed_fixture();
    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains("no audit history yet"),
        "expected empty-state notice; got: {stdout}"
    );
}

#[test]
fn test_audit_history_empty_state_edge_drill_returns_ok() {
    let (_dir, root) = setup_indexed_fixture();
    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "edge", "42"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("no history for edge_id 42"));
}

#[test]
fn test_audit_history_empty_state_pattern_drill_returns_ok() {
    let (_dir, root) = setup_indexed_fixture();
    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "pattern", "ts.calls.method"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("no history for pattern_id"));
}

#[test]
fn test_audit_history_empty_state_json_dashboard_shape() {
    let (_dir, root) = setup_indexed_fixture();
    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "--json"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["latest_audit_id"].is_null());
    assert_eq!(v["records_total"].as_u64().unwrap(), 0);
    assert!(v["regressing_patterns"].as_array().unwrap().is_empty());
    assert!(v["flapping_edges"].as_array().unwrap().is_empty());
}

// ─────────────────────────────────────────────────────────────
// Populated path: one mixed-verdict --label run already executed.

#[test]
fn test_audit_history_populated_dashboard_carries_headline() {
    let (_dir, root) = setup_indexed_fixture();
    let (_sample, (correct, incorrect, skipped)) = emit_and_label_mixed(&root);
    let total = correct + incorrect + skipped;

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("latest_audit_id\t1"));
    assert!(stdout.contains(&format!("records_total\t{total}")));
    assert!(stdout.contains(&format!("records_correct\t{correct}")));
    assert!(stdout.contains(&format!("records_incorrect\t{incorrect}")));
    assert!(stdout.contains(&format!("records_skipped\t{skipped}")));
}

#[test]
fn test_audit_history_populated_dashboard_json_shape() {
    let (_dir, root) = setup_indexed_fixture();
    let (_sample, (correct, incorrect, skipped)) = emit_and_label_mixed(&root);
    let total = correct + incorrect + skipped;

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "--json"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["latest_audit_id"].as_i64().unwrap(), 1);
    assert_eq!(v["records_total"].as_u64().unwrap() as usize, total);
    assert_eq!(v["records_correct"].as_u64().unwrap() as usize, correct);
    assert_eq!(v["records_incorrect"].as_u64().unwrap() as usize, incorrect);
    assert_eq!(v["records_skipped"].as_u64().unwrap() as usize, skipped);
    // overall_precision = correct / (correct + incorrect); present when
    // any labelled rows exist.
    let labelled = correct + incorrect;
    if labelled > 0 {
        let expected = correct as f64 / labelled as f64;
        let got = v["overall_precision"].as_f64().unwrap();
        assert!((got - expected).abs() < 1e-9);
    }
    assert!(v["regressing_patterns"].is_array());
    assert!(v["flapping_edges"].is_array());
}

#[test]
fn test_audit_history_populated_edge_drill_lists_timeline() {
    let (_dir, root) = setup_indexed_fixture();
    let (_sample, _) = emit_and_label_mixed(&root);

    // Pick an edge_id that exists in the audit history — query the DB
    // directly rather than guessing.
    let db_path = root.join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let edge_id: i64 = conn
        .query_row(
            "SELECT edge_id FROM edge_audit_history ORDER BY edge_id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "edge"])
        .arg(edge_id.to_string())
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(
        stdout.contains(&format!("# scope audit history edge {edge_id}")),
        "expected header with edge_id: {stdout}"
    );
    assert!(stdout.contains(
        "audit_id\tlabelled_at\tlabeller_id\tlabel\ttarget_proposed\tkind_proposed\tconfidence_proposed"
    ));
    // At least one timeline row beneath the header (the one we just
    // wrote in --label).
    let body_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect();
    assert!(body_lines.len() >= 2, "expected header + ≥1 row: {stdout}");
}

#[test]
fn test_audit_history_populated_edge_drill_json_shape() {
    let (_dir, root) = setup_indexed_fixture();
    let _ = emit_and_label_mixed(&root);
    let db_path = root.join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let edge_id: i64 = conn
        .query_row(
            "SELECT edge_id FROM edge_audit_history ORDER BY edge_id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "edge"])
        .arg(edge_id.to_string())
        .arg("--json")
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["edge_id"].as_i64().unwrap(), edge_id);
    let timeline = v["timeline"].as_array().unwrap();
    assert!(!timeline.is_empty());
    for r in timeline {
        for field in [
            "audit_id",
            "labelled_at",
            "labeller_id",
            "label",
            "target_proposed",
            "kind_proposed",
            "confidence_proposed",
        ] {
            assert!(r.get(field).is_some(), "missing {field}: {r}");
        }
    }
}

#[test]
fn test_audit_history_populated_pattern_drill_lists_timeline() {
    let (_dir, root) = setup_indexed_fixture();
    let _ = emit_and_label_mixed(&root);

    // Pick a pattern_id that appears in the audit history.
    let db_path = root.join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let pattern_id: String = conn
        .query_row(
            "SELECT e.pattern_id FROM edge_audit_history eh
               JOIN edges e ON e.edge_id = eh.edge_id
              GROUP BY e.pattern_id
              ORDER BY e.pattern_id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "pattern", &pattern_id])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains(&format!("# scope audit history pattern {pattern_id}")));
    assert!(stdout.contains("# precision-over-time"));
    assert!(stdout.contains("audit_id\tlabelled_at\tlabelled_count\tcorrect_count\tprecision"));
    assert!(stdout.contains("# currently incorrect"));
}

#[test]
fn test_audit_history_populated_pattern_drill_json_shape() {
    let (_dir, root) = setup_indexed_fixture();
    let _ = emit_and_label_mixed(&root);
    let db_path = root.join(".scope").join("graph.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let pattern_id: String = conn
        .query_row(
            "SELECT e.pattern_id FROM edge_audit_history eh
               JOIN edges e ON e.edge_id = eh.edge_id
              GROUP BY e.pattern_id
              ORDER BY e.pattern_id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "pattern", &pattern_id, "--json"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["pattern_id"].as_str().unwrap(), pattern_id);
    let timeline = v["precision_over_time"].as_array().unwrap();
    assert!(!timeline.is_empty());
    for p in timeline {
        for field in [
            "audit_id",
            "labelled_at",
            "labelled_count",
            "correct_count",
            "precision",
        ] {
            assert!(p.get(field).is_some(), "missing {field}: {p}");
        }
    }
    assert!(v["currently_incorrect"].is_array());
}

#[test]
fn test_audit_history_two_runs_dashboard_shows_latest_only() {
    // Two back-to-back runs against the same labelled sample. The
    // dashboard headline must surface `latest_audit_id = 2` and the
    // headline counts must reflect run 2's rows only (not the
    // accumulated total across both runs).
    let (_dir, root) = setup_indexed_fixture();
    let (sample, (correct, incorrect, skipped)) = emit_and_label_mixed(&root);
    let total = correct + incorrect + skipped;

    let _ = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "history", "--json"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["latest_audit_id"].as_i64().unwrap(), 2);
    assert_eq!(v["records_total"].as_u64().unwrap() as usize, total);
}
