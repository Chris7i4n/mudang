//! JSONL read / write helpers for the v2 contract.
//!
//! Read semantics (per `AUDIT-LABEL-SCHEMA.md` § File format):
//! - One JSON object per line.
//! - Empty lines and lines starting with `#` are skipped.
//! - Each parsed record's `schema_version` must equal [`SCHEMA_VERSION`];
//!   any other value is rejected with [`ParseError::UnknownSchemaVersion`],
//!   matching the diagnostic Scope's CLI emits in `label_pass`.
//!
//! Write semantics:
//! - One record per line; no trailing whitespace except the newline.
//! - Field order is the struct's declaration order, which matches the
//!   schema doc § Record schema table — a test in this module locks it.

use std::io::{self, BufRead, Write};

use crate::record::{SampleRecord, SCHEMA_VERSION};

/// Reading-side error surface.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("line {line}: io error: {source}")]
    Io {
        line: usize,
        #[source]
        source: io::Error,
    },
    #[error("line {line}: invalid JSON: {source}")]
    Json {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "line {line}: unknown schema_version {got:?}; expected {expected:?}. \
         Per AUDIT-LABEL-SCHEMA.md § Versioning rules, the remediation is \
         wipe corpus + reindex + re-emit + re-label."
    )]
    UnknownSchemaVersion {
        line: usize,
        got: String,
        expected: &'static str,
    },
}

/// Streaming iterator over the records of a JSONL sample file.
///
/// Each item is `Result<SampleRecord, ParseError>`. The iterator continues
/// past parse errors so the caller can drive a best-effort labelling pass;
/// a stricter caller may stop on the first error.
pub struct RecordIter<R: BufRead> {
    reader: R,
    line_no: usize,
    buf: String,
}

impl<R: BufRead> Iterator for RecordIter<R> {
    type Item = Result<SampleRecord, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.buf.clear();
            let read = match self.reader.read_line(&mut self.buf) {
                Ok(n) => n,
                Err(source) => {
                    let line = self.line_no + 1;
                    self.line_no = line;
                    return Some(Err(ParseError::Io { line, source }));
                }
            };
            if read == 0 {
                return None;
            }
            self.line_no += 1;
            let line_no = self.line_no;

            let trimmed = self.buf.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let record: SampleRecord = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(source) => return Some(Err(ParseError::Json { line: line_no, source })),
            };
            if record.schema_version != SCHEMA_VERSION {
                return Some(Err(ParseError::UnknownSchemaVersion {
                    line: line_no,
                    got: record.schema_version,
                    expected: SCHEMA_VERSION,
                }));
            }
            return Some(Ok(record));
        }
    }
}

/// Stream records from a JSONL source.
pub fn read_records<R: BufRead>(reader: R) -> RecordIter<R> {
    RecordIter { reader, line_no: 0, buf: String::new() }
}

/// Append one record as a single JSONL line followed by `\n`.
pub fn write_record<W: Write>(mut writer: W, record: &SampleRecord) -> io::Result<()> {
    serde_json::to_writer(&mut writer, record).map_err(io::Error::other)?;
    writer.write_all(b"\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn example_v2_line() -> &'static str {
        r#"{"schema_version":"2","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":"2021","label":null,"evidence":null,"target_proposed":null,"kind_proposed":null,"confidence_proposed":null,"reasoning_text":null,"lang_version_evidence":null,"labeller_id":null}"#
    }

    #[test]
    fn reads_example_v2_line() {
        let cursor = Cursor::new(example_v2_line().as_bytes().to_vec());
        let mut iter = read_records(cursor);
        let rec = iter.next().unwrap().expect("parse");
        assert_eq!(rec.schema_version, "2");
        assert_eq!(rec.edge_id, "e-9f2c");
        assert_eq!(rec.kind, "calls");
        assert_eq!(rec.from_id, "crate::handlers::greet");
        assert_eq!(rec.to_id, "crate::utils::format_name");
        assert_eq!(rec.lang_version.as_deref(), Some("2021"));
        assert_eq!(rec.label, None);
        assert!(iter.next().is_none());
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let bad = example_v2_line().replace("\"2\"", "\"1\"");
        let cursor = Cursor::new(bad.into_bytes());
        let mut iter = read_records(cursor);
        let err = iter.next().unwrap().unwrap_err();
        match err {
            ParseError::UnknownSchemaVersion { line, got, expected } => {
                assert_eq!(line, 1);
                assert_eq!(got, "1");
                assert_eq!(expected, "2");
            }
            other => panic!("expected UnknownSchemaVersion, got {other:?}"),
        }
    }

    #[test]
    fn skips_blank_and_comment_lines() {
        let content = format!(
            "# header comment\n\n{}\n#trailing comment\n",
            example_v2_line()
        );
        let cursor = Cursor::new(content.into_bytes());
        let records: Vec<_> = read_records(cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].edge_id, "e-9f2c");
    }

    #[test]
    fn write_record_preserves_field_order() {
        let cursor = Cursor::new(example_v2_line().as_bytes().to_vec());
        let rec = read_records(cursor).next().unwrap().expect("parse");
        let mut out = Vec::<u8>::new();
        write_record(&mut out, &rec).expect("write");
        let written = String::from_utf8(out).expect("utf8");
        // Schema-doc canonical order: schema_version first, then edge_id, kind,
        // confidence, producer, pattern_id, from, to, source_snippet,
        // lang_version, label, evidence, target_proposed, kind_proposed,
        // confidence_proposed, reasoning_text, lang_version_evidence,
        // labeller_id.
        let expected_prefix = r#"{"schema_version":"2","edge_id":"e-9f2c","kind":"calls","confidence":"high","producer":"rust","pattern_id":"rust.calls.method","from":"crate::handlers::greet","to":"crate::utils::format_name","source_snippet":"format_name(&user.name)","lang_version":"2021","label":null,"evidence":null,"target_proposed":null,"kind_proposed":null,"confidence_proposed":null,"reasoning_text":null,"lang_version_evidence":null,"labeller_id":null}"#;
        assert_eq!(written.trim_end(), expected_prefix);
        assert!(written.ends_with('\n'));
    }

    #[test]
    fn round_trip_byte_for_byte() {
        let cursor = Cursor::new(example_v2_line().as_bytes().to_vec());
        let rec = read_records(cursor).next().unwrap().expect("parse");
        let mut out = Vec::<u8>::new();
        write_record(&mut out, &rec).expect("write");
        let written = String::from_utf8(out).expect("utf8");
        assert_eq!(written.trim_end(), example_v2_line());
    }

    #[test]
    fn rejects_non_object_evidence() {
        // The v2 schema requires `evidence: object | null`. The struct's
        // type-level constraint (`Option<serde_json::Map<String, Value>>`)
        // makes any other JSON shape unparseable. A labeller built on this
        // core cannot accidentally emit records `--label` rejects.
        for bad_shape in [r#""checked""#, r#"["a","b"]"#, r#"42"#, r#"true"#] {
            let bad = example_v2_line()
                .replace(r#""evidence":null"#, &format!(r#""evidence":{bad_shape}"#));
            let cursor = Cursor::new(bad.into_bytes());
            let err = read_records(cursor).next().unwrap().unwrap_err();
            match err {
                ParseError::Json { line, .. } => assert_eq!(line, 1),
                other => panic!("expected Json error for evidence={bad_shape}, got {other:?}"),
            }
        }
    }

    #[test]
    fn accepts_object_evidence() {
        let labelled = example_v2_line().replace(
            r#""evidence":null"#,
            r#""evidence":{"resolver":"rust-analyzer","target_uri":"file:///x"}"#,
        );
        let cursor = Cursor::new(labelled.into_bytes());
        let rec = read_records(cursor).next().unwrap().expect("parse");
        let ev = rec.evidence.expect("evidence present");
        assert_eq!(ev.get("resolver").and_then(|v| v.as_str()), Some("rust-analyzer"));
        assert_eq!(ev.get("target_uri").and_then(|v| v.as_str()), Some("file:///x"));
    }

    #[test]
    fn parse_error_carries_line_number() {
        let content = format!("{}\nnot-json\n", example_v2_line());
        let cursor = Cursor::new(content.into_bytes());
        let mut iter = read_records(cursor);
        let _ok = iter.next().unwrap().expect("first ok");
        let err = iter.next().unwrap().unwrap_err();
        match err {
            ParseError::Json { line, .. } => assert_eq!(line, 2),
            other => panic!("expected Json error on line 2, got {other:?}"),
        }
    }
}
