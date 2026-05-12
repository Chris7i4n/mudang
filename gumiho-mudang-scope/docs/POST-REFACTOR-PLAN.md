# Post-Refactor Plan

Work queued to start **after** `ARCHITECTURAL-REFACTOR.md` closes (Phase E acceptance). No item below ships before then. Each item additionally respects its own gate — language depth follows `LANGUAGE-PLAYBOOK.md` adoption flow; framework adoption follows `FRAMEWORK-PLAYBOOK.md` triggers.

This document is the answer to "what comes next?" Until Phase E acceptance is met, this is a queue, not a backlog being worked.

---

## Gate

Phase E acceptance is the entry condition. Every bullet below must hold before any item in this document is started:

- Every universal rule in the inventory tables (`CHARTER.md` §5 hard limits and `LANGUAGE-PLAYBOOK.md` Step 4) is in class 1 (mechanical), class 2 (detectable), or the explicit class-3 universal list (B1, C2, E3).
- Every active language plugin's `docs/languages/<name>.md` has zero `NEEDS REVIEW` entries.
- `scope audit confidence` exists and runs against the reference fixture corpus.
- CI gates active: malformed-source (R6), trait-shape audit + spawn-denylist (R12), immutable-source (R9).
- Full benchmark suite shows < 10% regression from pre-refactor baseline.

`REFACTOR-STATUS.md` reflects the live state.

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

### Gate to start

Phase E acceptance (per "Gate" section above). Specifically: R8 must be `shipped` and the reference fixture corpus must be committed, so this priority has a working sensor to build on.

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

- **R0 `edges.args_text` 2 KB cap** — see [`ARCHITECTURAL-REFACTOR.md` § R0 → Mitigation 2](ARCHITECTURAL-REFACTOR.md#r0--schema-closures--edge-kind-additions--symbols-metadata-shape) and the const `ARGS_TEXT_CAP_BYTES = 2048` in [`scope-core/src/edge.rs`](../scope-core/src/edge.rs). Truncating call-site / declaration-site argument literals at 2 KB plus a `[truncated]` marker is an approximation justified by *"common case fits"* — **not** by any hard runtime constraint. SQLite TEXT holds up to ~1 GB; long literals make Scope slower on pathological codebases but cannot panic, OOM, or fail to run. The fix is to drop the cap and the truncation marker; the pre-1.0 wipe-and-reindex policy (CHARTER §2) absorbs the schema impact for existing local DBs.

### Sub-items (sequenced — (a) and (b) feed (c) and (d))

- **(a) Charter-grade audit.** Walk every R-move acceptance bullet, every schema comment, every doc rationale. Flag every use of the words *cap*, *truncate*, *limit*, *approximate*, *sample*, *heuristic*, *good enough*, *common case*, *roughly*. For each: is the trade-off justified by a hard runtime constraint (panic / OOM / won't-run)? If yes — leave it and surface the constraint verbatim in the doc. If no — queue for fix.
- **(b) Code-grade audit.** Grep every workspace crate for `const .*: usize = ` whose name contains `CAP`, `LIMIT`, `MAX`, `TRUNC`, `BUDGET`, `THRESHOLD`, or that is followed by truncation / sampling / fallback logic. Same triage as (a).
- **(c) Drop the R0 `args_text` 2 KB cap** (known offender above) **and bump the audit-sample JSONL `schema_version` from `"1"` to `"2"` adding a `producer_captured_args: string | null` field**. The two changes ship together because they are the same fidelity move from two angles: (c.i) the schema bump exposes what the extractor actually captured at index time as a first-class column in the JSONL sample, so an external labeller can compare current source against the index-time capture side-by-side instead of squinting at `args_text` through R8's source-file fallback; (c.ii) dropping the cap makes that index-time capture actually faithful (under the 2 KB cap, `producer_captured_args` would carry the truncated stub and inherit the lie). Bundled, the post-refactor Scope ships the auditor a complete two-source comparison: current source via `source_snippet`, index-time capture via `producer_captured_args`. One commit, charter-grade amendment on `main`:
  - Delete `ARGS_TEXT_CAP_BYTES`, `TRUNCATION_MARKER`, and the truncation logic in `scope-core/src/edge.rs`.
  - Delete the matching unit test (currently asserts the truncation byte length).
  - Update the schema comment in `scope-graph/src/sql/schema.sql` (drop "capped at 2 KB / truncation marker" text).
  - Update [`ARCHITECTURAL-REFACTOR.md` § R0 → Mitigation 2](ARCHITECTURAL-REFACTOR.md#r0--schema-closures--edge-kind-additions--symbols-metadata-shape) (replace with a note recording the original cap was dropped by Priority 2; honesty over performance; pre-1.0 wipe policy stands).
  - Bump `schema_version` from `"1"` to `"2"` in [`AUDIT-LABEL-SCHEMA.md`](AUDIT-LABEL-SCHEMA.md), add the `producer_captured_args: string | null` record field with the auditor-comparison rationale, add a migration note. Update `--label` rejection logic so old `schema_version: "1"` samples error with a re-emit instruction.
  - Log entry in `REFACTOR-STATUS.md` documenting the amendment (paper-trail discipline per §3 ambiguity protocol).
- **(d) Fix any further offenders found by (a) + (b).** Each fix lands as its own charter-grade amendment with paper trail.
- **(e) Capture remaining justified approximations as explicit invariants.** Where (a) or (b) finds a trade-off that *is* justified by a hard runtime constraint, the constraint moves into the document as a first-class invariant (not a footnote). Future sprints know the line was drawn deliberately and where.

### Gate to start

Phase E acceptance (per "Gate" section above). Runs in parallel with Priority 1 — independent surfaces (Priority 1 builds the self-correction actuator on top of R8; Priority 2 audits the data the actuator measures). Neither blocks the other.

### Why this is **not** absorbed by Priority 1 (self-correction cycle)

Priority 1's labelling pipeline reads `source_snippet` directly from the source file at audit time — it deliberately sidesteps `args_text` precisely because R8's design recognised the approximation issue. So Priority 1 ships safely even before Priority 2 lands. But every **other** consumer of `args_text` (resolver, framework plugins, future LSP integration, time-travel queries) is still reading a possibly-truncated string. Priority 2 plugs the leak system-wide.

---

## Cross-cutting items (charter §6 soft-expansion zone, not absorbed by refactor)

The refactor absorbed several soft-expansion items into its R-moves (resolution pass → R3, domain edge kinds → R0, config-file readers → R4, confidence/provenance metadata → R0, decorator/annotation argument capture → R0 + R5). The items below are the **remainder**: they sit in the soft-expansion zone but require new work after the refactor closes.

- **Re-export resolution.** `pub use` chain following (Rust), `export * from` / `export {x} from` (TypeScript), `__all__` (Python), via static text. Lives in the resolver layer added by R3 — but R3 only ships the framework; the per-language re-export rules are post-refactor work.
- **Doc-comment chain merging.** `///` chains and `//!` inner docs (Rust), JSDoc multi-line (TS), `"""` blocks (Python). Improves docstring quality without semantic work.
- **Cross-project edges (`scope link`).** Mono-repo and microservice graphs as a single queryable index. Already on the roadmap per CHARTER §6.
- **Vector embeddings for `scope find`.** Semantic search by intent over name + doc + path + callers. CHARTER §6 names this.
- **Time-travel queries (`scope query @sha`).** Per-commit indices for PR review and historical impact analysis. CHARTER §6 names this; cost-tier `high`.
- **`.scm` query expansion** for additional symbol kinds (e.g., `mod` declarations as kind=module, `macro_rules!` definitions as kind=macro, JSX components as kind=… — extensions beyond the universal set already covered by the refactor).

Order is set by separate triggers, not by this document.

---

## Per-language depth queue

Per `LANGUAGE-PLAYBOOK.md` Step 6, each language plugin's depth queue lives in `docs/languages/<name>.md`. The seed list below is a copy of `CHARTER.md` §7 IN-scope items; the per-doc queue is the source of truth once each per-language doc is populated post-refactor.

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

- **Self-indexing of Scope's own source** — `scope` is not run on the Scope repository during development. Until a stable release ships, dogfooding hides bugs from the developer (the same buggy binary builds the index that the developer queries to debug the bug, so symptoms and tooling fail together). Re-enable when stable. In the meantime, the `bench-self` justfile recipe is removed; benchmarks run against external sandboxes (`bench-rails`) only. References to "dogfooding" in user-facing copy (README "Done" list) are removed; CHANGELOG entries are historical and stay.
- **Per-sub-root version detection** for npm/Python multi-package monorepos (`FRAMEWORK-PLAYBOOK.md` Step 3 known limitation; promoted when frequency justifies).
- **Module isolation** for stronger A1–A3 + B2 mechanical enforcement (`ARCHITECTURAL-REFACTOR.md` "Why detectable, not mechanical"; charter-amendment-grade follow-up).
- **Byte-level lossy file reading** for invalid UTF-8 (`ARCHITECTURAL-REFACTOR.md` R6 known limitation; separate refactor with its own trigger).
- **`scope audit coverage`** subcommand for recall-side detection (separate from R8; trigger-deferred).
- **`.js` / `.jsx` indexing** via cheap path (extend `LanguageId::TypeScript.extensions()` arm) or strict path (new `JavaScript` variant of `LanguageId` — `scope-core/src/languages/id.rs`) — governed by `LANGUAGE-PLAYBOOK.md` adoption flow when triggers prove the need.
- **Optional symbol-kind renames** (`const` → `constant`, `type` → `type_alias`) — deferred to a post-R0 follow-up migration; not a blocker.

---

## Amendment rule

- Adding an item to "Cross-cutting" or "Per-language depth": commit message `docs(post-refactor): queue <item>` with one-paragraph rationale.
- Promoting an item from "deliberately deferred" to a queue: requires triggers logged in the matching trigger file + decision entry; commit message `docs(post-refactor): promote <item>`.
- Removing an item: commit message `docs(post-refactor): remove <item>` with one-paragraph rationale.
- Reordering within a queue is not amendment-controlled; ordering is set by triggers and ROI as items mature.
