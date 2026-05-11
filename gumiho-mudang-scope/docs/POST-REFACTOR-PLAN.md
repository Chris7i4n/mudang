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
- **`.js` / `.jsx` indexing** via cheap path (extend TypeScriptPlugin extensions) or strict path (new `JavaScript` SupportedLanguage variant) — governed by `LANGUAGE-PLAYBOOK.md` adoption flow when triggers prove the need.
- **Optional symbol-kind renames** (`const` → `constant`, `type` → `type_alias`) — deferred to a post-R0 follow-up migration; not a blocker.

---

## Amendment rule

- Adding an item to "Cross-cutting" or "Per-language depth": commit message `docs(post-refactor): queue <item>` with one-paragraph rationale.
- Promoting an item from "deliberately deferred" to a queue: requires triggers logged in the matching trigger file + decision entry; commit message `docs(post-refactor): promote <item>`.
- Removing an item: commit message `docs(post-refactor): remove <item>` with one-paragraph rationale.
- Reordering within a queue is not amendment-controlled; ordering is set by triggers and ROI as items mature.
