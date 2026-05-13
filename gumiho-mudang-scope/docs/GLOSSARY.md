# Glossary

Central definitions for terms used across the documentation. Each entry: term + one-line definition + source doc.

When a term collides with a Rust crate name (`semver`, `tree-sitter`), the entry refers to the role it plays in Scope, not the crate's general semantics.

---

## Architecture

| Term | Definition | Source |
|---|---|---|
| **Class 1 / mechanical** | Architecture makes the violation impossible (does not compile or cannot be produced through the public API) | `ENFORCEMENT-MAP.md` § "Three classes of constraint" |
| **Class 2 / detectable** | Architecture allows compile but a test or audit catches the violation before merge | same |
| **Class 3 / discipline-only** | Cannot be prevented or detected by code; review and judgment catch it. Universal list is exactly three rules: B1, C2, E3 | same |
| **R-entry** | One enforcement entry (R0, R1, …) with ID, rules enforced, durable contract, where in the tree, CI gates. Stable anchor external docs deep-link to | `ENFORCEMENT-MAP.md` § "Rule enforcements" |
| **Hard limit** | Permanent out-of-scope rule; rejected without debate | `CHARTER.md` §5 |
| **Soft expansion zone** | Directions Scope may grow without breaking identity | `CHARTER.md` §6 |
| **3-question test** | Quick eligibility check (no toolchain / static second pass / preserves invariants) | `CHARTER.md` §4 |
| **4th question** | Priority booster: framework or domain semantics that LSP will never cover | `CHARTER.md` §4 |
| **Universal edge** | Edge kind every language plugin emits: `calls`, `imports`, `contains`, `references`, `references_type`, `extends`, `implements`, `instantiates` | `LANGUAGE-PLAYBOOK.md` Step 5 + R0 |
| **Domain edge** | Framework-specific edge (30 total R0). R0 baseline (13): `http_route`, `queue_handler`, `orm_relation`, `green_thread_spawn`, `renders`, `hook_use`, `inherits_from`, `migration`, `cron`, `feature_flag`, `awaits_on`, `channel_send`, `channel_recv`. Tier 1 (5): `middleware`, `validates_with`, `error_handler`, `websocket_handler`, `client_route`. Tier 2 (5): `auth_guard`, `cache_binding`, `runtime_task_spawn`, `route_mount`, `store_select`. Tier 3 (7): `sse_stream`, `signal_handler`, `cancel_token`, `lazy_load`, `query_binding`, `os_process_spawn`, `os_thread_spawn`. | `CHARTER.md` §6 + R0 |
| **Polyglot single graph** | All languages share one `symbols` / `edges` schema (charter invariant 4) | `CHARTER.md` §3 |
| **Resolution pass** | Stage that lifts `RawEdge` to `InsertableEdge` by assigning `status` based on workspace symbol-table lookup | R3 |
| **Typestate pipeline** | Extract → resolve → write enforced by Rust types: `RawCaptures` → `RawEdge` → `InsertableEdge` | R2 + R3 |

---

## Refactor types

| Term | Definition | Source |
|---|---|---|
| `RawCaptures` | Plugin output (R2): `{captures, metadata, skipped_ranges}` | R2 |
| `RawEdge` | Extractor output without `status`; produced by `EdgeBuilder` | R1 |
| `InsertableEdge` | Resolver output with `status` set; only type implementing `Insertable` | R3 |
| `EdgeBuilder` | Typestate builder; missing required field is a compile error; no `.status()` setter | R1 |
| `Edge` | Sealed struct; fields `pub(crate)`; constructors live inside `core::graph` only | R1 |
| `Capture` | Single `.scm` capture result inside `RawCaptures.captures` | R2 |
| `MetadataField` | Entry inside `RawCaptures.metadata`: decorator / annotation / template_call | R2 + R0 |
| `SkippedRange` | `{start_line, end_line, reason}` for partial-index recording | R0 + R6 |
| `Symbol` | Graph node; carries `metadata: TEXT JSON` column | `CHARTER.md` Appendix A + R0 |
| `Producer` | String identifier of producing plugin or layer (e.g., `rust_lang`, `python`, `framework:flask`, `resolution`) | R0 |
| `pattern_id` | Short slug naming the pattern that produced the edge (e.g., `calls.method`, `http_route.decorator_literal`) | R0 |
| `capture_id` | Tree-sitter capture name when applicable (`@call`, `@http_route`) | R0 |
| `Pattern` | Framework pattern struct: `{id, edge_kind, available_in, predicate}` | R5 |
| `Detection` | Framework detect output: `{detected, version, applies_to_languages}` | R5 |

---

## Workspace context

| Term | Definition | Source |
|---|---|---|
| `LanguageWorkspaceContext` | Trait visible to language plugins; deliberately omits version-coupled fields (mechanical safeguard for C2). The C2 line is **at this trait**, not at the workspace readers — readers in `scope-core/src/workspace/*.rs` live indexer-side and may expose version-extraction free functions for indexer-side consumers (R8 audit emit per sprint 0003 (d)) per the R4 indexer-side carveout | R4 |
| `FrameworkWorkspaceContext` | Trait visible to framework plugins; extends `LanguageWorkspaceContext` with `framework_versions()` and `lockfile()` | R4 |
| `WorkspaceContext` | Historical name; replaced by the R4 split. Do not introduce new uses | R4 |

---

## Versioning

| Term | Definition | Source |
|---|---|---|
| `DetectedVersion` | Outcome of reading workspace's framework version: `Resolved(semver::Version)` / `Indeterminate` / `NoVersionConcept` | R5 |
| `ResolvedVersion` | Version actually passed to `match_edges`: `Detected(v)` / `Fallback` / `Assumed(v)` / `Versionless` | R5 |
| `UnknownVersionPolicy` | Framework plugin's choice when version is `Indeterminate`: `Skip` / `StableOnlyLowConfidence` / `AssumeLatest(v)` | R5 + `FRAMEWORK-PLAYBOOK.md` Step 3 |
| `VersionReq` | `semver` crate's full-granularity version requirement; carried by each `Pattern.available_in` | R5 |
| `available_in` | `Pattern` field; `VersionReq` declaring where the pattern applies | R5 |
| Version coercion | Per-framework rule mapping non-strict-semver strings (Rails `7.0.4.3`, Python `3.11.0a1`) to `semver::Version` | R5 + `FRAMEWORK-PLAYBOOK.md` Step 3 |

---

## Confidence and status (orthogonal)

| Term | Definition | Source |
|---|---|---|
| `Confidence` | Pattern precision: `high` / `medium` / `low`; assigned by extractor; preserved by resolver | R0 + R3 |
| `status` | Lookup outcome: `resolved` / `ambiguous` / `dangling`; assigned by resolver only | R0 + R3 |
| Orthogonality | Confidence describes pattern precision; status describes lookup outcome; both columns queried independently | R3 + `LANGUAGE-PLAYBOOK.md` D2 |
| Cleanest-signal filter | `confidence='high' AND status='resolved'` | R3 |

---

## Schema

| Term | Definition | Source |
|---|---|---|
| `StatusData` | Struct returned by `scope status` (`src/commands/status.rs`) | R0 |
| `file_hashes.skipped_ranges` | JSON column with `[{start_line, end_line, reason}]` | R0 + R6 |
| Surrogate PK | `edges.edge_id INTEGER PRIMARY KEY AUTOINCREMENT`; replaces composite `(from_id, to_id, kind)` | R0 |
| `edges.args_text` | Optional TEXT NULL column carrying call-site / declaration-site argument literal capped at 2 KB. Mitigation 1: resolver skips fully-qualified targets. Mitigation 2: truncated literals end in `[truncated]`. Consumed by framework predicates and downstream cross-language stitching | R0 |
| Schema bumps | No in-place migration. Single-operator posture (`CHARTER.md` §2): old `.scope/` indexes are wiped (`rm -rf .scope/ && scope index`). Revisit only if external operation is ever entertained per CHARTER §2 | R0 |
| Labelled corpus | Committed regression assets at `gumiho-mudang-scope/scope-core/tests/fixtures/reference/<db_slug>/audit-samples/*.jsonl`; each file is a labelled `--emit-sample` output reproducing a precision baseline byte-for-byte | `AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy + `BACKLOG.md` § Priority 1 (e) |
| Provenance record | Append-only row in `<db_slug>/audit-samples/MANIFEST.md`; minimum fields `sample_file`, `labeller_id`, `labelled_at`, `scope_commit`, `sample_count`; machine-readable counterpart to the commit-message narrative | `AUDIT-LABEL-SCHEMA.md` § Corpus accumulation policy |
| `labeller_id` | String identifier of the labeller that produced an audit verdict (e.g., `lsp:rust-analyzer`, `llm:claude-sonnet-4-6`, `human:cferreir4`). Required on `schema_version: "2"` JSONL records; persists into the `edge_audit_history` table and is the join key for cross-labeller agreement views | `AUDIT-LABEL-SCHEMA.md` (schema v2) + `BACKLOG.md` § Priority 1 (g) + (i) + (j) |
| `edge_audit_history` | Append-only DB table introduced by sprint 0004 (BACKLOG (j)) — schema `(audit_id, edge_id, labelled_at, labeller_id, label, target_proposed, kind_proposed, confidence_proposed, evidence_json)`. Writable namespace **distinct from** source-derived tables (`edges`, `symbols`, `file_hashes`); auditor-immutability rule preserved by carving out the writable namespace explicitly. Indexed on `(edge_id, audit_id)` and `(labeller_id, audit_id)` | `BACKLOG.md` § Priority 1 (j) + `AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule (writable namespace) |
| `coverage_ratio` | Per-row field on the precision report (`schema_version: "2"`): `labelled_count / (labelled_count + skipped_count)`. Computed once at write time so consumers don't re-derive. Paired with `skipped_count` and `labelled_count` so an operator can distinguish "labeller covered every record" from "labeller covered N of M" at a glance | `BACKLOG.md` § Priority 1 (h) |
| `coverage_summary` | Top-level report object (`schema_version: "2"`): `{ records_total, records_labelled, records_skipped, distinct_groups_with_coverage, distinct_groups_fully_skipped }`. Single-glance view of overall labelling depth across the whole sample | `BACKLOG.md` § Priority 1 (h) |
| `agreement matrix` | NxN matrix where cell `(i, j)` = percentage of `(edge_id, audit_id)` tuples where labellers `i` and `j` produced the same `label`. Surfaced by `scope audit history agreement-matrix`. Sparse-to-empty under single-operator posture; sprint 0006 (i) ships the view when multi-labeller pipelines exist | `BACKLOG.md` § Priority 1 (i) + (j) |

---

## Plugin shapes

| Term | Definition | Source |
|---|---|---|
| `LanguageId` | Enum in `scope-core/src/languages/id.rs`: `TypeScript`, `CSharp`, `Python`, `Go`, `Java`, `Rust`, `Ruby`. JavaScript is not a variant today. The enum **carries all per-language behaviour** via inherent methods — there is no `trait LanguagePlugin` and no `*Plugin` unit structs (per [R7](ENFORCEMENT-MAP.md#r7--indexer-level-dispatch-enforcement)). `LanguageId::extract_metadata`, `extract_edge`, `ts_language`, etc. are exhaustive matches that delegate to per-language modules. `as_str()` returns canonical lowercase slugs (`"typescript"`, …) used by database text columns and log output | R7 + `FRAMEWORK-PLAYBOOK.md` § "Language scope" |
| `FrameworkPlugin` | Trait owned by R5; consumes `&[Symbol]` and `&[Edge]`, never AST | R5 |
| `Extractor` | Layer that converts `RawCaptures` to `EdgeBuilder` calls (R2) | R2 |
| `EdgeKind` | Closed whitelist; R0 = 38 kinds (8 universal + 30 domain across R0 baseline + Tier 1 + Tier 2 + Tier 3) | R0 |
| 4-kind concurrency split | `os_process_spawn` / `os_thread_spawn` / `green_thread_spawn` / `runtime_task_spawn`; producer-side plugin picks one based on stack ownership + scheduler + sync-block safety, not surface API spelling | R0 |
| `kind` (symbols) | Closed whitelist; R0 = 13 kinds (10 legacy + `macro`, `module`, `trait`) | R0 |
| Reserved metadata keys | `decorators`, `annotations`, `template_calls`; populated by language plugin, consumed by framework plugin. All three template-system-agnostic — `template_calls` covers JSX, ERB partials, Jinja includes, HEEx components, etc. | R0 + R5 |

---

## Process

| Term | Definition | Source |
|---|---|---|
| Trigger | Logged friction event; one per real incident (Path A only) | `LANGUAGE-PLAYBOOK.md` Step 1 + `FRAMEWORK-PLAYBOOK.md` Step 1 |
| Trigger threshold | Path A gate: 3+ in 30 days (frameworks); 5+ in 60 days (languages) → moves to evaluation | playbooks |
| Path A / Trigger-driven adoption | Adoption path requiring trigger log to reach threshold; for candidates of uncertain value | playbooks Step 1 |
| Path B / Maintainer-asserted adoption | Adoption path for daily-driver languages/frameworks already in active maintainer projects; skips trigger log; ROI worksheet still required | playbooks Step 1 |
| ROI worksheet | Quantified evaluation: annual savings vs total cost; non-negotiable on either path | playbooks Step 2 |
| Verdict | `BUILD` / `DEFER` / `REJECT` | playbooks Step 2 |
| Strategy | Framework-level **version** strategy: `A (latest only)` / `B (multi-version)` / `C (decline)`. Distinct from adoption Path A/B above | `FRAMEWORK-PLAYBOOK.md` Step 3 |
| Depth target | Language-level: `surface` / `depth` | `LANGUAGE-PLAYBOOK.md` Step 3 |
| Promotion | Surface → depth language transition; requires triggers + amendment | `LANGUAGE-PLAYBOOK.md` Step 3 |
| Demotion | Depth → surface; freezes existing depth items | `LANGUAGE-PLAYBOOK.md` Step 3 |
| Sunset | Plugin retired; archived path; existing indices retain edges | `LANGUAGE-PLAYBOOK.md` Step 8 + `FRAMEWORK-PLAYBOOK.md` Step 7 |
| Trigger-gated | Adoption requires triggers reaching threshold; never speculative | both playbooks |

---

## Subcommands

| Term | Definition | Source |
|---|---|---|
| `scope status` | Reports index health | R0 |
| `scope index` | Builds `.scope/`; `--watch` mode polls filesystem | `CHARTER.md` §3 |
| `scope audit confidence` | Precision report per `(kind, tier, producer, pattern_id)`; not recall | R8 |
| `scope audit coverage` (planned) | Recall-side report: edges emitted per pattern per fixture | `BACKLOG.md` |
| `scope audit history` | Read-side surface over `edge_audit_history`. Sprint 0004 ships three forms: no-subcommand (aggregate dashboard — headline + top-N regressing patterns + top-N flapping edges), `edge <edge_id>` (per-edge timeline), `pattern <pattern_id>` (precision-over-time + culprit edges). Sprint 0006 (i) adds `labeller <id>` (per-labeller timeline) and `agreement-matrix` (cross-labeller NxN) when multi-labeller density makes them meaningful | `BACKLOG.md` § Priority 1 (j) + (i) |
| `scope link` (planned) | Cross-project edges; mono-repo / microservice graph | `CHARTER.md` §6 |
| `scope diff --ref main` | Git-aware diff query | `CHARTER.md` §8 |
| `scope find` | Intent search (FTS5 + planned vectors) | `CHARTER.md` §6 + §8 |
| `scope query` | Read-only graph query | `CHARTER.md` §3 |

---

## CI gates

| Term | Definition | Source |
|---|---|---|
| Gate | Mechanical or test-based check that blocks merge on violation | `CI-GATES.md` |
| Gate status | `planned` (spec'd, not implemented) / `active` (CI runs it, blocks) / `disabled` (bypassed with rationale) | `CI-GATES.md` |
| Allowlist tag | Comment immediately preceding a call site that whitelists it for an audit script: `// scope:audit-allow <kind> — <rationale>` | `CI-GATES.md` § "Allowlist convention" |

---

## Discipline labels

| Term | Definition | Source |
|---|---|---|
| `NEEDS REVIEW` | Placeholder in a per-instance compliance log; ship-blocker | language and framework templates |
| `mechanically enforced` | Compliance achieved by an R-move's architecture | language template Step 4 |
| `compliant by design` | Tempting shortcut explicitly rejected; logged in "Rejected approaches" | language and framework templates |
| `trivially compliant` | Plugin does not even attempt the forbidden behavior | language template |

---

## Out-of-scope-permanent (handy shortcuts)

| Term | Why it is out | Source |
|---|---|---|
| Type inference | Requires per-language type system | `CHARTER.md` §5 |
| Macro expansion | Requires per-language macro engine | `CHARTER.md` §5 |
| Borrow / lifetime analysis | Requires Rust compiler frontend | `CHARTER.md` §5 |
| Conditional / mapped type evaluation | Requires TS type checker | `CHARTER.md` §5 |
| Metaclass / monkey-patching resolution | Inherently runtime | `CHARTER.md` §5 |
| Rename refactor with semantic guarantees | Requires exact reference set + type system | `CHARTER.md` §5 |
| Type / borrow / lint diagnostics | Compiler / linter territory | `CHARTER.md` §5 |
| Editor-buffer state | Requires daemon | `CHARTER.md` §5 |
| Network at query time | Breaks determinism, sandbox, offline use | `CHARTER.md` §5 + R12 |

These exist as a quick-lookup; the authoritative list is `CHARTER.md` §5.
