//! v2 `SampleRecord` shape per `AUDIT-LABEL-SCHEMA.md` § Record schema.
//!
//! Field declaration order matches the schema doc table so the default
//! `serde_json::to_string` serialisation emits the canonical column ordering
//! the schema doc commits to. Tests in `io::tests` lock this ordering.

use serde::{Deserialize, Serialize};

/// The single accepted JSONL schema version. Per single-operator posture
/// (`CHARTER.md` § 3 invariant 1) `--label` rejects every other value with
/// the same "unknown schema_version" diagnostic that fires on a forward
/// bump. The remediation is wipe-and-reindex + re-emit.
pub const SCHEMA_VERSION: &str = "2";

/// One row of the JSONL sample file.
///
/// Two of the schema columns are Rust keywords (`from`, `to`) — serde
/// renames them to the wire names; the struct fields are named `from_id` /
/// `to_id` to match Scope-side terminology elsewhere in the codebase.
///
/// All seven labeller-fillable columns (`evidence`, `target_proposed`,
/// `kind_proposed`, `confidence_proposed`, `reasoning_text`,
/// `lang_version_evidence`, `labeller_id`) are `Option`-typed with `null`
/// on emit; capable labellers populate them. Partial population is
/// tolerated per the schema doc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SampleRecord {
    pub schema_version: String,
    pub edge_id: String,
    pub kind: String,
    pub confidence: String,
    pub producer: String,
    pub pattern_id: String,
    #[serde(rename = "from")]
    pub from_id: String,
    #[serde(rename = "to")]
    pub to_id: String,
    pub source_snippet: String,
    pub lang_version: Option<String>,
    pub label: Option<bool>,
    pub evidence: Option<serde_json::Value>,
    pub target_proposed: Option<String>,
    pub kind_proposed: Option<String>,
    pub confidence_proposed: Option<String>,
    pub reasoning_text: Option<String>,
    pub lang_version_evidence: Option<String>,
    pub labeller_id: Option<String>,
}
