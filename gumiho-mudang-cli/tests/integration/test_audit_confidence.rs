/// Integration tests for `scope audit confidence` (R8).
///
/// Surface: `--emit-sample <PATH>` writes a JSONL sample, `--label
/// <PATH>` reads it back. Both flows hard-abort on source drift
/// (auditor immutability rule — see
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

/// Rewrite the `"lang_version":...` value in a single JSONL line to
/// `new_value` (which must include surrounding quotes / be the literal
/// `null`). Returns `None` when the line carries no `lang_version`
/// field. Handles both `null` and string-quoted values.
fn rewrite_lang_version(line: &str, new_value: &str) -> Option<String> {
    let key = "\"lang_version\":";
    let key_pos = line.find(key)?;
    let value_start = key_pos + key.len();
    let rest = &line[value_start..];
    let value_end = if rest.starts_with("null") {
        4
    } else if rest.starts_with('"') {
        // Find closing quote (no JSON-escape handling needed — lang_version
        // values come from manifests and never contain `"` or `\`).
        rest[1..].find('"').map(|i| i + 2)?
    } else {
        return None;
    };
    let mut out = String::with_capacity(line.len() + new_value.len());
    out.push_str(&line[..value_start]);
    out.push_str(new_value);
    out.push_str(&rest[value_end..]);
    Some(out)
}

/// Remove the entire `,"lang_version":...` field (including the
/// leading comma) from a JSONL line.
fn strip_lang_version_field(line: &str) -> Option<String> {
    let key = ",\"lang_version\":";
    let key_pos = line.find(key)?;
    let value_start = key_pos + key.len();
    let rest = &line[value_start..];
    let value_end = if rest.starts_with("null") {
        4
    } else if rest.starts_with('"') {
        rest[1..].find('"').map(|i| i + 2)?
    } else {
        return None;
    };
    let mut out = String::with_capacity(line.len());
    out.push_str(&line[..key_pos]);
    out.push_str(&rest[value_end..]);
    Some(out)
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

    // Every line must be a valid JSON record with schema_version "2"
    // and the contract-mandated fields per docs/AUDIT-LABEL-SCHEMA.md.
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} not valid JSON: {e}\n{line}", i + 1));
        assert_eq!(v["schema_version"], "2");
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
            "evidence",
            "target_proposed",
            "kind_proposed",
            "confidence_proposed",
            "reasoning_text",
            "lang_version_evidence",
            "labeller_id",
        ] {
            assert!(
                v.get(field).is_some(),
                "line {} missing field {field}: {line}",
                i + 1
            );
        }
        assert!(v["label"].is_null(), "emitted label must be null");
        // Sprint 0003 (BACKLOG.md § Priority 1 sub-item (d)) — the
        // indexer-side detector matrix populates `lang_version` on
        // emit; the typescript-simple fixture carries a tsconfig.json
        // with `target: "ES2020"` so every record sampled from it
        // must surface that value. A `null` here would be a
        // detector regression on the wiring path.
        assert_eq!(
            v["lang_version"],
            "ES2020",
            "line {} expected lang_version=ES2020 (from fixture tsconfig.json): {line}",
            i + 1
        );
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
            assert!(row.get(field).is_some(), "row missing field {field}: {row}");
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

    // Preamble: three `#`-prefixed lines carrying the precision-only
    // disclaimer, the sample-file schema pointer, and the
    // coverage-limitation note.
    let mut lines = stdout.lines();
    let p1 = lines.next().expect("preamble line 1");
    let p2 = lines.next().expect("preamble line 2");
    let p3 = lines.next().expect("preamble line 3");
    assert!(p1.starts_with("# "), "preamble must be #-prefixed: {p1:?}");
    assert!(p2.starts_with("# "), "preamble must be #-prefixed: {p2:?}");
    assert!(p3.starts_with("# "), "preamble must be #-prefixed: {p3:?}");
    assert!(
        p1.contains("precision report") && p1.contains("recall"),
        "first preamble line must carry the precision-only disclaimer: {p1:?}"
    );
    assert!(
        p2.contains("docs/AUDIT-LABEL-SCHEMA.md"),
        "second preamble line must point to the sample-file schema doc: {p2:?}"
    );
    assert!(
        p3.contains("BACKLOG.md"),
        "third preamble line must point to `BACKLOG.md`: {p3:?}"
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
fn test_audit_confidence_label_rejects_unparseable_edge_id() {
    // --label must NOT silently drop
    // records whose `edge_id` is not a parseable i64. Before the fix,
    // such records were excluded from the drift gate (via
    // filter_map(|r| r.edge_id.parse::<i64>().ok())) but still entered
    // the precision report, letting a tampered sample bypass the
    // integrity guarantees of the audit command.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label all true, then corrupt the first record's edge_id.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    for (i, l) in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .enumerate()
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if i == 0 {
            // Replace the first edge_id with a non-integer string.
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let real_id = v["edge_id"].as_str().unwrap().to_string();
            line = line.replace(
                &format!("\"edge_id\":\"{real_id}\""),
                "\"edge_id\":\"not-an-int\"",
            );
        }
        out_lines.push(line);
    }
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file integrity check failed"))
        .stderr(contains("non-integer edge_id"))
        .stderr(contains("not-an-int"))
        .stderr(contains("re-emit the sample"));
}

#[test]
fn test_audit_confidence_label_rejects_unknown_edge_id() {
    // Second arm: --label must NOT
    // silently drop records whose `edge_id` parses as i64 but no
    // longer exists in the current index. Before the fix, such
    // records bypassed the drift gate while still contributing to
    // the precision math.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label all true, then replace the first record's edge_id with a
    // very large i64 that cannot exist in this small fixture.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    for (i, l) in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .enumerate()
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if i == 0 {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let real_id = v["edge_id"].as_str().unwrap().to_string();
            line = line.replace(
                &format!("\"edge_id\":\"{real_id}\""),
                "\"edge_id\":\"999999999\"",
            );
        }
        out_lines.push(line);
    }
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file integrity check failed"))
        .stderr(contains("not in the current index"))
        .stderr(contains("999999999"))
        .stderr(contains("re-emit the sample"));
}

#[test]
fn test_audit_confidence_label_rejects_tampered_confidence_field() {
    // --label must NOT pass tier gate when
    // the labeller rewrites `confidence` (or other report-key fields)
    // while preserving a valid edge_id. Scenario: a buggy
    // labeller flips `confidence: high -> low` would silently route
    // the row into the low-tier (no minimum) and the run would pass
    // even though the indexed edge actually belongs to the high tier
    // (95% target).
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Flip the first record's confidence to a different valid tier
    // while keeping every other field intact. Whatever tier the
    // indexer stamps for this fixture (high / medium / low — typically
    // medium for the default builder), rewrite it to a value that
    // *differs* from the indexed row so the tamper gate fires.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut tampered_pair: Option<(String, String)> = None;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if tampered_pair.is_none() {
            for (real, fake) in [
                ("\"confidence\":\"high\"", "\"confidence\":\"low\""),
                ("\"confidence\":\"medium\"", "\"confidence\":\"low\""),
                ("\"confidence\":\"low\"", "\"confidence\":\"high\""),
            ] {
                if line.contains(real) {
                    line = line.replacen(real, fake, 1);
                    let real_tier = real
                        .trim_start_matches("\"confidence\":\"")
                        .trim_end_matches('"');
                    let fake_tier = fake
                        .trim_start_matches("\"confidence\":\"")
                        .trim_end_matches('"');
                    tampered_pair = Some((real_tier.to_string(), fake_tier.to_string()));
                    break;
                }
            }
        }
        out_lines.push(line);
    }
    let (real_tier, fake_tier) = tampered_pair
        .expect("emit-sample must produce at least one record with a valid confidence tier");
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file tamper check failed"))
        .stderr(contains("confidence"))
        .stderr(contains(format!("sample = \"{fake_tier}\"")))
        .stderr(contains(format!("indexed = \"{real_tier}\"")))
        .stderr(contains("re-emit the sample"));
}

#[test]
fn test_audit_confidence_label_rejects_tampered_lang_version() {
    // `lang_version` is the last non-`label` field the tamper gate
    // checks. The indexer-side detector matrix (per `BACKLOG.md`
    // § Priority 1 sub-item (d)) populates it on emit; the labelled
    // pass recomputes it via the same detector and compares. A
    // labeller rewriting `lang_version` to any other value is sample
    // tamper.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Rewrite the first record's `lang_version` to a value that does
    // not match what the detector recomputes for this fixture
    // (typescript-simple's tsconfig target is `ES2020`).
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut tampered = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if !tampered {
            if let Some(new_line) = rewrite_lang_version(&line, "\"fake-1.0\"") {
                line = new_line;
                tampered = true;
            }
        }
        out_lines.push(line);
    }
    assert!(tampered);
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file tamper check failed"))
        .stderr(contains("lang_version"))
        .stderr(contains("fake-1.0"));
}

#[test]
fn test_audit_confidence_label_rejects_records_missing_label_field() {
    // Regression: `serde` deserializes a missing `label` key
    // identically to `label: null` — both become `Option::None`. The
    // schema doc names `label` as required (with value null / true /
    // false), so a labeller bug that drops the key entirely should
    // surface as a contract failure, not silently flow through the
    // partial-coverage skip path.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Remove the `label` field from the first record entirely (not
    // just set to null). This is the "labeller serializer drops nulls"
    // scenario the scenario describes.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut stripped = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.to_string();
        if !stripped {
            // Remove `,"label":null` (the trailing field; emit always
            // serializes label last per the struct field order).
            line = line.replace(",\"label\":null", "");
            stripped = true;
        }
        out_lines.push(line);
    }
    assert!(stripped);
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("required field(s) `label`"))
        .stderr(contains(
            "missing key is not the same as an explicit `null`",
        ));
}

#[test]
fn test_audit_confidence_label_rejects_records_missing_lang_version_field() {
    // Same shape as the previous tamper test but for the other
    // Option-typed required field. A labeller that drops nulls would
    // omit `lang_version` (always null on emit) and produce JSONL
    // that violates the schema_version "2" required-fields contract.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut stripped = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if !stripped {
            if let Some(new_line) = strip_lang_version_field(&line) {
                line = new_line;
                stripped = true;
            }
        }
        out_lines.push(line);
    }
    assert!(stripped);
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("required field(s) `lang_version`"))
        .stderr(contains(
            "missing key is not the same as an explicit `null`",
        ));
}

#[test]
fn test_audit_confidence_default_no_flags_succeeds_with_usage_hint() {
    // Regression: the documented `scope audit confidence`
    // no-flag invocation must succeed and surface usage instructions,
    // not bail with a stale chunk-plan pointer.
    let (_dir, root) = setup_indexed_fixture();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence"])
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    assert!(stdout.contains("precision report"), "missing disclaimer");
    assert!(
        stdout.contains("docs/AUDIT-LABEL-SCHEMA.md"),
        "missing schema doc pointer"
    );
    assert!(
        stdout.contains("--emit-sample"),
        "missing usage hint for --emit-sample"
    );
    assert!(stdout.contains("--label"), "missing usage hint for --label");
    assert!(
        stdout.contains("high >= 95%") && stdout.contains("medium >= 70%"),
        "missing tier-target summary"
    );
    assert!(
        !stdout.contains("chunks 5-6") && !stdout.contains("land in sprint"),
        "stale planning pointer must be gone: {stdout}"
    );
}

#[test]
fn test_audit_confidence_source_drift_surfaces_drift_error_not_tamper() {
    // Regression: when a source file changes between emit and
    // label, the diagnosis must be source drift with `scope index`
    // remediation, NOT sample tamper with re-emit remediation. The
    // bug was that the tamper gate ran before the drift gate and
    // re-derived the snippet from the changed file, attributing the
    // diff to sample tamper instead of source drift.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label every record true (a clean labelling).
    let raw = std::fs::read_to_string(&sample).unwrap();
    let labelled: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.replace("\"label\":null", "\"label\":true"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&sample, format!("{labelled}\n")).unwrap();

    // Edit a source file in the working tree (NOT the sample file).
    // This is exactly the source-drift case: source drift between
    // emit and label.
    let service = root.join("src/payments/service.ts");
    let original = std::fs::read_to_string(&service).unwrap();
    std::fs::write(&service, format!("{original}\n// drifted after emit\n")).unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&out.get_output().stderr).to_string();

    assert!(
        stderr.contains("source drift detected"),
        "expected source-drift error; got: {stderr}"
    );
    assert!(
        stderr.contains("scope index"),
        "expected re-index remediation; got: {stderr}"
    );
    assert!(
        !stderr.contains("sample-file tamper check failed"),
        "drift must NOT be misdiagnosed as tamper; got: {stderr}"
    );
}

#[test]
fn test_audit_confidence_label_rejects_tampered_from_field() {
    // Regression: labeller alters `from` while keeping a valid
    // edge_id. The labeller then judged a different edge but report
    // credits the indexed one.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut tampered = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if !tampered {
            // Replace the first record's `from` value with a fake one.
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let original = v["from"].as_str().unwrap().to_string();
            line = line.replace(
                &format!("\"from\":\"{original}\""),
                "\"from\":\"fake::origin\"",
            );
            tampered = true;
        }
        out_lines.push(line);
    }
    assert!(tampered);
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file tamper check failed"))
        .stderr(contains("from"))
        .stderr(contains("fake::origin"));
}

#[test]
fn test_audit_confidence_label_rejects_tampered_to_field() {
    // Regression (second axis): labeller alters `to`.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut tampered = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if !tampered {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let original = v["to"].as_str().unwrap().to_string();
            line = line.replace(&format!("\"to\":\"{original}\""), "\"to\":\"fake::target\"");
            tampered = true;
        }
        out_lines.push(line);
    }
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file tamper check failed"))
        .stderr(contains("to: sample = "))
        .stderr(contains("fake::target"));
}

#[test]
fn test_audit_confidence_label_rejects_tampered_source_snippet() {
    // Regression (third axis): labeller alters `source_snippet`.
    // The labeller saw text the indexer never emitted; verdict applies
    // to a fake context.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    let mut tampered = false;
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut line = l.replace("\"label\":null", "\"label\":true");
        if !tampered {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            let original = v["source_snippet"].as_str().unwrap().to_string();
            // Rewrite snippet to something that definitely won't match.
            // Use a JSON-encoded literal so the line stays valid JSON.
            let original_json = serde_json::to_string(&original).unwrap();
            line = line.replace(
                &format!("\"source_snippet\":{original_json}"),
                "\"source_snippet\":\"// FAKE CONTEXT THE INDEXER NEVER SAW\"",
            );
            tampered = true;
        }
        out_lines.push(line);
    }
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("sample-file tamper check failed"))
        .stderr(contains("source_snippet"))
        .stderr(contains("FAKE CONTEXT"));
}

#[test]
fn test_audit_confidence_label_rejects_all_null_labels() {
    // Resolution: --label *tolerates* partial coverage
    // per the schema doc, but a sample where no record at all has been
    // labelled means no labelling has happened; that case still aborts.
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
        .stderr(contains("every record has label=null"))
        .stderr(contains("no labelling has been performed"));
}

#[test]
fn test_audit_confidence_label_tolerates_partial_coverage() {
    // Regression: per AUDIT-LABEL-SCHEMA.md the LSP cross-check
    // labeller leaves records it cannot classify as `label:null`. The
    // doc explicitly says "--label tolerates partial coverage". A
    // hard-rejection of nulls would contradict that; the active code
    // drops null records from group accumulation so the precision
    // denominator is honest (= number of labelled records per group).
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label half of the records true; leave the other half null.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    for (i, l) in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .enumerate()
    {
        if i % 2 == 0 {
            out_lines.push(l.replace("\"label\":null", "\"label\":true"));
        } else {
            out_lines.push(l.to_string()); // leave null
        }
    }
    std::fs::write(&sample, out_lines.join("\n") + "\n").unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = report["report"].as_array().expect("report array");
    assert!(!rows.is_empty(), "partial-coverage report still has rows");
    for row in rows {
        let n = row["sample_size"].as_u64().unwrap();
        let k = row["correct_count"].as_u64().unwrap();
        assert!(n > 0, "sample_size must be > 0 for emitted rows");
        // Half-and-half labelling with all-true on labelled side:
        // every emitted row has correct_count == sample_size.
        assert_eq!(k, n, "precision denominator must be labelled-count");
        assert_eq!(row["precision"].as_f64().unwrap(), 1.0);
    }
}

#[test]
fn test_audit_confidence_emit_refuses_to_overwrite_existing_file() {
    // Regression: --emit-sample pointed at an indexed source
    // path previously truncated the working tree (File::create) and
    // would then emit empty / wrong source_snippets for edges in that
    // file. Fix: refuse to overwrite anything at the destination.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("preexisting.jsonl");
    std::fs::write(&sample, "this file already exists\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("refusing to overwrite an existing path"));

    // The pre-existing file is intact.
    let after = std::fs::read_to_string(&sample).unwrap();
    assert_eq!(after, "this file already exists\n");
}

#[test]
fn test_audit_confidence_label_rejects_duplicate_edge_ids() {
    // Regression: duplicate edge_id in JSONL collapsed in the
    // freshness set but still double-counted in the precision report.
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    // Label all true, then append a duplicate of the first record.
    let raw = std::fs::read_to_string(&sample).unwrap();
    let labelled: Vec<String> = raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| l.replace("\"label\":null", "\"label\":true"))
        .collect();
    let mut with_dup = labelled.clone();
    with_dup.push(labelled[0].clone()); // exact duplicate
    std::fs::write(&sample, with_dup.join("\n") + "\n").unwrap();

    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .failure()
        .stderr(contains("integrity check failed"))
        .stderr(contains("repeated across multiple records"))
        .stderr(contains("on lines"));
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
    let bumped = raw.replacen("\"schema_version\":\"2\"", "\"schema_version\":\"99\"", 1);
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
fn test_audit_confidence_label_accepts_v1_records_backward_compatible() {
    // Sprint 0004 CP2: `--label` accepts both schema_version "1" and
    // "2". A v1 record carries only the v1 key set (11 keys) and has
    // the seven v2-only fields treated as `null` on read. This is the
    // sole post-bump backward-compat path; there is no dual-write or
    // auto-upgrade. See docs/AUDIT-LABEL-SCHEMA.md § Migration "1" → "2".
    let (_dir, root) = setup_indexed_fixture();
    let sample = root.join("sample.jsonl");

    // Emit (v2), then downgrade each record to v1: bump version to "1",
    // strip the seven v2 keys, label every record `true`.
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();

    let raw = std::fs::read_to_string(&sample).unwrap();
    let mut out_lines = Vec::new();
    for l in raw
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
        let obj = v.as_object_mut().unwrap();
        obj.insert(
            "schema_version".to_string(),
            serde_json::Value::String("1".to_string()),
        );
        for k in [
            "evidence",
            "target_proposed",
            "kind_proposed",
            "confidence_proposed",
            "reasoning_text",
            "lang_version_evidence",
            "labeller_id",
        ] {
            obj.remove(k);
        }
        obj.insert("label".to_string(), serde_json::Value::Bool(true));
        out_lines.push(serde_json::to_string(&v).unwrap());
    }
    std::fs::write(&sample, format!("{}\n", out_lines.join("\n"))).unwrap();

    let out = Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--label"])
        .arg(&sample)
        .current_dir(&root)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Report shape is identical to the v2 path: every label=true,
    // so every row's precision is 1.0.
    let rows = report["report"].as_array().unwrap();
    assert!(!rows.is_empty(), "v1 records produced an empty report");
    for r in rows {
        assert_eq!(
            r["precision"].as_f64().unwrap(),
            1.0,
            "v1 record path produced non-1.0 precision: {r}"
        );
    }
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

// ─────────────────────────────────────────────────────────────
// lang_version coverage gate (sprint 0003 (d)).
//
// One fixture per supported language; each fixture carries the
// canonical manifest form for that language's `lang_version`
// detector, plus enough source to produce sampleable edges. The
// gate fires when **any** emitted JSONL record carries `lang_version:
// null` — that is the "extractor regressed; detector unwired or
// fixture missing manifest" signal the sprint plan calls out.

const LANG_FIXTURES: &[(&str, &str, &str)] = &[
    ("rust", "tests/fixtures/rust-simple", "2021"),
    ("go", "tests/fixtures/go-simple", "1.21"),
    ("python", "tests/fixtures/python-simple", ">=3.10"),
    ("typescript", "tests/fixtures/typescript-simple", "ES2020"),
    ("java", "tests/fixtures/java-simple", "21"),
    ("csharp", "tests/fixtures/csharp-simple", "net8.0"),
    ("ruby", "tests/fixtures/ruby-simple", "3.2.2"),
];

fn run_audit_emit_for_fixture(fixture: &str) -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    copy_dir_all(Path::new(fixture), dir.path());
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
    let sample = dir.path().join("sample.jsonl");
    Command::cargo_bin("mudang")
        .unwrap()
        .args(["audit", "confidence", "--emit-sample"])
        .arg(&sample)
        .current_dir(dir.path())
        .assert()
        .success();
    let raw = std::fs::read_to_string(&sample).unwrap();
    (dir, raw)
}

#[test]
fn test_audit_confidence_lang_version_coverage_all_seven_languages() {
    // Per-fixture, parse each emitted JSONL record and assert
    // `lang_version` is the manifest-declared value (no `null`, no
    // drift). A `null` here is the detector regression signal: either
    // the dispatcher arm is unwired, the manifest reader returns
    // `None` for a shape it should handle, or the fixture's manifest
    // does not match what the doc claims.
    for &(lang, fixture, expected) in LANG_FIXTURES {
        let (_dir, raw) = run_audit_emit_for_fixture(fixture);
        let mut record_count = 0usize;
        for (i, line) in raw.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("[{lang}] line {} not valid JSON: {e}\n{line}", i + 1));
            assert_eq!(
                v["lang_version"],
                expected,
                "[{lang}] line {} expected lang_version={expected:?} (from fixture manifest); \
                 got {:?}",
                i + 1,
                v["lang_version"]
            );
            record_count += 1;
        }
        assert!(
            record_count > 0,
            "[{lang}] fixture {fixture} produced zero sampleable edges — \
             extend the fixture so audit emit has something to sample"
        );
    }
}
