//! R6 — Malformed-source resilience harness.
//!
//! Walks every fixture under `tests/fixtures/malformed/<lang>/<case>/`
//! and asserts the four R6 acceptance contracts (see
//! `docs/ARCHITECTURAL-REFACTOR.md` § R6):
//!
//! 1. No panic on any fixture.
//! 2. The parseable prefix produces ≥ 1 symbol.
//! 3. `file_hashes.skipped_ranges` is non-empty when the file is
//!    partially malformed (silent drops are no longer acceptable).
//! 4. The recorded reason + range are pinned per fixture via `insta`
//!    snapshots so future regressions surface as snapshot diffs.
//!
//! Wired into CI by `just test-malformed` (`CI-GATES.md § Malformed-source harness`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use scope_core::extract::SkippedRange;
use scope_core::parser::CodeParser;

/// Snapshot-friendly mirror of `SkippedRange`. The production type
/// isn't pubic-derive-`Debug`-pretty in a way that snapshots cleanly;
/// this 3-field projection is what the harness pins.
#[derive(Debug)]
#[allow(dead_code)]
struct SkippedRangeRepr {
    reason: String,
    start_line: u32,
    end_line: u32,
}

/// Repo-relative path to the malformed fixture corpus.
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("malformed")
}

/// Returns `(lang_slug, case_name, fixture_dir)` for every fixture
/// directory, sorted for deterministic test ordering.
fn list_fixture_dirs() -> Vec<(String, String, PathBuf)> {
    let root = fixture_root();
    let mut out: Vec<(String, String, PathBuf)> = Vec::new();
    for lang_entry in fs::read_dir(&root).expect("read malformed/ root") {
        let lang_entry = lang_entry.expect("read malformed/ entry");
        let lang_path = lang_entry.path();
        if !lang_path.is_dir() {
            continue;
        }
        let lang_slug = lang_path
            .file_name()
            .expect("lang dir has name")
            .to_str()
            .expect("lang slug is utf8")
            .to_string();
        for case_entry in fs::read_dir(&lang_path).expect("read lang dir") {
            let case_entry = case_entry.expect("read case entry");
            let case_path = case_entry.path();
            if !case_path.is_dir() {
                continue;
            }
            let case_name = case_path
                .file_name()
                .expect("case dir has name")
                .to_str()
                .expect("case name is utf8")
                .to_string();
            out.push((lang_slug.clone(), case_name, case_path));
        }
    }
    out.sort();
    out
}

/// The fixture dir contains exactly one `source.<ext>` file alongside
/// `expected.md`. Locate it.
fn find_source_file(dir: &Path) -> PathBuf {
    for entry in fs::read_dir(dir).expect("read fixture dir") {
        let entry = entry.expect("read entry");
        let path = entry.path();
        let name = path
            .file_name()
            .expect("entry has name")
            .to_str()
            .expect("name is utf8");
        if name.starts_with("source.") {
            return path;
        }
    }
    panic!("no source.* file in {}", dir.display());
}

/// Mechanical 5-fixture floor per language — `R6 acceptance` deliverable.
/// Category selection is editorial (recorded in each fixture's
/// `expected.md`), but the floor itself is mechanical.
#[test]
fn floor_5_fixtures_per_language() {
    let fixtures = list_fixture_dirs();
    let mut per_lang: BTreeMap<String, usize> = BTreeMap::new();
    for (lang, _, _) in &fixtures {
        *per_lang.entry(lang.clone()).or_default() += 1;
    }
    let required = [
        "csharp",
        "go",
        "java",
        "python",
        "ruby",
        "rust",
        "typescript",
    ];
    for slug in required {
        let count = per_lang.get(slug).copied().unwrap_or(0);
        assert!(
            count >= 5,
            "{slug} has only {count} malformed fixtures; R6 mandates 5+ per language"
        );
    }
}

/// Every fixture must pass the four R6 acceptance contracts. The body
/// of this test deliberately accumulates failures across all fixtures
/// before panicking — a single bad fixture should surface every
/// fixture's status in the failure output, not stop at the first.
#[test]
fn every_fixture_satisfies_r6_acceptance() {
    let mut parser = CodeParser::new().expect("create CodeParser");
    let fixtures = list_fixture_dirs();
    assert!(!fixtures.is_empty(), "no fixtures discovered");

    let mut failures: Vec<String> = Vec::new();

    for (lang_slug, case, dir) in &fixtures {
        let source_path = find_source_file(dir);
        let source = match fs::read_to_string(&source_path) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!(
                    "{lang_slug}/{case}: read {} failed: {e}",
                    source_path.display()
                ));
                continue;
            }
        };
        let lang = match CodeParser::detect_language(&source_path) {
            Ok(l) => l,
            Err(e) => {
                failures.push(format!(
                    "{lang_slug}/{case}: detect_language {} failed: {e}",
                    source_path.display()
                ));
                continue;
            }
        };
        let file_path = format!(
            "{}/{}/{}",
            lang_slug,
            case,
            source_path
                .file_name()
                .expect("source has name")
                .to_str()
                .expect("source name utf8")
        );

        // Contract 1 — no panic. `Result` return suffices; production
        // code must not panic on any malformed input.
        let symbols = match parser.extract_symbols(&file_path, &source, lang) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!(
                    "{lang_slug}/{case}: extract_symbols errored: {e}"
                ));
                continue;
            }
        };
        let ranges: Vec<SkippedRange> =
            match parser.collect_skipped_ranges(&file_path, &source, lang) {
                Ok(v) => v,
                Err(e) => {
                    failures.push(format!(
                        "{lang_slug}/{case}: collect_skipped_ranges errored: {e}"
                    ));
                    continue;
                }
            };

        // Contract 2 — ≥ 1 symbol from parseable prefix.
        if symbols.is_empty() {
            failures.push(format!(
                "{lang_slug}/{case}: extract_symbols returned 0 symbols; \
                 every fixture's parseable prefix must emit ≥ 1 symbol \
                 (charter §3 inv 5 — useful-if-incomplete index)"
            ));
        }

        // Contract 3 — skipped_ranges non-empty when the file is
        // partially malformed.
        if ranges.is_empty() {
            failures.push(format!(
                "{lang_slug}/{case}: skipped_ranges empty; a partially-\
                 malformed file must record at least one skip — silent \
                 drops are no longer acceptable"
            ));
        }

        // Contract 4 — snapshot the reason + range per fixture.
        // Snapshot path lives under `tests/snapshots/malformed_sources/`
        // following insta defaults; one snapshot file per fixture.
        // `assert_debug_snapshot!` is the default-features-only insta
        // macro (no `yaml` / `json` feature needed); the debug format of
        // `Vec<SkippedRangeRepr>` is stable and reviewer-friendly.
        let snapshot_data: Vec<SkippedRangeRepr> = ranges
            .iter()
            .map(|r| SkippedRangeRepr {
                reason: r.reason.clone(),
                start_line: r.start_line,
                end_line: r.end_line,
            })
            .collect();
        let snapshot_name = format!("{lang_slug}__{case}");
        insta::with_settings!({
            snapshot_path => "snapshots/malformed_sources",
            prepend_module_to_snapshot => false,
        }, {
            insta::assert_debug_snapshot!(snapshot_name, snapshot_data);
        });
    }

    if !failures.is_empty() {
        panic!(
            "{} R6 acceptance failure(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
