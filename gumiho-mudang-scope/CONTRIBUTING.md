# Contributing to gumiho-mudang-scope

Newbie on-ramp. Glue between the canonical docs — every rule lives in the doc this page links to, never inline here.

If you have not read [`docs/README.md`](docs/README.md) yet, start there. It defines the reading order for the governing docs ([`CHARTER.md`](docs/CHARTER.md), [`ENFORCEMENT-MAP.md`](docs/ENFORCEMENT-MAP.md), [`LANGUAGE-PLAYBOOK.md`](docs/LANGUAGE-PLAYBOOK.md), [`FRAMEWORK-PLAYBOOK.md`](docs/FRAMEWORK-PLAYBOOK.md), [`BACKLOG.md`](docs/BACKLOG.md), [`sprints/README.md`](docs/sprints/README.md)). This file is the orchestration layer above them.

---

## First-time setup

Prereqs:

- Rust toolchain — pinned in [`rust-toolchain.toml`](../rust-toolchain.toml). Stable channel, `rustfmt` + `clippy` components. Run any `cargo` command once and `rustup` picks the pin up.
- `just` task runner (recipes documented in [`justfile`](../justfile)).
- `sqlite3` on `PATH` — the runtime links libsqlite3.
- macOS / Linux. Other platforms not exercised.

### Per-machine cargo config (opt-in, gitignored)

Every `.cargo/` directory in the repo is gitignored ([`.gitignore`](../.gitignore) lines 38–42) by deliberate single-operator-posture convention — cargo settings are per-machine, not workspace baselines. Drop your preferences in a local file the repo will never track:

```toml
# .cargo/config.toml (repo root, or any subcrate). Gitignored.

# Faster linker. Prereq: lld on PATH.
#   macOS: brew install lld
#   Linux: apt install lld  (or dnf / pacman / etc.)
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

Other useful opt-ins (install per contributor, configure in the same file):

- `sccache` — shared compile cache. After `cargo install sccache`, export `RUSTC_WRAPPER=sccache` in your shell. Big win on cold rebuilds.
- `mold` (Linux only) — faster linker than `lld`. After `apt install mold`, swap `-fuse-ld=lld` for `-fuse-ld=mold` in the Linux target sections.
- `-C target-cpu=native` — append to any `rustflags` array for binaries you'll never ship to other machines (dev-loop wins, breaks portability).

Install the dev tooling (one-shot):

```bash
just tools-install
```

That brings in `cargo-nextest` (fast test runner), `cargo-deny` (dep audit), and `cargo-insta` (snapshot review). Recipe lives in [`justfile`](../justfile); add new dev tools by extending the recipe.

Smoke test the build:

```bash
just build
just test
```

If `just test` is green, the workspace compiles and every unit + integration test passes. You are ready to edit.

Test runner flavours:

- `just test` — `cargo test --workspace`. The portable default; no nextest dependency.
- `just test-fast` — `cargo nextest run --profile dev-fast`. Aggressive fail-fast inner loop.
- `just test-scope` / `just test-lsp` / `just test-cli` — per-crate, when you want to bound the run.

nextest profiles are declared in [`.config/nextest.toml`](../.config/nextest.toml) — `dev`, `dev-fast`, and `ci` are checked in.

`just test` reports `1 ignored` — `test_incremental_performance` in [`gumiho-mudang-cli/tests/integration/test_incremental.rs`](../gumiho-mudang-cli/tests/integration/test_incremental.rs) asserts a < 2.5 s incremental-index budget that only the release-profile `mudang` binary satisfies. Opt in when auditing performance:

```bash
cargo test -p gumiho-mudang-cli --release --test test_incremental -- --ignored
```

---

## Pre-commit checklist

Before pushing any branch:

```bash
just gate          # fmt-check + clippy + test (fast loop)
just gate-refactor # 16 architecture gates (slower, mandatory)
```

`just gate-refactor` is the architecture contract — every gate is owned by an R-entry in [`docs/ENFORCEMENT-MAP.md`](docs/ENFORCEMENT-MAP.md) and listed in [`docs/CI-GATES.md` § Gate inventory](docs/CI-GATES.md#gate-inventory). If a gate fails, [`docs/CI-GATES.md` § Where to look when a gate fails](docs/CI-GATES.md#where-to-look-when-a-gate-fails) is the recovery guide.

Allowlist tags for the R12 spawn / network / fs gates live in [`docs/CI-GATES.md` § Allowlist convention](docs/CI-GATES.md#allowlist-convention-r12-spawn--network--fs-gates). The tag goes on the line immediately preceding the denylisted construct — wrapping the construct in a helper to evade the gate is forbidden.

---

## Change → test mapping

Quick reference. Run the narrowest scope first; `just gate-refactor` is always the final guard.

| You edited … | Run first | Why |
|---|---|---|
| `gumiho-mudang-scope/scope-core/src/languages/<lang>.rs` | `just test-scope` + `just ci-trait-shape` | Trait-shape audit catches forbidden fn names. See [`docs/CI-GATES.md` row "Trait-shape audit"](docs/CI-GATES.md#gate-inventory). |
| `gumiho-mudang-scope/scope-core/src/extract/<lang>.rs` | `just test-scope` + `just test-malformed` | R6 harness re-runs on fixtures. See [`docs/ENFORCEMENT-MAP.md` § R6](docs/ENFORCEMENT-MAP.md#r6--malformed-source-test-harness). |
| `gumiho-mudang-scope/scope-core/src/languages/dispatch.rs` | `just ci-dispatch` | Dispatch fns must live only here per R7. |
| `gumiho-mudang-scope/scope-core/src/frameworks/<name>/` | `just test-scope` + `just ci-patterns` + `just audit-confidence` | Pattern catalog audit + R8 audit subcommand regression. |
| `gumiho-mudang-scope/scope-graph/src/sql/schema.sql` | `just gate-refactor` | Schema bumps cross multiple R-entries; full sweep first. |
| `gumiho-mudang-cli/src/output/` or `commands/` | `just ci-output-schema` + `just test-cli` | R10 banned field-name set; CLI integration tests. |
| Any `.scm` query in `scope-core/src/languages/queries/` | `just test-scope` | Tree-sitter query parse + extractor unit tests. |
| A test fixture under `scope-core/tests/fixtures/` | `just test-scope`; if snapshot diff appears, `cargo insta review`. | See [§ Snapshot test workflow](#snapshot-test-workflow). |
| `justfile` recipe rename / addition | `just gate-refactor` to confirm canonical scripts still run; update [`docs/CI-GATES.md`](docs/CI-GATES.md) if a gate's local invocation changes. | Recipes are convenience; canonical script paths live in CI-GATES.md. |
| Anything touching audit scripts, schema definitions, trait surfaces, typed APIs, or dispatch | Update [`docs/ENFORCEMENT-MAP.md`](docs/ENFORCEMENT-MAP.md) in the same commit | Mandatory per [`docs/sprints/README.md` § 7.5](docs/sprints/README.md#75-enforcement-map-update). |

When in doubt, run `just gate-refactor`. It is the source of truth for "did I break the architecture".

---

## Snapshot test workflow

Several suites use [`insta`](https://insta.rs/) for snapshot pinning — most prominently the R6 malformed-source harness ([`docs/ENFORCEMENT-MAP.md` § R6](docs/ENFORCEMENT-MAP.md#r6--malformed-source-test-harness)). When a test mutates the snapshotted output, the run produces a `.snap.new` file beside the pinned `.snap`.

To review and accept changes:

```bash
cargo insta review     # interactive — accept/reject each pending snapshot
cargo insta accept     # accept all pending without review (use sparingly)
```

A snapshot diff that you did not intend is a real regression — investigate before accepting. The R6 corpus documents what each fixture pins in [`scope-core/tests/fixtures/malformed/README.md` § Purpose](scope-core/tests/fixtures/malformed/README.md#purpose).

---

## Adding test fixtures

Three corpora, three contracts. Read the corpus README before adding a fixture — provenance rules differ.

| Corpus | Owner | Read first |
|---|---|---|
| Malformed-source (R6) | Parser-recovery harness | [`scope-core/tests/fixtures/malformed/README.md`](scope-core/tests/fixtures/malformed/README.md) — hand-crafted synthetic, one failure mode per fixture, parseable preamble required. |
| Reference (R8) | Confidence audit subcommand | [`scope-core/tests/fixtures/reference/README.md`](scope-core/tests/fixtures/reference/README.md) — anonymized real-shape code; [§ Anonymization rules](scope-core/tests/fixtures/reference/README.md#anonymization-rules) is mandatory. |
| Framework integration (R5) | Framework dispatch + per-framework predicates | [`scope-core/tests/fixtures/frameworks/README.md`](scope-core/tests/fixtures/frameworks/README.md) — see [§ Current state](scope-core/tests/fixtures/frameworks/README.md#current-state) for what lands when. |

The commit that adds a fixture must reference which rule it covers (per the corpus README) and which contributor-rule checklist items were honoured.

---

## Adding a language plugin

Follow [`docs/LANGUAGE-PLAYBOOK.md`](docs/LANGUAGE-PLAYBOOK.md) end-to-end. The procedure is:

1. Step 1 — adoption trigger (Path A or B).
2. Step 2 — evaluation (ROI worksheet, verdict).
3. Step 3 — depth strategy (surface-only vs depth target).
4. Step 4 — walk the 18 boundaries before writing code ([§ Step 4 — The 18 universal boundaries](docs/LANGUAGE-PLAYBOOK.md#step-4--the-18-universal-boundaries)).
5. Step 5 — implementation ([§ Step 5 — Implementation procedure (within bounds)](docs/LANGUAGE-PLAYBOOK.md#step-5--implementation-procedure-within-bounds)).
6. Step 6 — copy [`docs/languages/_TEMPLATE.md`](docs/languages/_TEMPLATE.md) to `docs/languages/<name>.md`.

Mechanically enforced rules during plugin authoring are listed in [`docs/ENFORCEMENT-MAP.md` § Inventory of constraints](docs/ENFORCEMENT-MAP.md#inventory-of-constraints); discipline-only rules (B1, C2, E3) are listed in [§ Discipline-only rules](docs/ENFORCEMENT-MAP.md#discipline-only-rules).

---

## Adding a framework plugin

Follow [`docs/FRAMEWORK-PLAYBOOK.md`](docs/FRAMEWORK-PLAYBOOK.md). Distinct from language adoption — framework plugins consume `Symbol` + `Edge` rows, never AST ([`docs/ENFORCEMENT-MAP.md` § R5](docs/ENFORCEMENT-MAP.md#r5--frameworkplugin-operates-on-symbols-and-edges-not-ast-graph-only-via-metadata)).

1. Steps 1–2 — trigger + evaluation.
2. Step 3 — version strategy (A / B / C) + unknown-version policy ([§ Step 3 — Version strategy](docs/FRAMEWORK-PLAYBOOK.md#step-3--version-strategy)).
3. Step 4 — walk the gotcha catalogue ([§ Step 4 — Gotcha catalogue](docs/FRAMEWORK-PLAYBOOK.md#step-4--gotcha-catalogue)).
4. Step 5 — implementation order ([§ Step 5 — Implementation order within a framework](docs/FRAMEWORK-PLAYBOOK.md#step-5--implementation-order-within-a-framework)).
5. Copy [`docs/frameworks/_TEMPLATE.md`](docs/frameworks/_TEMPLATE.md) to `docs/frameworks/<name>.md`.

Reserved framework-primitive metadata keys (the only structured language→framework communication surface) are documented in [`docs/LANGUAGE-PLAYBOOK.md` § Metadata schema for framework primitives](docs/LANGUAGE-PLAYBOOK.md#metadata-schema-for-framework-primitives).

---

## Sprint methodology

Non-trivial work runs as a sprint. The methodology is permanent and lives in [`docs/sprints/README.md`](docs/sprints/README.md).

Key entry points:

- [`docs/sprints/_TEMPLATE.md`](docs/sprints/_TEMPLATE.md) — copy to start a new sprint doc.
- [`docs/sprints/README.md` § 5. Branch protocol](docs/sprints/README.md#5-branch-protocol--linear-incremental-atomic-phase-shipment) — branch naming, single-vs-multi-sprint phases, hot-fix protocol.
- [`docs/sprints/README.md` § 6. Commit-message conventions](docs/sprints/README.md#6-commit-message-conventions) — `<type>(scope): …`, `docs(<initiative>): …`, `ci(<initiative>): …`, etc.
- [`docs/sprints/README.md` § 7. CI gate activation](docs/sprints/README.md#7-ci-gate-activation) — flip `planned` → `active` in the same commit that ships the gate.
- [`docs/sprints/README.md` § 7.5 Enforcement-map update](docs/sprints/README.md#75-enforcement-map-update) — mandatory R-entry update gate.
- [`docs/sprints/README.md` § 9. Codex consultation protocol](docs/sprints/README.md#9-codex-consultation-protocol) — review checkpoint + implementation-doubt consultation.

If during work you hit ambiguity in a governing doc, **halt and amend the doc**, not the sprint — [§ 3. Ambiguity protocol](docs/sprints/README.md#3-ambiguity-protocol--consult-the-human-amend-the-source-doc) is the rule.

---

## Codex review checkpoint

Every sprint runs `codex review` before closing — see [`docs/sprints/README.md` § Role 1 — Mandatory sprint review checkpoint](docs/sprints/README.md#role-1--mandatory-sprint-review-checkpoint) for the canonical invocation and focus checklist.

Setup notes:

- Codex CLI is installed as a Claude Code plugin (`openai-codex`). The plugin ships the `codex` binary plus skill definitions for the rescue + review flows. Install via the Claude Code plugin marketplace and authenticate with a ChatGPT account that has access to `gpt-5.5`.
- The `gpt-5.5-medium` model **variant** is not accepted by ChatGPT-account-backed Codex — use plain `gpt-5.5` plus `model_reasoning_effort = "medium"` per the canonical command shape.
- `--base` is mutually exclusive with `[PROMPT]`. The review focus checklist lives in the PR body, not in the invocation.

Implementation-doubt consultation (Role 2) is bounded by [`docs/sprints/README.md` § Role 2 — Implementation-doubt consultation](docs/sprints/README.md#role-2--implementation-doubt-consultation). Codex is **not** an authority on rule decisions — those go through the ambiguity protocol.

---

## Commit messages

The conventions are in [`docs/sprints/README.md` § 6. Commit-message conventions](docs/sprints/README.md#6-commit-message-conventions). Summary:

- Sprint-scope implementation: `<type>(scope): <summary>` where `<type>` matches the initiative's convention (`refactor`, `feat`, `fix`, etc.).
- Doc-only updates: `docs(<initiative>): <summary>`.
- CI gate activation: `ci(<initiative>): activate gates for sprint NNNN`.
- Charter amendment (rare, requires the ambiguity protocol first): `docs(charter): <summary>` per [`docs/CHARTER.md` § 11. Amending this charter](docs/CHARTER.md#11-amending-this-charter).

---

## Charter posture you need to keep in mind

Two non-negotiables every contribution upholds:

1. **Single-operator posture.** No backward-compat shims, no dual-read paths, no stored-format version detectors. Wipe + reindex is the canonical migration path. Full rationale: [`docs/CHARTER.md` § Single-operator posture](docs/CHARTER.md#single-operator-posture). The charter-sweep gate ([`docs/CI-GATES.md` row "Charter sweep"](docs/CI-GATES.md#gate-inventory)) refuses re-introduction of the named shim shapes.
2. **Hard limits — Scope will never cross these.** Listed in [`docs/CHARTER.md` § 5. Hard limits](docs/CHARTER.md#5-hard-limits--scope-will-never-cross-these). Crossing one is rejected.

Core invariants that any change must preserve: [`docs/CHARTER.md` § 3. Core invariants](docs/CHARTER.md#3-core-invariants--must-never-break).

---

## Where to put a new note

The decision table lives in [`docs/README.md` § Where to put a new note](docs/README.md#where-to-put-a-new-note). Skim it before writing prose in a doc file — most new notes have a designated home.
