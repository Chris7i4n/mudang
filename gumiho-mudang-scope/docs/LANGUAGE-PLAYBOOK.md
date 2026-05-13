# Language Plugin Playbook

Companion to `CHARTER.md`, `ARCHITECTURAL-REFACTOR.md`, and `FRAMEWORK-PLAYBOOK.md`.

The charter defines what Scope is and is not. The architectural refactor defines the structural closure that mechanically enforces the rules below. The framework playbook governs framework adoption. This document governs language plugin adoption and maintenance.

The charter (sections 5 and 7) states architectural hard limits and per-language IN/OUT examples. This playbook abstracts the universal pattern: a single set of rules that **any language plugin must respect**, regardless of which language it targets. The rules are the answer to one question — "what makes a language plugin become a worse LSP, and how do we never cross that line?"

Where this playbook says "the rule is enforced mechanically by …", the corresponding move in `ARCHITECTURAL-REFACTOR.md` (R0–R12, closed 2026-05-12) is the implementation. The closure record's inventory tables map each rule to its enforcement class (mechanical / detectable / discipline) and the R-move that owns it.

This document has two purposes:

1. **Procedure for adopting a new language plugin** (analogous to the framework playbook's adoption flow).
2. **Universal boundaries for every existing language plugin** — every change to Rust, Python, Go, TypeScript, Java, C#, or Ruby plugins must respect the 18 rules below.

The guiding principle, as with frameworks, is **on-demand only**. No language is added because it is popular or trendy. A language is added when it is being used in active maintainer work and friction has been logged repeatedly.

---

## Step 1 — Adoption trigger

A language adoption requires evidence that the language belongs in Scope. Two paths produce that evidence; either is sufficient. Both paths feed Step 2 — the ROI worksheet is non-negotiable on either path.

### Path A — Trigger-driven (uncertain candidates)

For languages whose value to the maintainer is not yet obvious. The friction log proves the case empirically.

**Trigger log.** Maintain `docs/LANGUAGE-TRIGGERS.md`. Append an entry whenever Scope cannot answer a structural question about a language file because that language is not yet supported.

Format:

```
- 2026-05-08 | kotlin | agent had to grep for "fun " across android/ to find handlers
- 2026-05-09 | kotlin | trace from controller to repository required reading 8 files
```

**Trigger threshold.** 5+ entries for the same language within 60 days → candidate moves to **Step 2 (evaluation)**. The threshold is higher than for frameworks (5 vs 3) because adding a language is more expensive.

**Trigger discipline.** Log honestly, log immediately, no padding, no aspirational triggers. A language not used in any active project gets no triggers — "I might learn Elixir someday" is not a trigger.

### Path B — Maintainer-asserted (obvious daily-use languages)

For languages the maintainer already uses heavily in active work. Logging 5 friction events in 60 days for a daily-driver is theatre — the maintainer already knows the language belongs in Scope. Path B skips the trigger log and goes directly to Step 2.

**When path B applies.**

- The maintainer uses the language in **active work this week or month** — not "plan to use", not "used to use".
- Friction is predictable: absence of support produces the same kind of pain repeatedly; logging each instance adds no information beyond what the maintainer already knows.

**Discipline.**

- Path B is **opt-in and recorded.** The decision log entry (Step 2) must declare `Path: maintainer-asserted` and name the active projects using the language. Without that record, the adoption looks speculative six months later.
- Path B **does not bypass Step 2.** The ROI worksheet still runs; if build cost exceeds savings, the verdict is REJECT or DEFER. Maintainer-asserted is evidence of *need*, not a fast-track around *cost*.
- Path B is **not** a fast-track for popularity. "Rust is popular" is not a maintainer-asserted reason. "I write Rust every day in projects X, Y, Z" is.

### Choosing between paths

| Situation | Path |
|---|---|
| Daily-driver in active maintainer projects (Rust today, Ruby today) | B |
| Used occasionally in maintainer work, ROI unclear | A |
| Used by external collaborators but not in maintainer's own stack | neither — out of scope |
| Aspirational ("might use someday") | neither — log nothing |

---

## Step 2 — Evaluation

Once a candidate clears the trigger threshold, fill the ROI worksheet.

### ROI worksheet

```
Language: _____________________________
Used in projects: _____________________ (active projects, last-touched dates)
Tree-sitter grammar: __________________ (link, version, maturity assessment)
Sessions per week relevant: ___________
Estimated minutes saved per session: __
Build estimate (days): ________________ (symbols.scm + edges.scm + plugin + tests)
Maintenance estimate (hours per year): _ (grammar updates, regression fixes)
Initial depth target: surface | depth
Verdict: BUILD | DEFER | REJECT
Notes: _________________________________
```

### Verdict matrix

Same formula as framework playbook:

- `annual_savings_hours = (minutes_per_session × sessions_per_week × 50) / 60`
- `total_cost_hours = build_days × 8 + maintenance_hours_per_year`

Decide:

- `annual_savings > total_cost` → **BUILD**
- `0.5× total_cost < annual_savings ≤ total_cost` → **DEFER** (re-evaluate in 90 days)
- `annual_savings ≤ 0.5× total_cost` → **REJECT**

### Decision logging

Log the verdict in `docs/LANGUAGE-DECISIONS.md` with the same format as `FRAMEWORK-DECISIONS.md`.

---

## Step 3 — Depth strategy

Languages adopted into Scope have one of two depth levels:

### Surface-only

The plugin indexes basic symbol kinds (function, class, struct, type) and basic edge kinds (calls, imports, contains). No language-specific depth work beyond what the universal boundaries (Step 4) and the existing plugin scaffolding require.

Pick surface-only when:

- The language is used occasionally in maintainer projects but is not a primary daily-driver.
- ROI justifies the index existing but not investing per-language depth.

Surface-only languages do not earn per-language depth feature work. They live in a frozen state — bug fixes only.

### Depth target

The plugin earns post-refactor depth feature work (currently Rust, Python, Go, TypeScript). Per-language items earn investment because the language is used heavily and the per-language depth pays back.

Pick depth target when:

- Language is a primary daily-driver in active projects.
- Triggers continue to accumulate even after surface-only support exists.
- Per-language depth items have measurable ROI.

A language can be promoted from surface to depth (or demoted) via amendment to `LANGUAGE-DECISIONS.md`. Promotion queues depth feature work for the language (resumed only after `ARCHITECTURAL-REFACTOR.md` ships). Demotion freezes existing items but does not retroactively remove them.

---

## Step 4 — The 18 universal boundaries

These rules apply to **every language plugin**. A change that violates any of them is rejected, regardless of the language.

The rules are grouped into six categories. Each category corresponds to a known mode by which a language plugin degrades into a worse LSP.

### Category A — Type system

A1. **No type inference.**
The plugin captures declared types as text and as `references_type` edges to symbol IDs. The plugin does not compute the inferred type of any expression. If the type is not written in the source, Scope does not know it.

A2. **No constraint solving.**
Trait bounds (`T: Send + Sync`), conditional types (`T extends U ? A : B`), mapped types, generic constraints, type predicates — none of these are evaluated. The plugin captures the syntactic constraint as text; it does not resolve it.

A3. **No name resolution that requires the type system.**
Overload resolution (which `foo` does `obj.foo()` call when there are three `foo` methods on parameterized types?), method dispatch on generic instantiations, and any resolution that requires knowing a value's actual type at use-site — out. Static visibility, symbol table lookup, and lexical scope walking are in.

### Category B — Runtime semantics

B1. **No flow analysis.**
The plugin does not narrow types via `isinstance` / `instanceof`, does not track variable values, does not perform constant propagation, does not analyze control flow for path-dependent behavior.

B2. **No runtime / dynamic resolution.**
`getattr`, `setattr`, `eval`, `exec`, reflection, dynamic dispatch outcomes, monkey-patching, `__init_subclass__`, metaclass behavior — all out. If knowing the answer requires running the program, the plugin does not produce it.

B3. **No assumption of valid syntax.**
Tree-sitter recovers from syntax errors; the plugin must too. A malformed file produces a partial index, not a panic and not a silent skip. Indexing is deterministic regardless of source code validity.

### Category C — Macros, templates, and version semantics

C1. **No macro / template / preprocessor expansion.**
Definitions are captured. Expansions are not produced. This applies to Rust `macro_rules!`, C `#define`, C++ templates, Python f-strings interpreted as code, Ruby `define_method`, TypeScript decorator factories, and any equivalent.

C2. **No version-specific compiler-quirk modelling.**
The plugin does not track Rust 2018-vs-2021 edition semantic differences, Python 2-vs-3 semantic differences, TypeScript strict-mode behavior, Go generics-aware overload resolution, etc. The plugin reads source as syntactic structure; semantic interpretation belongs to the compiler.

The plugin **does not read** `.ruby-version`, `python_requires` in `pyproject.toml`, the `target` field in `tsconfig.json`, the `edition` in `Cargo.toml`, the `go` directive in `go.mod`, or any equivalent. A plugin that conditioned its extraction on "is this a Python 2 file" would have to model Python 2 semantics, which is the compiler's job. The mechanical safeguard for this rule is `LanguageWorkspaceContext` (`ARCHITECTURAL-REFACTOR.md` R4 split): the language-facing context trait deliberately omits any accessor for these fields, so reading them is a compile error inside a language plugin. Adding such an accessor is a charter-amendment-grade change.

A single language plugin handles every language version that its pinned tree-sitter grammar parses (typically a syntactic superset across major versions — see `CHARTER.md` section 7 "Multi-version posture"). Newer-version syntax in older sources simply does not appear; older syntax in newer sources is recovered when grammar still recognises it.

**Asymmetry with framework version**: rule C2 governs **language** version. **Framework** version branching is allowed and expected — framework predicates use `Detection.version` (`ARCHITECTURAL-REFACTOR.md` R5) to handle Rails 5 vs 7, Express 4 vs 5, etc. The split is intentional: language semantics are the compiler's territory (out of Scope per CHARTER section 5); framework patterns are the maintainer's working surface (in scope per CHARTER section 6, governed by `FRAMEWORK-PLAYBOOK.md`).

### Category D — Resolution discipline

D1. **No cross-file resolution beyond what config files declare.**
Module hierarchy is derivable from filesystem layout plus config files (`Cargo.toml`, `package.json`, `tsconfig.json`, `pyproject.toml`, `go.mod`). The plugin does not "search the world" or guess based on naming conventions outside these constraints.

D2. **No "best guess" fallback resolution.**
When multiple candidates exist for a name, the resolution pass (R3 in `ARCHITECTURAL-REFACTOR.md`) sets `status='ambiguous'` and writes one row per candidate target (multiplicity is allowed because R0 makes the edge PK a surrogate `edge_id`). The extractor's `confidence` is preserved through resolution **as-is**: a clean syntactic pattern (`class Foo extends Bar`) keeps `confidence='high'` even when the workspace has multiple visible `Bar` symbols, because confidence describes the **pattern's precision** while status describes the **lookup outcome** — they are orthogonal and both columns must be queried by the consumer that wants the cleanest signal (`confidence='high' AND status='resolved'`). The plugin never silently picks one candidate and writes a `resolved` edge. Honest ambiguity beats false certainty. (An earlier wording of D2 collapsed ambiguous-target into `confidence='medium'`; the post-R3 split distinguishes the two columns and is the version that governs new code.)

D3. **No symbol-id collision resolution by guessing.**
If two symbols collide on `file::name::kind::line` (the implementation in `src/core/parser.rs:220` includes the declaration line as a uniqueness disambiguator), that is a real ambiguity (or a bug). Record both with a disambiguating qualifier in metadata, or mark the symbol as ambiguous. Do not smooth over the collision.

### Category E — Output discipline

E1. **No semantic correctness assertions.**
The plugin does not say "this code is wrong", "this won't compile", "this has a borrow error", "this type is unused." Diagnostic output is LSP and linter territory. Scope records what is in the source; it does not judge it.

E2. **No metadata interpretation.**
The plugin captures decorator arguments, attribute values, annotation text as raw structured data. It does not interpret the meaning of those arguments. (Interpretation lives in framework plugins, which are layered on top of language plugins, and respect their own playbook.)

E3. **No heuristic optimization for hot paths.**
Indexing output is deterministic. The plugin does not "skip this if it looks like generated code", does not "approximate when the file is big", does not "use a faster but less accurate parser when memory is tight." Either index correctly or skip the file cleanly with a recorded reason.

### Category F — Architecture discipline

F1. **No multi-pass semantic analysis inside the plugin.**
Plugins are one-pass extractors (Appendix A of charter). If a feature requires re-walking with new information, that feature lives in the cross-cutting **resolution pass** (R3 in `ARCHITECTURAL-REFACTOR.md`, enforced via type-state pipeline ordering), not inside the language plugin.

F2. **No write-back to source.**
The plugin reads files. The plugin never writes to source files. Refactoring, code generation, and formatting are out of scope permanently — those are editor and compiler-toolchain features.

F3. **No file-format parsing beyond tree-sitter and a plain-text fallback.**
A plugin that wants to read embedded YAML, JSON, or another structured format inside a source file must defer to a **config reader** (R4 in `ARCHITECTURAL-REFACTOR.md`: `WorkspaceContext` is the only typed access path). The plugin does not import a YAML parser to understand a Rust attribute that contains YAML; it captures the attribute as text and lets a config reader do its job.

F4. **No language detection by content sniffing beyond extension and shebang.**
The plugin is invoked when the file extension or shebang matches. It does not try to parse a `.txt` file as Rust to "see if it fits." Detection is the indexer's job, not the plugin's.

### Why these 18 and not others

Each rule closes a known degradation path. A language plugin that violates rule An becomes a "type checker pretending to be quick." Rule Bn → "interpreter that reads code without running it." Rule Cn → "compiler frontend that misses half the rules." Rule Dn → "tool that lies about certainty." Rule En → "linter that doesn't know its own limits." Rule Fn → "monolith that can't be maintained."

Each rule maps one-to-one to a known failure mode. Together they define the negative space that Scope's language plugins occupy.

---

## Step 5 — Implementation procedure (within bounds)

When a new language is approved for adoption (BUILD verdict, depth target chosen):

1. **Locate or pick a tree-sitter grammar.** Verify maturity, license, and that it produces the AST shapes you need. Reject the language adoption if no usable grammar exists.
2. **Write `queries/<lang>/symbols.scm`.** Cover the language's basic symbol kinds: function, class/struct/interface/enum, type alias, constant, module. Skip macro/template definitions if Category C makes them awkward.
3. **Write `queries/<lang>/edges.scm`.** Cover `calls`, `imports`, `contains`, `references_type`, `extends`, `implements`. These are the universal edges; any language-specific edges (e.g., Go's `green_thread_spawn`, renamed from the earlier `goroutine_spawn` draft to fit the 4-kind concurrency taxonomy in `ARCHITECTURAL-REFACTOR.md` R0) are added later in the depth phase. For call-site edges whose target carries semantic anchor information in its arguments (HTTP routes, queue enqueues, env reads, GraphQL operations, pubsub topics), capture the raw argument list as `Edge.args_text` per R0. The extractor caps `args_text` at 2 KB; the resolver skips writing it when the target is a fully-qualified import (Mitigation 1 / 2). Do **not** interpret the captured text at the language-plugin layer — it is a verbatim literal for framework plugins (R5) and downstream consumers to consume. Interpreting at this layer violates rule E2.
4. **Populate `Symbol.metadata` with the three reserved framework-primitive keys when present in source** (see "Metadata schema for framework primitives" below). Without these keys, no framework plugin can match against this language for AST-shape patterns. This step is mandatory even for surface-only adoption.
5. **Implement `LanguagePlugin` trait.** Two shapes coexist depending on whether `ARCHITECTURAL-REFACTOR.md` R2 has shipped:
   - **Closed-shape methods on `impl LanguageId`** (post-R2 / R7): `symbol_kind_for_node` (the historical `infer_symbol_kind` was renamed at R12 to align with the trait-shape audit — pure node-kind-string-to-symbol-kind-label match expression, no type-system involvement), `scope_node_types`, `extract_metadata`, `extract_edge`, docstring extraction. Per-language behaviour lives in `impl LanguageId` match arms delegating to `scope-core/src/languages/<lang>.rs` modules; there is no `LanguagePlugin` trait and no `*Plugin` unit structs.
   - **Post-R2** (target shape): the trait returns `RawCaptures` (typed bag of capture results, declared metadata, `skipped_ranges`) and a separate `Extractor` layer converts those into `Edge::builder()` calls. Plugins do not emit edges directly. See `ARCHITECTURAL-REFACTOR.md` R2 for the full target shape.
6. **Build 5+ real-world fixtures.** Same discipline as frameworks — anonymized snippets from real maintainer projects, not synthetic toy code.
7. **Run the index, measure precision and recall.** For each edge kind, manually verify a sample. Edges that fail precision target get downgraded to `medium` or skipped.
8. **Stop when**:
   - Surface-only target: `calls`, `imports`, `contains` work on real fixtures with > 80% precision and the symbol set covers > 80% of relevant top-level definitions; the three reserved metadata keys (`decorators`, `annotations`, `template_calls`) are populated where AST exposes them.
   - Depth target: same as surface-only baseline plus per-language depth feature work queued for later sprints (resumed only after `ARCHITECTURAL-REFACTOR.md` ships).

### Metadata schema for framework primitives

Framework plugins are forbidden from accessing AST or running their own `.scm` queries (R5 graph-only via metadata). The mechanism is that the language plugin populates three reserved keys in `Symbol.metadata` (JSON column on every symbol). These keys are the only **structured** language→framework communication surface; framework plugins read every other dimension (symbol name, kind, calls edges, etc.) directly off `Symbol`/`Edge` rows.

| Key | Shape | When to populate |
|---|---|---|
| `decorators` | `[{name: string, args_text: string?}]` | AST `decorator` nodes — Python `@decorator(...)`, TypeScript `@Decorator(...)`, anywhere the grammar exposes a dedicated decorator node |
| `annotations` | `[{name: string, args_text: string?}]` | AST `annotation` / `attribute_item` nodes — Java/C# annotations (`@Override`, `[Authorize]`), Rust attributes (`#[derive(...)]`, `#[tokio::main]`) |
| `template_calls` | `[{name: string, args_text: string?}]` | AST template/component-call nodes — JSX components in TS/TSX (`<Foo prop={x} />`), ERB partial calls in Ruby (`render :user`), Jinja `{% include %}` / `{% extends %}` in Python, HEEx function components in Elixir (`<.user_card user={@user} />`), Razor in C#, Slim/Haml in Ruby, etc. The `name` is the called template/component name as written. The key is template-system-agnostic by design; naming it after one syntax (e.g., `jsx_renders`) would violate the polyglot single-graph invariant (CHARTER §3 invariant 4). |

`args_text` here is the **nested** field inside each metadata entry (decorator / annotation / template-call) — raw argument text for that specific instance. It is **distinct from `Edge.args_text`** (the top-level column on `edges`, R0), which carries call-site argument literals for outbound edges (`http_route`, `queue_handler`, etc.). The two share the name `args_text` because both record raw argument literals captured by the language plugin, but they live on different rows: nested `args_text` lives inside `symbols.metadata` JSON; column `args_text` lives on `edges`. Same E2 rule applies to both — the language plugin captures the literal verbatim; interpretation lives at the framework layer (R5).

If the language has no concept matching a metadata key (no decorators, no annotations, no templates), the key is omitted (not present in JSON), not set to an empty array — that distinction lets the audit detect "language did not implement this surface" vs "AST has no instances".

**Why these three keys.** All three correspond to dedicated AST node shapes — capturing them is purely structural and language-plugin-safe (no E2 interpretation). Each key is a noun-phrase capturing what the AST literally said (decorators applied to a symbol; annotations applied to a symbol; template/component calls inside a symbol's body). The corresponding *resolved* relationships live in the **edge** layer (`http_route`, `renders`, etc.) and are emitted only when a framework predicate matches — never by the language plugin. Pre-resolution (metadata) and post-resolution (edge) are separate stages; see `ARCHITECTURAL-REFACTOR.md` R3 (resolution) and R5 (framework match).

**Why not a `hooks` key.** An earlier draft included `hooks` (React-style `^use[A-Z]` calls) as a reserved key. It was removed because applying a regex to a function name to decide "this is a hook" interprets a naming convention — exactly what E2 forbids the language plugin from doing. React, Vue's composition API, and any other hook-style framework are matched at the framework-plugin layer, where the predicate is allowed to apply naming-convention regexes to `Symbol.name` and `edges.kind='calls'` rows.

**Why `template_calls` and not `jsx_renders`.** Earlier draft used `jsx_renders` as a reserved key. The name was specific to one templating syntax (JSX) but the underlying AST shape — "this symbol invokes another template/component by name with an argument blob" — is universal across ERB, Jinja, HEEx, Razor, Slim, Haml. Naming a reserved key after one syntax breaks the polyglot single-graph invariant: every Ruby/Python/Elixir plugin would either leave the key omitted or fight the name. Rename to `template_calls` makes the key uniformly populatable when each language's template plugin ships.

Populating these keys is the **primary** way a framework plugin can react to AST-shape patterns. Skipping a key disables every framework plugin that would have used it, with no fallback. Naming-convention patterns (hook prefixes, callback naming schemes) are matched separately by framework plugins against `Symbol.name` directly.

### Concurrent verification: walk Step 4

Before declaring a language plugin done, walk through the 18 boundary rules. For each rule, the plugin's behavior must be one of:

- **Trivially compliant** (the plugin does not even attempt the forbidden behavior).
- **Compliant by deliberate design** (the plugin had a tempting shortcut but explicitly rejected it; document in `docs/languages/<name>.md`).

A plugin that is "probably compliant but we didn't check" is not done.

---

## Step 6 — Per-language doc template

Every adopted language has a companion document at `docs/languages/<name>.md`:

```markdown
# Language: <name>

## Tree-sitter grammar
- Crate / package: tree-sitter-<name> (version)
- Maturity assessment: ...
- Known grammar gaps: ...

## Depth target
- surface | depth
- Post-refactor depth queue: yes | no (depth feature work resumes only after `ARCHITECTURAL-REFACTOR.md` ships)

## Symbol kinds emitted
- function | class | struct | enum | trait | interface | type_alias | constant | module | macro | property
  (with notes per kind)

## Edge kinds emitted
- calls, imports, contains, references_type, extends, implements
  (with confidence rationale per kind)

## Universal boundaries — compliance log
For each of the 18 rules in LANGUAGE-PLAYBOOK Step 4, briefly note compliance:
- A1 (no type inference): trivially compliant — type hints captured as text only
- A2 (no constraint solving): trivially compliant — trait bounds captured as metadata
- A3 (no type-system name resolution): compliant by design — overload resolution rejected;
  see commit <sha> for the temptation that was rejected
- B1 (no flow analysis): trivially compliant
- ... (one line per rule)

## Known gotchas
1. ...
2. ...

## Test fixtures
- tests/fixtures/languages/<name>/ — real-world fixtures
- tests/integration/test_<name>.rs — integration test entry point

## SUNSET (filled in only when sunset)
- Date, reason, last supported grammar version
```

A `docs/languages/_TEMPLATE.md` mirrors this structure.

---

## Step 7 — Maintenance triggers

After a language plugin ships, watch for:

### Tree-sitter grammar update

- Grammar releases new version. Re-run fixtures.
- AST shape may have shifted; queries may need updating.
- Log the update outcome in `docs/languages/<name>.md`.

### Test fixture failure after dependency bump

- Patch the queries or extractor.
- A non-trivial patch (more than a few lines) gets recorded in the gotcha section.

### Tempted to violate a Step 4 rule

- This is the most important trigger. When implementing a feature for a language, you may notice a "shortcut" that would cross one of the 18 boundaries.
- Reject the shortcut. Record the temptation in the language's gotcha doc as a "rejected approach" entry. Future-you reads it and understands why the plugin does not do the obvious-looking thing.

### Depth promotion request

- If triggers accumulate against a surface-only language, evaluate promotion to depth target.
- Promotion queues post-refactor depth feature work for the language. Demotion freezes existing items.

### Surface plugin unused for 12 months

- Mark dormant. Consider sunset.

---

## Step 8 — Sunset procedure

When a language plugin is no longer worth maintaining:

1. **Document** the decision in `docs/languages/<name>.md` SUNSET section.
2. **Move plugin code** from `src/languages/active/` to `src/languages/archived/` or feature-gate.
3. **Existing indices retain symbols and edges** — do not retroactively delete.
4. **Future indexing skips the language**.
5. **Remove fixtures and tests** after one release cycle.

### When to sunset

- Language removed from all active maintainer projects.
- Tree-sitter grammar abandoned upstream.
- Repeated grammar-update breakage with no maintainer interest in fixing.
- 18+ months of dormancy.

---

## Step 9 — Discipline and anti-patterns

### Anti-patterns

- **Adding a language for completeness.** Completeness is not a goal; coverage of the maintainer's actual stack is.
- **Adding a language because tree-sitter has a grammar.** A grammar's existence does not mean the language belongs in Scope.
- **Speculating depth before surface stabilizes.** A new language ships surface-only first. Depth comes after triggers prove the case.
- **Crossing a Step 4 boundary "just this once."** The 18 rules are not negotiable per-feature. If a feature requires crossing one, the feature is rejected — full stop. The boundary exists precisely so this question never has to be re-debated.
- **Skipping the per-language doc.** Every adopted language has a companion doc. Without it, six months later you cannot remember why the plugin behaves the way it does.
- **Ignoring grammar maturity.** A new tree-sitter grammar that breaks every release is not ready. Wait or pick another.

### Decision flow summary

```
new pain felt with unsupported language
    → is the language a daily-driver in an active maintainer project?
        → yes → path B (maintainer-asserted) → fill ROI worksheet
        → no  → path A (trigger-driven) → log trigger entry
            → 5 triggers same language within 60 days?
                → no  → keep logging
                → yes → fill ROI worksheet
    → ROI verdict
        → BUILD
            → choose depth target (surface | depth)
            → implement (Step 5) within Step 4 boundaries
            → ship surface; queue depth phase if applicable
        → DEFER → wait 90 days → re-evaluate
        → REJECT → log decision and stop
```

---

## Step 10 — Document index

Working set (static documents that govern decisions):

- **`CHARTER.md`** — what Scope is and is not. Permanent.
- **`ARCHITECTURAL-REFACTOR.md`** — closure record of the structural refactor (shipped 2026-05-12) that mechanically enforces charter hard limits and the 18 rules in Step 4.
- **`LANGUAGE-PLAYBOOK.md`** (this file) — how to add and maintain language plugins within universal boundaries.
- **`FRAMEWORK-PLAYBOOK.md`** — how to add and maintain framework plugins within universal boundaries.

Runtime artifacts (created on demand, updated as decisions occur):

- `docs/LANGUAGE-TRIGGERS.md` — append-only friction log for new-language candidates.
- `docs/LANGUAGE-DECISIONS.md` — verdict log for language adoptions.
- `docs/languages/<name>.md` — per-language gotcha doc with Step 4 compliance log.
- `docs/languages/_TEMPLATE.md` — starting template.

Symmetric structure to the framework working set: one charter, one plan, two playbooks (one per plugin type), and per-instance docs as plugins are adopted.

---

## Closing principle

Every language plugin in Scope is a small machine that turns parsed source code into rows in a graph. Its value comes from being correct, deterministic, and tolerant. Its failure mode is to drift across one of the 18 boundaries and become a half-baked imitation of the language's compiler.

The 18 rules in Step 4 are the negative space that defines what Scope's language layer is. They are not aspirational; they are the contract that every plugin signs and that every change to a plugin must respect.

A language plugin that respects the 18 rules is small, sustainable, and useful. A language plugin that violates one of them is a maintenance liability that compounds over time. The on-demand discipline (Steps 1–3) and the bounded implementation discipline (Step 4) together keep the language layer honest.
