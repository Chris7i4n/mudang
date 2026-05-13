# Post-Refactor Plan

Work queued against the current architecture. [`ENFORCEMENT-MAP.md`](ENFORCEMENT-MAP.md) holds the rule→implementation map; the items below are queued for delivery in priority order. Each item additionally respects its own gate — language depth follows `LANGUAGE-PLAYBOOK.md` adoption flow; framework adoption follows `FRAMEWORK-PLAYBOOK.md` triggers.

---

## Eligibility

The architecture is stable and every bullet below holds:

- Every universal rule in the inventory tables (`CHARTER.md` §5 hard limits and `LANGUAGE-PLAYBOOK.md` Step 4) is in class 1 (mechanical), class 2 (detectable), or the explicit class-3 universal list (B1, C2, E3).
- Every active language plugin's `docs/languages/<name>.md` has zero `NEEDS REVIEW` entries.
- `scope audit confidence` runs against the reference fixture corpus.
- CI gates active: malformed-source (R6), trait-shape audit + spawn-denylist (R12), immutable-source (R9), plus every other gate in [`CI-GATES.md`](CI-GATES.md).
- Full benchmark suite shows < 10% regression from pre-refactor baseline.

Items below are ordered by priority. Each respects its own per-item gate stated under "Gate to start".

---

## Priority 1 (immediately post-refactor) — Self-correction cycle

R8 (sprint 0007) ships the **sensor**: `scope audit confidence` measures per-`(producer, pattern_id)` precision against a labelled fixture corpus and fails the build when any tier is below target. The JSONL sample format ([`AUDIT-LABEL-SCHEMA.md`](AUDIT-LABEL-SCHEMA.md)) is the contract that lets any external labeller (LLM, LSP cross-check, hybrid) plug in.

R8 alone does **not** close the loop. When a tier falls below target, a human still has to read the labelled samples, find the failing pattern, and patch the extractor by hand. The next index run then emits corrected edges (existing persisted edges are not retroactively fixed — `wipe-and-reindex` per CHARTER §2 is the migration path).

This priority-1 work ships the **actuator** — the closed loop that converts R8 signal into automated extractor improvement. It is **first** in this document because every other item below assumes precision is trustworthy; without the loop, precision drift is found late and fixed by hand.

### Sub-items (no internal ordering — each unblocks the others)

- **(a) Loop architecture document.** A new `docs/SELF-CORRECTION-CYCLE.md` formalising the pipeline: `R8 audit signal → labelled corpus → analyzer (ML / LLM / heuristic) → extractor patch suggestion → human review → merge → next index run`. Names the contract surfaces, the human gate, the rollback path when an analyzer-suggested patch regresses precision elsewhere.
- **(b) Reference labeller crates.** External-to-Scope crates that implement the JSONL contract: `scope-audit-labeller-llm` (provider-agnostic LLM wrapper), `scope-audit-labeller-lsp` (per-language LSP cross-check via `tower-lsp` clients), `scope-audit-labeller-hybrid` (LLM-first, human-reviews-diffs). These live in a separate workspace so Scope's surface stays minimal; they consume only the schema.
- **(c) Continuous re-audit in CI.** Per-PR run of `scope audit confidence --label committed-sample.jsonl` against the committed labelled corpus, with a precision diff printed in the PR body (`rust.calls.method: 96% → 94% (-2pp, 2 new failures)`). Catches extractor regressions before merge. Sample size capped for CPU budget; full audit still runs nightly.
- **(d) Per-language `lang_version` detector matrix.** Populate the `lang_version` JSONL slot atomically across all seven supported languages: Rust (edition + rust-version from `Cargo.toml` — module exists), Go (`go.mod` directive — module exists), Python (`requires-python` from `pyproject.toml` / `setup.py`), TypeScript (`tsconfig.json` `target`), Java (Maven `<source>/<target>` + Gradle source compatibility — two build systems), C# (`<TargetFramework>` from `.csproj`), Ruby (`.ruby-version` + Gemfile `ruby` directive). Sprint 0007 emits `null` for every language; this sub-item turns all seven on in a single delivery so the labelled corpus does not split into a "versioned" and "unversioned" era. Per-language detector wiring is workspace-side only and does not violate R4's `LanguageWorkspaceContext` shape (these stay off the plugin trait surface).
- **(e) Labelled corpus accumulation policy.** Committed `*.jsonl` files under `scope-core/tests/fixtures/reference/<lang>/audit-samples/` are regression assets. Sample provenance recorded per-commit (labeller used, date). Old samples are kept until the underlying fixture is removed — stable precision over time is itself the signal.
- **(f) ML-driven extractor patch suggester.** Long-horizon: an analyzer that reads labelled failures, locates the offending pattern in the extractor source, and proposes a code patch (branch on an AST shape, downgrade the confidence stamp, add a guard). The human review gate stays mandatory — this is a suggester, not an applier. Triggers when the labelled corpus is large enough to train a meaningful model (heuristic: 1000+ samples across ≥4 languages).
- **(g) Richer auditor verdict types — JSONL `schema_version` bump `"1"` → `"2"`.** Sprint 0007 locked the JSONL sample-row at `schema_version: "1"` with a binary `label: bool | null` verdict. That surface is **minimum-viable for the plug-point only**: a binary correct/wrong/skipped is information-poor signal — a labeller saying "wrong" tells you nothing about *why* it is wrong, and "skipped" tells you nothing about *what* was actually true. The actionable signal for closing the self-correction loop is **qualitative**: the auditor saying *"Scope claimed X, here is the evidence that Y is the truth"* — proposed corrected target, proposed corrected kind, proposed corrected confidence tier, free-text reasoning, cross-check evidence. The schema-bump-2 record adds (each `null` on emit, populated by capable labellers; partial population tolerated):
  - **`evidence: object | null`** — labeller-supplied structured evidence behind the verdict. Schema is labeller-defined; conventional keys: `{"resolver": "rust-analyzer", "target_uri": "...", "definition_range": [...]}` for LSP cross-check; `{"model": "claude-sonnet-4-6", "reasoning": "...", "prompt_hash": "..."}` for LLM. The auditor records the *how*, not just the *what*.
  - **`target_proposed: string | null`** — labeller's correction for `to`. *"Scope said `to = foo::bar`; I see this call resolves to `foo::baz` instead."* This is the actionable diff: feeds the ML-driven patch suggester ((f)) to localise the extractor bug.
  - **`kind_proposed: string | null`** — labeller's correction for `kind`. *"Scope said `references_type`; this is actually `calls` (the call expression is parenthesised after the identifier)."*
  - **`confidence_proposed: string | null`** — labeller's correction for `confidence`. *"Scope said `high`; I see overload ambiguity here, this should be `medium`."* Distinct from a binary "wrong" verdict: the labeller may agree the edge is correct but say the confidence stamp is overstated.
  - **`reasoning_text: string | null`** — free-text human (or LLM) explanation. The post-hoc audit trail when a `false` verdict is reviewed months later.
  - **`lang_version_evidence: string | null`** — for distinguishing detected vs declared version (already named in `AUDIT-LABEL-SCHEMA.md` § Reserved-for-future fields).

  The bump lands together with sub-item (h) (coverage surfacing on the report side) and sub-item (j) (DB storage shape), because all three are entry points for the same qualitative-signal surface. `schema_version: "1"` labellers continue to work — `--label` accepts both versions, treating the new fields as `null` when absent. Removing or repurposing existing `"1"` fields is still charter-grade.

- **(h) Per-group coverage surfaced on the precision report.** Sprint 0007 ships `--label` with a hard rule: precision is computed over labelled records only (`label = true | false`), records with `label = null` are skipped. The report's `sample_size` per group is therefore the labelled count — the precision denominator. But the report does **not** today surface *how many records were skipped*. An operator reading `calls/high/rust/method: precision=1.0, sample_size=15` cannot tell whether the labeller covered every record (transparent) or only 15 out of 45 (opaque — 2/3 unaudited). That gap is acknowledged in the report header (see `COVERAGE_LIMITATION_NOTE` in `gumiho-mudang-cli/src/commands/audit.rs` and the inline comment on `compute_precision_report`); it closes here. The schema-bump-2 report adds:
  - **`skipped_count: usize`** per row — number of records in this group whose label was `null`.
  - **`labelled_count: usize`** per row — explicit alias for `sample_size`, kept side-by-side with `skipped_count` so the report is self-documenting (no need to compute `total = sample_size + skipped_count` mentally).
  - **`coverage_ratio: f64`** per row — `labelled_count / (labelled_count + skipped_count)`. Computed once at write time so consumers don't re-derive.
  - **Top-level `coverage_summary: { records_total: usize, records_labelled: usize, records_skipped: usize, distinct_groups_with_coverage: usize, distinct_groups_fully_skipped: usize }`** — single-glance view of overall labelling depth across the whole sample.

  Honesty implication: paired with (g)'s richer verdict types, the operator finally sees the **full** signal — what Scope claimed, what the labeller could and could not judge, and where it disagreed with reasons.

- **(i) Multi-labeller verdict aggregation.** Realistic labelling pipelines run several labellers in series or parallel (LSP fast-path for `calls`, LLM for everything else, human reviewer for diffs) and produce conflicting verdicts on the same `edge_id`. Sprint 0007 has no aggregation surface — each `--label` invocation consumes one file. (i) designs:
  - **Multi-source JSONL format** — multiple labellers' outputs concatenated or merged, each record carrying a new `labeller_id: string` field (added in (g)'s schema bump) identifying which labeller produced the verdict.
  - **Aggregation policy** — when LSP says `true`, LLM says `false`, human says `null`, what does the precision report say for that edge? Options:
    - Priority order (e.g., human overrides LLM overrides LSP)
    - Quorum (n-of-m agree → use that; disagreement → flag for review)
    - Per-labeller confidence weight (`labeller_id` → trust score, applied to `confidence_proposed`)
    - Hybrid (use the LSP fast-path when available; fall back to LLM; defer to human on confirmed disagreement)
  - **Disagreement diagnostics** — when labellers disagree on `kind_proposed` or `target_proposed`, that disagreement is itself a precision-system signal worth surfacing (one labeller is wrong; or the edge is ambiguous; or Scope's stamp is the source of the confusion).
  - **Policy lives in the runner, not in Scope** — the labeller-crates ecosystem from (b) carries aggregation; Scope's `--label` reads the *aggregated* JSONL output. No new flag on the subcommand.

- **(j) Audit-history persistence in the DB — separate writable namespace, auditor immutability preserved.** Sprint 0007's `--label` consumes a JSONL file and emits a stdout/stdout report. Nothing of the audit result flows back to `graph.db`. That keeps the auditor-immutability rule absolute *for the source-derived tables* (`edges`, `symbols`, `file_hashes`) but means **historical audit data is not queryable**. A pattern that fails precision every Tuesday for three months produces three months of independent reports — Scope itself cannot say *"this pattern_id has trended downward over N audits"*. (j) designs the persistence layer:
  - **New table `edge_audit_history`** (or analogous) — schema: `(audit_id, edge_id, labelled_at, labeller_id, label, target_proposed, kind_proposed, confidence_proposed, evidence_json)`. Append-only; one row per (audit run × edge) tuple a labeller weighed in on. Indexed by `(edge_id, audit_id)` and `(labeller_id, audit_id)`.
  - **Sibling auditor-immutability rule** — the table is writable by `scope audit confidence --label` but *never* mutates `edges` / `symbols` / `file_hashes` rows. The source-derived schema stays frozen during audit; the audit-derived schema records what the auditor decided. Two distinct namespaces, two distinct enforcement gates.
  - **Auditor immutability rule extension** — `AUDIT-LABEL-SCHEMA.md` § Auditor immutability rule gets a new paragraph carving out the writable namespace explicitly so the read-only-on-source promise remains mechanically checkable.
  - **`scope audit history`** subcommand — read-side surface for the new table: per-edge audit timeline, per-pattern_id precision-over-time, per-labeller agreement matrix. Becomes the input for (f) (the patch suggester operates on history, not just a single audit run).
  - **Retention policy** — committed labelled samples under `tests/fixtures/reference/<lang>/audit-samples/` accumulate. `edge_audit_history` accumulates indefinitely too, unless a future sprint adds eviction. The bytes are cheap; the longitudinal signal is the value.

- **(k) Confidence re-stamping policy from accumulated audit signal.** Once (j) ships, the system *can* answer "this `(producer, pattern_id)` has consistently failed precision at the high tier over the last 30 audits". The natural next move: downgrade Scope's confidence stamp for that pattern_id. (k) is **the policy decision that gates the actuator turning on**:
  - **Automatic downgrade** — when N consecutive audits show precision < tier-target by some margin, the next index run stamps the edge `medium` instead of `high`. Pro: closes the loop fully. Con: indirection between extractor source code and emitted confidence — the extractor stops being the single source of truth for what a stamp means.
  - **Flag-for-review** — when the same threshold is met, the next audit report surfaces "pattern_id X is consistently sub-target; manual review recommended"; the human edits the extractor source and commits a downgrade. Pro: extractor source stays canonical. Con: human-in-the-loop on every regression; slow.
  - **Hybrid** — automatic downgrade for tier-internal moves (`high → medium`); manual-only for cross-tier (`medium → low` is downgrade but `low → medium` is upgrade and dangerous).
  - **Audit-trail invariant** — every automatic re-stamp is logged in a dedicated audit-trail file (named when this priority opens) so future maintainers can trace why a given edge in the index carries a confidence stamp different from what the extractor source naively produces. The audit trail is non-optional; without it, the loop becomes opaque.

  This policy is the riskiest piece in Priority 1 and ships last. Premature automation here can pollute the index with stamps that lag the actual extractor behaviour by audit-cycle epochs.

### Gate to start

Eligibility holds (architecture closed). R8 ships the sensor; the reference fixture corpus is committed; this priority builds the actuator on top.

---

## Priority 2 (immediately post-refactor) — Honesty audit: eliminate non-essential approximations

### Principle (charter-grade)

Scope is a code analyser. The quality and size of the code submitted to it can legitimately impact the **cost** of analysis — analysing a 1-megabyte SQL literal embedded in source costs more I/O than analysing a 50-byte one, full stop. But cost-driven choices may **never** introduce **silent inaccuracy** into the analysis itself. A code analyser that lies is worse than one that is slow: a slow analyser tells the truth eventually; a lying one corrupts every downstream decision (refactor, LSP, audit, agent reasoning).

The **only** acceptable trade-offs against fidelity are **hard runtime constraints**:

- The un-approximated version would **panic** (integer overflow, stack overflow, unwrap on `None` we cannot eliminate at the type level).
- The un-approximated version would **OOM** on realistic input (not on pathological input — on the kind of input we genuinely expect to handle).
- The un-approximated version would **not run at all** (filesystem, syscall, transport, OS-imposed bound).

Anything weaker than that — *"this might be slow on bad code"*, *"literals are usually small anyway"*, *"the truncated form is good enough for the common case"*, *"99% of cases fit in N bytes"* — is **not** a valid justification for sacrificing fidelity. Bad code is exactly the input Scope must analyse honestly.

This principle has been implicit throughout every refactor sprint (Charter, R-move acceptance bullets, sprint deliverables consistently chose fidelity over speed). Priority 2 makes it explicit and re-validates every prior choice against it.

### Known offender (Priority 2 sprint opens here)

- **R0 `edges.args_text` 2 KB cap** — see [`ENFORCEMENT-MAP.md` § R0 → Mitigation 2](ENFORCEMENT-MAP.md#r0--schema-closures) and the const `ARGS_TEXT_CAP_BYTES = 2048` in [`scope-core/src/edge.rs`](../scope-core/src/edge.rs). Truncating call-site / declaration-site argument literals at 2 KB plus a `[truncated]` marker is an approximation justified by *"common case fits"* — **not** by any hard runtime constraint. SQLite TEXT holds up to ~1 GB; long literals make Scope slower on pathological codebases but cannot panic, OOM, or fail to run. The fix is to drop the cap and the truncation marker; the single-operator wipe-and-reindex policy (CHARTER §2) absorbs the schema impact for existing local DBs.

### Sub-items (sequenced — (a) and (b) feed (c) and (d))

- **(a) Charter-grade audit.** Walk every R-move acceptance bullet, every schema comment, every doc rationale. Flag every use of the words *cap*, *truncate*, *limit*, *approximate*, *sample*, *heuristic*, *good enough*, *common case*, *roughly*. For each: is the trade-off justified by a hard runtime constraint (panic / OOM / won't-run)? If yes — leave it and surface the constraint verbatim in the doc. If no — queue for fix.
- **(b) Code-grade audit.** Grep every workspace crate for `const .*: usize = ` whose name contains `CAP`, `LIMIT`, `MAX`, `TRUNC`, `BUDGET`, `THRESHOLD`, or that is followed by truncation / sampling / fallback logic. Same triage as (a).
- **(c) Drop the R0 `args_text` 2 KB cap** (known offender above) **and bump the audit-sample JSONL `schema_version` from `"1"` to `"2"` adding a `producer_captured_args: string | null` field**. The two changes ship together because they are the same fidelity move from two angles: (c.i) the schema bump exposes what the extractor actually captured at index time as a first-class column in the JSONL sample, so an external labeller can compare current source against the index-time capture side-by-side instead of squinting at `args_text` through R8's source-file fallback; (c.ii) dropping the cap makes that index-time capture actually faithful (under the 2 KB cap, `producer_captured_args` would carry the truncated stub and inherit the lie). Bundled, the post-refactor Scope ships the auditor a complete two-source comparison: current source via `source_snippet`, index-time capture via `producer_captured_args`. One commit, charter-grade amendment on `main`:
  - Delete `ARGS_TEXT_CAP_BYTES`, `TRUNCATION_MARKER`, and the truncation logic in `scope-core/src/edge.rs`.
  - Delete the matching unit test (currently asserts the truncation byte length).
  - Update the schema comment in `scope-graph/src/sql/schema.sql` (drop "capped at 2 KB / truncation marker" text).
  - Update [`ENFORCEMENT-MAP.md` § R0 → Mitigation 2](ENFORCEMENT-MAP.md#r0--schema-closures) (replace with a note recording the original cap was dropped by Priority 2; honesty over performance; wipe policy stands).
  - Bump `schema_version` from `"1"` to `"2"` in [`AUDIT-LABEL-SCHEMA.md`](AUDIT-LABEL-SCHEMA.md), add the `producer_captured_args: string | null` record field with the auditor-comparison rationale, add a migration note. Update `--label` rejection logic so old `schema_version: "1"` samples error with a re-emit instruction.
  - Log entry in the priority-2 sprint's PR body documenting the amendment (paper-trail discipline).
- **(d) Fix any further offenders found by (a) + (b).** Each fix lands as its own charter-grade amendment with paper trail.
- **(e) Capture remaining justified approximations as explicit invariants.** Where (a) or (b) finds a trade-off that *is* justified by a hard runtime constraint, the constraint moves into the document as a first-class invariant (not a footnote). Future sprints know the line was drawn deliberately and where.

### Gate to start

Eligibility holds. Runs in parallel with Priority 1 — independent surfaces (Priority 1 builds the self-correction actuator on top of R8; Priority 2 audits the data the actuator measures). Neither blocks the other.

### Why this is **not** absorbed by Priority 1 (self-correction cycle)

Priority 1's labelling pipeline reads `source_snippet` directly from the source file at audit time — it deliberately sidesteps `args_text` precisely because R8's design recognised the approximation issue. So Priority 1 ships safely even before Priority 2 lands. But every **other** consumer of `args_text` (resolver, framework plugins, future LSP integration, time-travel queries) is still reading a possibly-truncated string. Priority 2 plugs the leak system-wide.

---

## Priority 3 (immediately post-refactor) — Layering audit: thin CLI, fat library

### Principle

The CLI crate (`gumiho-mudang-cli`) is a **presentation layer**: clap argument parsing, dispatch into the engine, and output formatting against the R10 typed schema. Domain logic — analysis algorithms, schema-versioned wire formats, mechanical invariants (drift gates, tier targets) — belongs in a scope sub-crate where it can be unit-tested in isolation, reused by future hosts (LSP, web service, batch CI tool), and audited against the architectural-refactor R-moves without the indirection of "look inside the CLI".

The split tracks the same charter discipline as the R-moves: each surface has one responsibility, the responsibility is named, and crossing the seam is mechanically detectable in review.

### Known offender (Priority 3 sprint opens here)

- **`gumiho-mudang-cli/src/commands/audit.rs` (~1400 LOC after sprint 0007)** — R8's entire engine lives in the CLI: `sample_stratified` + `xorshift64` PRNG (the sampling algorithm), `SampleRecord` + `PrecisionReport` + `ReportRow` + `ReportFormat` (the schema-versioned wire formats), `compute_precision_report` (the precision math), `write_report` (JSON + TSV serialisation), `check_tier_gate` + `HIGH_TIER_MIN` / `MEDIUM_TIER_MIN` (the mechanical invariant), `enforce_freshness` + `drift_error` + `read_source_snippet` (the auditor-immutability machinery). Every one of those surfaces is library-grade. The CLI's job is to parse `--emit-sample` / `--label` / `--format` and call the library; it currently does **the entire R8 implementation**. A future LSP / web-service / batch-CI host that wants to invoke R8 cannot today without pulling `gumiho-mudang-cli` as a dependency, which violates the charter §4 layering map (`gumiho-mudang-cli` depends on the engine, never the other way).

### Sub-items

- **(a) Cut a new sub-crate `scope-audit`** (or a module under `scope-core::audit`, decision in (b) below). Migrate from `gumiho-mudang-cli/src/commands/audit.rs` to it:
  - The sampling engine (`sample_stratified` + `xorshift64`).
  - The wire-format types (`SampleRecord` + `PrecisionReport` + `ReportRow` + `ReportFormat`) including their serde derives.
  - The precision computation (`compute_precision_report`).
  - The report writers (`write_report`).
  - The tier gate (`check_tier_gate` + `HIGH_TIER_MIN` + `MEDIUM_TIER_MIN`).
  - The auditor-immutability surface (`Graph::check_audit_freshness` may stay on `scope-graph` since it is graph-bound; the CLI's `enforce_freshness` + `drift_error` + `read_source_snippet` move with the audit library).
  - The constants `SCHEMA_VERSION`, `PRECISION_ONLY_DISCLAIMER`, `SCHEMA_DOC_POINTER`, `DEFAULT_SAMPLE_SIZE`, `DEFAULT_SEED`.
- **(b) Decide sub-crate vs. module.** A new sibling crate (`scope-audit`) matches the existing sub-crate split (R-move terminology — see `gumiho-mudang-scope/src/lib.rs` façade). A module under `scope-core::audit` is one fewer crate to compile. The trade-off is dependency direction: `scope-audit` would want `scope-graph` (`Graph`, `AuditEdgeRow`, `AuditFreshness`); a module under `scope-core` would force an upward dependency `scope-core → scope-graph` that does not exist today. Sibling sub-crate is the cleaner answer; the dispatch convenience of a module loses to the dependency-graph clarity. Confirm in the sprint plan.
- **(c) Reduce `commands/audit.rs` to dispatch only.** The CLI module retains: the `AuditArgs` / `AuditCommands` / `ConfidenceArgs` / `ReportFormat` (the **clap surface**, which is unavoidably CLI-grade); the `run(args, project_root)` entry point; the three subcommand-flow stubs (`run_confidence` → `default_summary` / `emit_sample` / `label_pass`) that call into `scope-audit` and print results. Target post-extraction size: under 200 LOC.
- **(d) Migrate the integration test suite.** `gumiho-mudang-cli/tests/integration/test_audit_confidence.rs` stays in the CLI (it exercises the CLI surface end-to-end), but the unit-test block currently inside `commands/audit.rs` migrates to `scope-audit` as a module test — the assertions test the engine, not the CLI.
- **(e) Audit every other CLI command for the same offender pattern.** Each `gumiho-mudang-cli/src/commands/*.rs` whose body exceeds the dispatch + formatting envelope is queued for the same extraction. Candidates spot-checked at sprint-0007 close: `index.rs` (large indexer driver), `flow.rs` / `trace.rs` (graph traversal lives in the CLI), `setup.rs` (process spawn lives in the CLI). Triage and queue them as further Priority 3 sub-items; do not bundle all of them into one sprint.

### Gate to start

Eligibility holds. Runs in parallel with Priority 1 and Priority 2 — independent surface.

### Why this is **not** absorbed by the refactor

The crate decomposition carved up `scope-core` / `scope-graph` / `scope-index` / `scope-search` / `scope-workspace` but did not retouch `gumiho-mudang-cli`. R8 is the first R-entry that grew a substantial engine; nothing in the R-entry catalogue forced the engine into the CLI, the implementation simply landed there because the CLI was the most obvious place to keep momentum. The honesty principle (Priority 2) applies here too in a different shape: *the layering on the box does not match the layering in the code*. Priority 3 corrects that.

---

## Cross-cutting items (charter §6 soft-expansion zone, not absorbed by refactor)

The architecture already absorbed several soft-expansion items into its R-entries (resolution pass → R3, domain edge kinds → R0, config-file readers → R4, confidence/provenance metadata → R0, decorator/annotation argument capture → R0 + R5). The items below are the **remainder**: they sit in the soft-expansion zone and remain new work against the current architecture.

- **Re-export resolution.** `pub use` chain following (Rust), `export * from` / `export {x} from` (TypeScript), `__all__` (Python), via static text. Lives in the resolver layer (R3) — the per-language re-export rules are new work against the current architecture.
- **Doc-comment chain merging.** `///` chains and `//!` inner docs (Rust), JSDoc multi-line (TS), `"""` blocks (Python). Improves docstring quality without semantic work.
- **Cross-project edges (`scope link`).** Mono-repo and microservice graphs as a single queryable index. Already on the roadmap per CHARTER §6.
- **Vector embeddings for `scope find`.** Semantic search by intent over name + doc + path + callers. CHARTER §6 names this.
- **Time-travel queries (`scope query @sha`).** Per-commit indices for PR review and historical impact analysis. CHARTER §6 names this; cost-tier `high`.
- **`.scm` query expansion** for additional symbol kinds (e.g., `mod` declarations as kind=module, `macro_rules!` definitions as kind=macro, JSX components as kind=… — extensions beyond the universal set already covered by the refactor).

Order is set by separate triggers, not by this document.

---

## Per-language depth queue

Per `LANGUAGE-PLAYBOOK.md` Step 6, each language plugin's depth queue lives in `docs/languages/<name>.md`. The seed list below is a copy of `CHARTER.md` §7 IN-scope items; the per-doc queue is the source of truth once each per-language doc is populated.

### Rust (depth target)
- `pub use` chain following via static text resolution
- `mod` declaration to file map (already partially done; complete via R4 workspace context)
- `#[derive(Trait)]` to `implements` edge (purely syntactic)
- `///` chain and `//!` inner-doc merging into a single docstring
- `async fn`, `unsafe fn`, `const fn` as metadata flags
- Multi-letter generic param filtering (`Item`, `Output`, `T1`) extending the existing single-letter filter
- Workspace member resolution via `Cargo.toml`
- `macro_rules!` definitions registered as `kind=macro` (definition only, not expansion — R11 enforces)
- `use ... as` alias capture

### Python (depth target)
- Decorators with arguments captured as metadata feeding domain edges (R0 hands the schema; per-language plugin populates the reserved `decorators` key)
- `__all__` export-list resolution
- Type hints captured as `references_type` edges
- `__init__.py` module hierarchy
- Class attributes captured as fields with `kind=property`
- `pyproject.toml` dependency graph for marking external imports

### Go (depth target)
- Interface satisfaction via static method-set comparison only (per CHARTER §7's narrowed bound — pointer-vs-value, embedded-interface promotion across packages, generic type parameters are explicitly out)
- Type embedding to method-promotion edges
- `go func()` to `green_thread_spawn` edge kind (renamed from `goroutine_spawn`; charter §7 Go section)
- Channel send/receive edges (`channel_send`, `channel_recv`)
- Build tag awareness (filter indexed files by `+build` / `//go:build`)
- `go.mod` workspace and module resolution

### TypeScript (depth target)
- JSX to `renders` edges (component tree)
- React hook usage edges — matched at the **framework** layer per `LANGUAGE-PLAYBOOK.md` Step 5 ("hooks" is not a reserved metadata key); the language plugin's contribution is populating the reserved `template_calls` key for JSX component invocations
- Decorator targets feeding domain edges (`@Controller`, `@Injectable`, `@Component`)
- `export * from` and `export {x} from` re-export resolution
- Type-only imports filtered out of `imports` edges
- `tsconfig.json` `paths` aliases for module resolution

### Ruby, Java, C# (surface)
- Bug-fix maintenance only. Promotion to depth target requires triggers per `LANGUAGE-PLAYBOOK.md` Step 7 ("Depth promotion request"). Until promoted, items above are not queued for these languages.

---

## Per-framework rollout (trigger-gated per FRAMEWORK-PLAYBOOK)

No framework ships before evidence proves the case via `FRAMEWORK-PLAYBOOK.md` Step 1 — either the trigger-driven path (3+ entries / 30 days) or the maintainer-asserted path for daily-driver frameworks already in active maintainer projects. The list below names candidates referenced in `CHARTER.md` §7 and the playbook examples — they are **not pre-approved**. Each requires its own Step 1 evidence + ROI worksheet + verdict in `FRAMEWORK-DECISIONS.md`.

- **Python**: Flask, FastAPI, Django, Celery
- **TypeScript**: Express, NestJS, Next.js (pages + app routes), Prisma, TypeORM
- **Ruby**: Rails (referenced throughout playbook examples; surface-only language plugin until Ruby itself promotes)
- **Rust**: Axum, Actix-web, Tokio (queue patterns)
- **Go**: gin, echo, gorilla/mux, sqlx, gorm

Each framework adoption ships independently per `FRAMEWORK-PLAYBOOK.md` Step 5 (one pattern at a time, real fixtures, precision audit before tier assignment).

---

## Items already absorbed by the refactor (do not re-plan)

Cross-references so a future reader does not duplicate work:

- **Resolution pass with confidence/status** — covered by R0 (schema) + R1 (builder) + R3 (typestate pipeline).
- **Domain edge kinds** — covered by R0 (whitelist additions: 31 net-new = `contains` universal + 30 domain across R0 baseline + Tier 1 + Tier 2 + Tier 3; final whitelist 38).
- **Call-site argument capture** (`edges.args_text`) — covered by R0; consumed by framework predicates (R5) and downstream cross-language stitching.
- **Config-file readers / WorkspaceContext** — covered by R4 (split into LanguageWorkspaceContext / FrameworkWorkspaceContext).
- **Symbol metadata structured fields** (decorators, annotations, template_calls) — covered by R0 (schema doc) + R5 (framework consumption).
- **Stable cross-session symbol IDs** — already shipped pre-refactor (`src/core/parser.rs:220`); maintained, not re-planned.

---

## Items deliberately deferred beyond this plan

Recorded so they are not lost; their triggers are insufficient today.

- **Self-indexing of Scope's own source** — `scope` is not run on the Scope repository during development. Self-indexing produces a feedback loop where the same buggy binary builds the index that the developer queries to debug the bug — symptoms and tooling fail together. The `bench-self` justfile recipe is removed; benchmarks run against external sandboxes (`bench-rails`) only. Dogfooding references in README "Done" list are removed.
- **Per-sub-root version detection** for npm/Python multi-package monorepos (`FRAMEWORK-PLAYBOOK.md` Step 3 known limitation; promoted when frequency justifies).
- **Module isolation** for stronger A1–A3 + B2 mechanical enforcement (`ENFORCEMENT-MAP.md` § "Why detectable, not mechanical"; charter-amendment-grade follow-up).
- **Byte-level lossy file reading** for invalid UTF-8 (`ENFORCEMENT-MAP.md` R6 known limitation; separate initiative with its own trigger).
- **`scope audit coverage`** subcommand for recall-side detection (separate from R8; trigger-deferred).
- **`.js` / `.jsx` indexing** via cheap path (extend `LanguageId::TypeScript.extensions()` arm) or strict path (new `JavaScript` variant of `LanguageId` — `scope-core/src/languages/id.rs`) — governed by `LANGUAGE-PLAYBOOK.md` adoption flow when triggers prove the need.
- **Optional symbol-kind renames** (`const` → `constant`, `type` → `type_alias`) — deferred to a post-R0 follow-up migration; not a blocker.

---

## Amendment rule

- Adding an item to "Cross-cutting" or "Per-language depth": commit message `docs(post-refactor): queue <item>` with one-paragraph rationale.
- Promoting an item from "deliberately deferred" to a queue: requires triggers logged in the matching trigger file + decision entry; commit message `docs(post-refactor): promote <item>`.
- Removing an item: commit message `docs(post-refactor): remove <item>` with one-paragraph rationale.
- Reordering within a queue is not amendment-controlled; ordering is set by triggers and ROI as items mature.
