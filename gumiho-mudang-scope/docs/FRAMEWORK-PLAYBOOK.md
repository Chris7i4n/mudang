# Framework Adoption Playbook

Companion to `CHARTER.md`, `ARCHITECTURAL-REFACTOR.md`, and `LANGUAGE-PLAYBOOK.md`.

The charter (section 6) says framework awareness is a legitimate expansion direction. The architectural refactor's R5 move ships the infrastructure: `FrameworkPlugin` consumes Symbols and Edges (never AST) and operates on the indexed graph rather than the parser's output. This document defines how the per-framework adoption decision is made — when to add a framework, how to support multiple versions, how to catalogue gotchas, and when to remove a framework that is no longer used.

The guiding principle is **on-demand only**. No framework is added because it is popular, fashionable, or "complete." A framework is added when the absence of support has caused measurable friction in the maintainer's own workflow, three or more times within a defined window.

This procedure is the gatekeeper. It is followed for every framework, with no exceptions.

---

## Step 1 — Adoption trigger

A framework adoption requires evidence that the framework belongs in Scope. Two paths produce that evidence; either is sufficient. Both paths feed Step 2 — the ROI worksheet is non-negotiable on either path.

### Path A — Trigger-driven (uncertain candidates)

For frameworks whose value to the maintainer is not yet obvious. The friction log proves the case empirically.

**Trigger log.** Maintain `docs/FRAMEWORK-TRIGGERS.md`. Append an entry whenever an LLM agent (or you, manually) gets stuck or wastes effort because Scope cannot answer a question that a framework-aware index would have answered immediately.

Format:

```
- 2026-05-08 | flask | agent grep'd app.py 4 times to find what handles POST /api/users
- 2026-05-09 | react | wrote a 30-line script to enumerate JSX components in src/
- 2026-05-12 | flask | trace from route to model required reading 6 files manually
```

Three fields: date, framework name, one-line description of the friction. Keep entries short and honest.

**Trigger threshold.** 3+ entries for the same framework within 30 days → candidate moves to **Step 2 (evaluation)**. Fewer than 3 → keep logging; do not act yet.

The threshold is deliberately conservative. Most frictions resolve themselves or are one-offs; only repeated friction earns engineering investment.

**Trigger discipline.**

- **Log honestly.** A one-liner script that solved the problem is not a trigger. The friction must have cost real time.
- **Log immediately.** Retrospective logs miss real friction; you forget the small irritations.
- **One trigger per real incident.** Do not pad the log to justify a framework you already want to build.
- **Do not log triggers for frameworks you do not currently use.** "I might want to use Vue someday" is not a trigger.

### Path B — Maintainer-asserted (obvious daily-use frameworks)

For frameworks the maintainer already uses heavily in active work. Logging 3 friction events in 30 days for a daily-driver framework (e.g., Rails on a Rails-shop maintainer, Tokio on a Rust-async-heavy maintainer) is theatre — the maintainer already knows the framework belongs in Scope. Path B skips the trigger log and goes directly to Step 2.

**When path B applies.**

- The maintainer works in projects that use the framework **this week or month** — not "want to try", not "old project on life support".
- Friction is predictable: every project of this stack produces the same kind of friction; logging each instance adds no information beyond what the maintainer already knows.

**Discipline.**

- Path B is **opt-in and recorded.** The decision log entry must declare `Path: maintainer-asserted` and name the active projects using the framework. Without that record, six months later the adoption looks speculative.
- Path B **does not bypass Step 2.** The ROI worksheet still runs; if build cost exceeds savings, the verdict is REJECT or DEFER. Maintainer-asserted is evidence of *need*, not a fast-track around *cost*.
- Path B is **not** a fast-track for popularity or ecosystem reach. "Express is everywhere" is not a maintainer-asserted reason. "I work in Express every day in projects X, Y, Z" is.

### Choosing between paths

| Situation | Path |
|---|---|
| Daily-driver in active maintainer projects (Rails, Tokio today) | B |
| Used occasionally, ROI unclear | A |
| Used by external collaborators but not in maintainer's own stack | neither — out of scope |
| Aspirational ("might want to support") | neither — log nothing |

---

## Step 2 — Evaluation

Once a candidate clears the trigger threshold, fill the ROI worksheet before any code is written.

### ROI worksheet

```
Framework: ____________________________
Used in projects: _____________________ (list active projects, with last-touched date)
Languages: ____________________________
Sessions per week relevant: ___________ (LLM agent sessions where this framework matters)
Estimated minutes saved per session: __ (be honest; conservative is better)
Build estimate (days): ________________ (predicate + detection + fixtures + tests; no `.scm` per framework — graph-only model per R5)
Maintenance estimate (hours per year): _ (framework version updates, regression fixes)
Confidence tier expected: high | medium | low
Versions to support initially: ________ (specific version numbers)
Verdict: BUILD | DEFER | REJECT
Notes: _________________________________
```

### Verdict matrix

Compute:

- `annual_savings_hours = (minutes_per_session × sessions_per_week × 50) / 60`
- `total_cost_hours = build_days × 8 + maintenance_hours_per_year`

Decide:

- `annual_savings_hours > total_cost_hours` → **BUILD**
- `annual_savings_hours` between `0.5 × total_cost_hours` and `total_cost_hours` → **DEFER** (re-evaluate in 90 days when more triggers may have accumulated)
- `annual_savings_hours < 0.5 × total_cost_hours` → **REJECT** (the framework is not added unless usage profile changes substantially)

### Decision logging

Whatever the verdict, log it in `docs/FRAMEWORK-DECISIONS.md`:

```
## YYYY-MM-DD — Framework: <name>

**Trigger count**: N (entries in FRAMEWORK-TRIGGERS.md from <start> to <end>)
**Verdict**: BUILD | DEFER | REJECT
**ROI worksheet**: [paste]
**Strategy**: A (latest only) | B (multi-version) | C (decline)
**Notes**: [reasoning, edge cases, caveats]
```

Future-you reads this when wondering why a framework is or isn't supported. Without this log, the same debate happens again in six months.

---

## Step 3 — Version strategy

Most frameworks have multiple major versions in active use simultaneously, and patterns diverge meaningfully between them — Rails 5 callback names vs Rails 7, Express 4 router signature vs Express 5, NestJS controller decorators across major versions. **Knowing the framework version is part of the framework plugin's contract**: every framework plugin must declare which versions it supports and detect the version at indexing time.

This is the **opposite** of the language layer. Language plugins do not branch by language version (rule C2, `LANGUAGE-PLAYBOOK.md` Step 4): the tree-sitter grammar handles the syntactic superset and version-specific semantics are the compiler's territory. Framework patterns, by contrast, are the maintainer's working surface and shift with each major framework release; ignoring the version produces false positives that audit (R8) cannot easily fix.

Mechanically: `Detection { detected, version, applies_to_languages }` (`ARCHITECTURAL-REFACTOR.md` R5) carries the version. The framework predicate inspects `Detection.version` and selects which `Symbol.metadata` shapes to match plus which `Edge.args_text` literals to filter (e.g., HTTP path strings, queue names, env-var names — `args_text` is captured raw by language plugins per R0 and interpreted by framework plugins). The version source of truth is the workspace config: `Gemfile.lock`, `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`, exposed via `WorkspaceContext` (R4). The framework plugin never reads files directly.

### Granularity (semver via VersionReq)

Predicates use full semver via `semver::VersionReq` rather than match arms on `(major, minor)`. Each pattern in the framework's catalog carries an `available_in: VersionReq` declaring the version range where it applies. Examples:

- `BELONGS_TO`: `available_in = VersionReq::parse(">=1.0.0").unwrap()` — every version
- `BEFORE_FILTER`: `available_in = VersionReq::parse(">=1.0.0, <5.1.0").unwrap()` — removed in 5.1
- `BEFORE_ACTION`: `available_in = VersionReq::parse(">=5.0.0").unwrap()` — added in 5.0
- `DELEGATED_TYPE`: `available_in = VersionReq::parse(">=6.1.0").unwrap()` — added in 6.1
- `ENCRYPTS`: `available_in = VersionReq::parse(">=7.0.0").unwrap()` — added in 7.0

Why full semver and not `(major, minor)`: real frameworks ship breaking changes in patches occasionally; `7.0.4.3` may behave differently from `7.0.4`. Full semver costs nothing extra (one `.matches(version)` call per pattern) and prevents the "I assumed minor was enough and was wrong" failure mode.

**Non-strict-semver versions** (Rails `7.0.4.3`, Python `3.11.0a1`, build-metadata-coupled tags) do not parse with `semver::Version::parse` directly. The version-coercion layer (`ARCHITECTURAL-REFACTOR.md` R5 → `DetectedVersion::Resolved`) maps each framework's raw version string to a `semver::Version` via a per-framework rule recorded in the per-framework doc — typically dropping the 4th component or stripping pre-release identifiers. The coercion is lossy: a `7.0.4.3` security patch that fixes behavior versus `7.0.4.0` is invisible to `VersionReq` after coercion. Frameworks whose versioning fundamentally cannot be coerced declare `DetectedVersion::NoVersionConcept` and treat every pattern as `VersionReq::STAR`.

**Range-only manifests** (a `package.json` with `"^7.0"` but no `package-lock.json`; a `pyproject.toml` declaring a range without `poetry.lock`) resolve to `DetectedVersion::Indeterminate`, not a synthetic concrete version. The plugin's `unknown_version_policy()` decides what happens (zero edges by default). Inventing a version inside the range would silently lock in an answer the workspace has not committed to.

### Unknown-version policy (per-plugin)

When `Detection.version == DetectedVersion::Indeterminate` (no lockfile, vendored fork without parseable version, range-only manifest, beta tag without coerced semver), the plugin declares one of three policies via `unknown_version_policy()`:

- **`Skip`** (recommended default) — emit zero domain edges. Honest fallback. The framework's value re-emerges as soon as the user runs `bundle install` / `npm install` / etc.
- **`StableOnlyLowConfidence`** — emit only patterns whose `available_in == VersionReq::STAR` (universally applicable). Edges carry `confidence=low` and `producer='framework:<name>:fallback'`. **Risk**: frameworks remove historically-stable patterns occasionally (Rails dropped `before_filter` in 5.1 despite it existing since 1.0); this policy accepts the resulting false-positive risk.
- **`AssumeLatest(version)`** — pretend the latest declared version is active. Edges carry `producer='framework:<name>:assumed_<version>'`. **Risk**: silent false positives if the actual project is on an older major.

Pick `Skip` unless you have a specific reason. Document the choice and rationale in the per-framework doc.

### Cross-workspace queries (cross-app, single workspace)

A typical case: a Cargo workspace (or Bundler / well-rooted pyproject equivalent) with multiple member crates/apps sharing the same framework version. Example: a Rust workspace where crate A produces `tokio` queue messages and crate B consumes them. The framework predicate sees the entire workspace as one symbol+edge pool (charter §3 invariant 4 — single polyglot graph), and emits cross-crate domain edges naturally. No special configuration is needed; cross-app queries are first-class because the graph is.

**Workspace-uniform-version assumption — and where it breaks.** The current `detect()` returns one `DetectedVersion` per workspace, treating the root manifest as authoritative. This is correct for Cargo (workspace inheritance is the canonical pattern) and Bundler (one `Gemfile.lock` per repo). It is **not universally correct** for npm workspaces with per-package `package.json` files, pip-only Python monorepos with per-package `pyproject.toml`, or any layout where each app pins independently. In those layouts a workspace can mix two framework majors (Rails 5 sub-app + Rails 7 sub-app, Express 4 + Express 5 across services). Today, edges from the wrong-version sub-app are emitted from the root-version pattern set with degraded precision. The R8 confidence audit surfaces the regression as a precision drop on that sub-root.

The known-limitation entry goes in the per-framework doc; per-sub-root detection is a future enhancement governed by trigger frequency (`docs/FRAMEWORK-TRIGGERS.md`). For maintainer stacks dominated by Cargo and Bundler the limitation rarely fires; for npm/Python-multi-package stacks it may fire frequently enough to promote to its own initiative (queued in [`POST-REFACTOR-PLAN.md` § Items deliberately deferred](POST-REFACTOR-PLAN.md#items-deliberately-deferred-beyond-this-plan)).

Decide strategy per framework, not globally.

### Strategy A — Latest only

Build the plugin against the latest stable major version. Older versions degrade gracefully:

- Patterns that no longer match silently produce no edges (better than wrong edges).
- The trade-off is deliberate: a partial index is honest; a wrong index is not.

Pick A when:

- All your active projects use the latest version.
- The framework's pattern surface is stable across recent versions (decorator names unchanged, import paths unchanged).
- The cost of multi-version support exceeds its benefit.

This is the default. Most frameworks should ship with strategy A.

### Strategy B — Multi-version with detection

Detect framework version from the workspace config (`package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`) via `WorkspaceContext` (R4). Branch the framework predicate by detected version.

Implementation pattern (graph-only model — see `ARCHITECTURAL-REFACTOR.md` R5):

- The framework plugin's predicate code reads `Detection { version, … }` and selects which `Symbol.metadata` shapes (`decorators`, `annotations`, `template_calls`) to match. Naming-convention shapes (React `^use[A-Z]` hooks, Vue composable conventions) are matched directly against `Symbol.name` and `edges WHERE kind='calls'` rows; they are not reserved metadata keys.
- No per-version `.scm` files exist. The language plugin remains unaware of the framework's versions; only the framework predicate branches.

Pick B when:

- Active projects span 2+ major versions of the same framework.
- Patterns differ enough that strategy A produces false positives or substantial false negatives.

Cost: roughly 1.3–1.5× the build cost of strategy A (one extra branch in the predicate plus matching fixtures per version).

### Strategy C — Decline

Do not build the plugin yet. Pick C when:

- Framework is mid-rewrite (e.g., one major version is being replaced by an incompatible successor — Vue 2 → Vue 3, Angular.js → Angular 2, Express 4 → Express 5 if patterns diverge).
- Patterns are unstable; ROI is eroded by churn.

Re-evaluate when the ecosystem stabilizes (typically 12–18 months after the rewrite ships).

### Recording the strategy

The chosen strategy goes in both `docs/FRAMEWORK-DECISIONS.md` and the framework's own doc (`docs/frameworks/<name>.md`).

---

## Step 4 — Gotcha catalogue

The 15 categories below are **per-instance decisions**, not universal rules. They are not part of the architectural-refactor inventory; they are recorded in the per-framework template (`docs/frameworks/_TEMPLATE.md`) walkthrough table. A framework plugin is not shippable until every row of the walkthrough has an explicit decision (matched / not matched / matched with confidence downgrade / N/A).

Every framework plugin has a companion document at `docs/frameworks/<name>.md` that captures everything a future maintainer (you in six months) needs to understand what the plugin does and why it does it that way.

### Language scope

Every framework plugin declares `Detection.applies_to_languages` (per R5 in `ARCHITECTURAL-REFACTOR.md`). The indexer pre-filters symbols and edges by this list before the predicate runs, so a Rails plugin scoped to `[Ruby]` cannot accidentally match a Python file that happens to use a decorator name shared with a Rails callback. Polyglot remains a feature of the graph; cross-language matching is opt-in per framework, not the default. Record the list in the per-framework doc and justify it.

**Allowed values are the variants of `LanguageId`** (`scope-core/src/languages/id.rs`, post-R7 rename of historical `SupportedLanguage`): `TypeScript`, `CSharp`, `Python`, `Go`, `Java`, `Rust`, `Ruby`. JavaScript is not currently a `LanguageId` variant. **`.js` and `.jsx` files are not indexed today** — the indexer's extension dispatch (`scope-core::languages::dispatch::dispatch_extension`) accepts only `ts|tsx`, and `LanguageId::TypeScript.extensions()` declares only `&["ts", "tsx"]`. React-style framework plugins therefore declare `applies_to_languages = vec![LanguageId::TypeScript]` and match exclusively against `.ts`/`.tsx` sources; a `.jsx` file written in plain JavaScript is invisible to every framework predicate today.

Adding `.js`/`.jsx` indexing requires one of two changes, each governed by its own playbook:

- **Cheap path** — extend `LanguageId::TypeScript.extensions()` (the inherent const-method match arm) to `&["ts", "tsx", "js", "jsx"]`. `tree-sitter-typescript` parses both syntactically as a superset (`.tsx` grammar covers JSX). Type annotations in TS-only sources continue to populate `references_type` edges; JS sources skip them. This is a small `LANGUAGE-PLAYBOOK.md` Step 7 maintenance change to the existing TypeScript arm and does not need a new `LanguageId` variant.
- **Strict path** — add `JavaScript` as a separate `LanguageId` variant with its own arms across every inherent method on `LanguageId`. Earns extension-disambiguation (a `.tsx` file is unambiguously TS-with-JSX; a `.jsx` file is unambiguously JS-with-JSX) at the cost of grammar duplication and per-language fixture work. Triggered through `LANGUAGE-PLAYBOOK.md`'s adoption flow (Step 1 trigger threshold, Step 2 ROI worksheet).

Pick the cheap path unless triggers prove the strict path is needed. Either way, the framework plugin's `applies_to_languages` list is what changes — never the framework's predicate code, which already operates over symbols and edges agnostic of which language plugin produced them.

### Per-framework doc structure

```markdown
# Framework: <name>

## Versions supported
- 1.x: detection via `<package> >= 1.0, < 2.0` in pyproject.toml/package.json/etc.
- 2.x: detection via `<package> >= 2.0, < 3.0`.

## Patterns matched

### http_route via decorator
- Predicate: `Symbol.metadata.decorators[].name` matches `app.get|post|put|delete|patch|route` and the first arg is a string literal.
- Emitted edge kind: `http_route` (from caller symbol → handler symbol).
- Confidence: high
- Versions: 1.x and 2.x identical
- Example:
  ```python
  @app.get("/foo")
  def handler(): ...
  ```

### http_route via class method
- Predicate: a class with `Symbol.metadata.decorators[].name == 'route'` plus methods carrying `decorators[].name` matching HTTP verbs; resolution joins the class-level prefix with method-level paths.
- Confidence: medium (requires resolving the `app` instance via the workspace's symbol table; ambiguity downgrades to medium).
- Versions: 2.x only
- Example: ...

## Patterns deliberately not matched
- Dynamic route registration (`app.add_route(handler, path_var)`):
  runtime-only; static parser cannot infer path.
- Conditional registration (`if env == 'dev': app.get(...)`):
  both branches produce edges with `medium` confidence; agent must filter.

## Known gotchas
1. **Wrapper imports** — `from foo import app as application`.
   Plugin follows alias via the resolution layer (R3) and the workspace context (R4).
2. **Multiple instances** — `admin_app = Flask(__name__); user_app = Flask(__name__)`:
   routes register to different instances.
   Plugin emits edges keyed by instance variable name.
3. **Blueprint binding** — Flask blueprints are registered on the app via
   `app.register_blueprint(bp)`; without this call, blueprint routes are dangling.
   Plugin reads the registration call.

## Confidence rationale
- Decorator + literal path → high.
- Decorator + variable path → medium (path may be a constant defined elsewhere).
- Class-based view → medium (binding via class registration).

## Test fixtures
- `tests/fixtures/frameworks/<name>/v1/` — version 1.x fixtures
- `tests/fixtures/frameworks/<name>/v2/` — version 2.x fixtures
- `tests/integration/test_framework_<name>.rs` — runs against fixtures
  and compares to expected edges (snapshot-tested via insta).

## SUNSET (filled in only when the plugin is sunset)
- Date: YYYY-MM-DD
- Reason: ...
- Last supported version: ...
```

A `docs/frameworks/_TEMPLATE.md` mirrors this structure and is the starting point for every new plugin.

### Common gotcha categories — checklist

Walk this checklist before declaring a framework plugin done. Each item must have an explicit decision: **matched**, **not matched**, or **matched with confidence downgrade**.

1. **Decorator vs function-call API** — does the framework expose both forms? (Flask: `@app.route("/x")` and `app.add_url_rule(...)`).
2. **Class-based vs function-based handlers** — Django views, NestJS controllers, React class vs function components.
3. **Convention-by-location** — Next.js `pages/`, Django `apps/<x>/views.py`, Rails `controllers/`. File path is part of the binding; detection logic must understand directory layout.
4. **Lazy or runtime registration** — Celery `app.send_task("name")`, Django `path("...", include("foo.urls"))`, dynamic plugin registration.
5. **Wrapper or re-export patterns** — `import { Router } from 'express'` vs `import express from 'express'; const r = express.Router()`.
6. **Inheritance and composition** — interface extending interface; class extending class; component composition (HOCs, render props).
7. **Generics-parameterized signatures** — Go interface satisfaction frequently fails here.
8. **Aliasing in imports** — `import { Router as R } from 'express'`. Plugin must follow the alias via the re-export resolution layer.
9. **Multiple instances** — multiple framework objects coexisting in one app (admin app vs user app).
10. **Conditional registration** — `if env === 'dev': app.use(devMiddleware)`. Static parser sees both branches; downgrade confidence or mark with metadata.
11. **Indirect handler binding** — `app.use("/api", routes)` where `routes` is defined elsewhere; requires resolution.
12. **Nested routers / sub-apps** — Express sub-routers, Flask blueprints, NestJS modules, Axum nested routers.
13. **Type-driven routing** — frameworks that use generics or types to determine route shape (NestJS, tRPC). Often medium or low confidence.
14. **String-based dispatch** — Celery's `send_task("module.task_name")`, Django's `reverse("view-name")`. The string is the binding; resolution depends on whether the target name is uniquely defined.
15. **Decorator factories** — `@cache(ttl=60)(handler)` returns a wrapped function. Plugin must recognize that the inner function is the actual handler.

For each, the framework's gotcha doc records the decision and the reasoning.

---

## Step 5 — Implementation order within a framework

Inside a single framework adoption (graph-only model — no `.scm` per framework, see `ARCHITECTURAL-REFACTOR.md` R5):

1. **Confirm the language plugin populates the relevant metadata keys.** For Flask, the Python plugin must populate `Symbol.metadata.decorators` with `{name, args_text}`. For React, the TypeScript plugin must populate `metadata.template_calls` with `{name, args_text}` for each JSX component invocation. (The same key is populated for ERB partials in Ruby, Jinja includes/extends in Python, HEEx components in Elixir, etc., when those plugins ship.) Hook detection (`^use[A-Z]` calls) is the framework predicate's responsibility — no `metadata.hooks` key exists, since regex-on-name violates E2 at the language layer. If a required AST-shape key is missing, file a language-plugin change first; framework adoption blocks on that.
2. **Pick the highest-frequency pattern first.** For Flask, that's the `@app.route` decorator. For React, that's JSX rendering. For Axum, that's the route macro. Implement the predicate (SQL or Rust matcher over `Symbol.metadata`, `Edge` rows, and `Edge.args_text` literals captured by the language plugin per R0) for just this pattern. The predicate may write framework-specific normalisation hints into `Symbol.metadata` for downstream consumers (e.g., `base_url`, `mount_prefix`, `version_prefix`, `method`, `queue`, `wildcard`) — the framework plugin is allowed to interpret what the language plugin captured raw.
3. **Build 5+ real-world fixtures** from your own projects (anonymized to remove secrets and proprietary names). Synthetic fixtures undercatch real gotchas; use real code.
4. **Run the index, measure precision** against manually labelled ground truth. Manual labelling is tedious but is the only way to know what tier the patterns deserve.
5. **If precision < 80%** on real fixtures, revisit the predicate or downgrade the confidence tier. Do not ship at high confidence with measured low precision. Honest medium-tier output is better than dishonest high-tier output.
6. **Add the second-most-frequent pattern.** Repeat 3–5.
7. **Stop when**:
   - 80%+ of relevant queries (the ones that triggered adoption) are answerable from indexed edges, OR
   - Diminishing returns: each new pattern catches < 5% additional edges.

Premature completionism is a maintenance trap. The bar is "useful for the maintainer's queries", not "exhaustive over the framework's API surface."

### Anti-pattern — accidental scope creep

Implementing one pattern often suggests three more. Resist. Each new pattern is more surface to maintain. If the new pattern would catch < 5% new edges and was not in the trigger log, do not build it.

---

## Step 6 — Maintenance triggers

After a framework plugin ships, watch for these signals:

### New major version released

- Re-run the version-detection logic. Fixtures may need updating.
- Decide A / B / C again — adoption strategy can change as the ecosystem moves.
- Log the re-evaluation in `docs/FRAMEWORK-DECISIONS.md` as an amendment to the original decision.
- If patterns broke and your active projects have moved to the new version, build version 2 patterns (strategy B) or migrate (strategy A pinned to v2).

### Test fixtures fail after dependency bump

- Patch the predicate, or fix the language plugin's metadata population if a missing reserved key is the root cause (graph-only model per R5 — no `.scm` per framework).
- If the patch is non-trivial (more than a few lines), record in the gotcha doc.
- A second consecutive non-trivial patch is a signal to consider sunset (Step 7) or downgrade confidence.

### Plugin unused for 6 months

- Mark dormant in `docs/FRAMEWORK-DECISIONS.md`.
- Consider sunset (Step 7).
- Dormant ≠ broken; it just means no recent triggers.

### Confidence audit fails (R8 in `ARCHITECTURAL-REFACTOR.md`)

- If `confidence='high'` edges from this plugin are wrong > 5%, downgrade to `medium` or root-cause and fix.
- If `confidence='medium'` edges are wrong > 30%, downgrade to `low`.
- A plugin that produces only `low` edges is a signal to consider sunset; low-tier edges are rarely useful enough to justify maintenance.

---

## Step 7 — Sunset procedure

When a framework is no longer used, no longer worth maintaining, or has been replaced in your own work:

1. **Document the decision** in `docs/frameworks/<name>.md` SUNSET section, with date and reason.
2. **Move plugin code** from `src/frameworks/active/` to `src/frameworks/archived/` (or feature-gate behind a cargo feature). Do not delete immediately.
3. **Existing indices retain their domain edges** — do not retroactively delete. Old indices remain queryable; the sunset only affects new indexing.
4. **Future indexing skips the framework**.
5. **Remove fixtures and tests** after one release cycle. This gives time to revert if you change your mind, and lets you verify no other plugin depends on the fixtures.

Sunsetting is not failure; it is honest pruning. A maintained set of small plugins beats an abandoned set of big ones.

### When to sunset

- Framework removed from your active projects.
- Framework deprecated upstream and your projects have migrated.
- Confidence audit consistently fails and root-cause is intractable.
- 12+ months of dormancy.

---

## Step 8 — On-demand discipline

This entire playbook exists to enforce one discipline: **do not build framework support before pain is felt**.

### Anti-patterns to avoid

- **Adding a framework "for completeness"** — completeness is not a goal of a personal tool.
- **Adding a framework because it is popular** — popularity in the world is not popularity in your work.
- **Speculating multi-version support** — start with strategy A; switch to B only when the second version actually appears in your projects.
- **Merging without 5 real fixtures** — real fixtures catch real gotchas; synthetic fixtures don't.
- **Shipping high confidence on measured-medium precision** — be honest about confidence; downgrade or rebuild.
- **Skipping the decision log** — every framework verdict, including REJECT, is logged. Otherwise the same conversation happens every six months.
- **"Quick wins" outside the playbook** — every framework follows this procedure. No shortcuts, no exceptions.

### Decision flow summary

```
new pain felt
    → is the framework a daily-driver in an active maintainer project?
        → yes → path B (maintainer-asserted) → fill ROI worksheet
        → no  → path A (trigger-driven) → log trigger entry
            → 3 triggers same framework within 30 days?
                → no  → keep logging
                → yes → fill ROI worksheet
    → ROI verdict
        → BUILD
            → choose version strategy (A / B / C)
            → implement (Step 5)
            → ship
        → DEFER → wait 90 days → re-evaluate
        → REJECT → log decision and stop
```

---

## Step 9 — Document index

Four documents form the working set for any framework decision:

- **`CHARTER.md`** — what Scope is and is not. Permanent constraints.
- **`ARCHITECTURAL-REFACTOR.md`** — structural closure that mechanically enforces charter and playbook rules. R5 lands the framework infrastructure.
- **`LANGUAGE-PLAYBOOK.md`** — language-plugin universal boundaries (the 18 rules). Framework gotchas may also touch language-plugin rules; check both.
- **`FRAMEWORK-PLAYBOOK.md`** (this file) — how to choose what frameworks to support, when, and at what version coverage.

Three runtime artifacts track ongoing decisions:

- **`docs/FRAMEWORK-TRIGGERS.md`** — append-only log of friction events (Step 1).
- **`docs/FRAMEWORK-DECISIONS.md`** — verdict log, one entry per evaluation (Step 2).
- **`docs/frameworks/<name>.md`** — per-framework gotcha doc (Step 4), one per BUILD verdict.

A `docs/frameworks/_TEMPLATE.md` provides the starting structure for new framework docs.

When in doubt, all three working-set documents are consulted before code is written. The runtime artifacts answer "why did we do this" six months from now.

---

## Closing principle

This playbook treats every framework as expensive. Build cost, maintenance cost, mental cost of remembering what's there. The only justification for paying that cost is repeated friction in your own real work.

A small, well-maintained set of framework plugins is the goal. A long list of "we support everything" is the failure mode of the previous owner of this codebase. The on-demand discipline is what prevents history from repeating.
