# Research & Design Decisions

## Summary
- **Feature**: `spec-constraints`
- **Discovery Scope**: Extension (light discovery over the three upstream designs) plus one build experiment that decides the BDD question the steering left open.
- **Key Findings**:
  - **`biodivine-lib-bdd` builds for `wasm32-wasip2` and adds no host import.** A scratch crate depending on `biodivine-lib-bdd` (0.5.27 and 0.6.3) compiles for `wasm32-wasip2` with the installed toolchain (rustc 1.98.1); a release `cdylib` with the plugin's profile (`opt-level = "s"`, `lto`, `strip`, `panic = "abort"`) links into a valid WASI preview-2 component (74 959 bytes) whose `wasi:*` imports are exactly those of a baseline `cdylib` without the crate (the `wasi:random/insecure-seed`, `wasi:cli/*`, `wasi:io/*`, `wasi:clocks/monotonic-clock` imports come from Rust's `std` for this target). Its dependency tree (`fxhash`, `num-bigint`, `num-traits`, `num-integer`, `num-rational` in 0.6, `rand` 0.8 with `getrandom` 0.2 → `wasi` 0.11) contains no filesystem, network or thread crate, and `getrandom` is not linked because no code path calls `thread_rng`. The crate is MIT, `rust-version = 1.88`, default features empty (`serde` optional). Decision: use it in core (steering's preferred option), version `0.6`.
  - **The library has what the budget needs.** `Bdd::binary_op_with_limit(limit, left, right, op) -> Option<Bdd>` returns `None` when the result would exceed `limit` nodes; `Bdd::sat_witness()` is a deterministic backward walk from the `true` terminal (same input, same valuation); `BddVariableSet::new_anonymous(n)` panics only for `n ≥ u16::MAX − 1`, far above any cap this design allows. The only other panics are on misuse (operands from different variable sets), which one variable set per feature rules out by construction.
  - **kiro-core needs no parser change.** Requirement 4.8 of kiro-core already records every `_Key: value_` line as an `Annotation { key, value, line }` on the requirement block or, when indented under a criterion, on that criterion, "without interpreting the key". `_Terms:_`, `_Guard:_`, `_Effect:_` and `_Conflicts:_` are therefore read from `Requirement.annotations` and `Criterion.annotations`; the brief's "parsing in core's requirements parser" is satisfied by that generic capture plus value parsing inside `checks`.
  - **vocabulary-grounding's API is sufficient and is the only resolution path.** `Vocabulary::resolve(query, topics) -> Lookup` (Found / NotInTopic / Ambiguous / Unknown / Unscoped), `Lookup::Found { term, replacement, .. }` with `Term.kind: Kind::{Entity, Bool, Enum { values }, Int { min, max }}`, `visible(topics)`, `spec_topics(meta)`, `scope_findings(..)`, `vocab::load(ws) -> Loaded { status, vocabulary, findings }`. The term check calls `resolve` with the spec's topics only; `NotInTopic` becomes `TERM_TOPIC_MISMATCH` and is never a match; `Unscoped` cannot occur because the check is skipped when the topic list is empty.
  - **plugin-core's extension points fit four commands.** `Group::Check` exists; `Positional::Spec` gives `check.spec [feature]`; `Output::with_findings` and `findings_text` render findings exactly as cli-gate will; `CommandContext::{resolve, load, workspace}`; the IDE passes no argument, so every command is spec-addressable and useful with no argument. The budget worked example already counts `check 4` base commands; this design adds one insert variant (`check.spec.insert`), which the worked example did not count (6 → 7 insert ids project-wide; the 256 budget still holds 16+ specs).
  - **Prior art is lifted as test cases, not code.** `ears.rs` (quoted-modal masking, hyphen-adjacent modals, possessive apostrophes, last comma before `shall`), `guard.rs` (grammar, "literal constants only", `=` → `==` hint, kebab-case terms), `bdd.rs` (one-hot blocks, contiguous order from the vocabulary, no reordering, domain constraint, witness decoding) and `docs/concepts/contradictions.md` (budget in sizes not seconds; a refused search is never a pass; group on conflicting effects, not only on shared terms) are re-specified against the cc-sdd model; nothing is imported.

## Research Log

### BDD library on `wasm32-wasip2`
- **Context**: steering lists `biodivine-lib-bdd` "only if it builds for `wasm32-wasip2`" and asks this spec to decide with a build experiment; the brief lists three options (library in core, small ROBDD in core, search only in cli-gate).
- **Sources Consulted**: scratch crates in the session scratchpad (`cargo tree --target wasm32-wasip2`, `cargo build --target wasm32-wasip2` for 0.5.27 and 0.6.3, `cargo build --release --target wasm32-wasip2` of a `cdylib` with the plugin's profile, byte scan of the produced component for `wasi:*` import names, the same scan on a dependency-free baseline `cdylib`); the crate source in the cargo registry (`src/_impl_bdd/_impl_boolean_ops.rs` for `binary_op_with_limit`, `src/_impl_bdd/_impl_util.rs` for `sat_witness`, `src/_impl_bdd_variable_set.rs` for the variable limit, `Cargo.toml` for features and `rust-version`); `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\Cargo.toml` (v1 pinned 0.5.27).
- **Findings**: builds on both targets, including the release component; no import delta against the baseline; `binary_op_with_limit` gives a per-operation node cap without wrapping the library; `sat_witness` is deterministic; variable sets are `u16`-indexed; the crate has no I/O, no threads; `num-bigint` is only used by `exact_cardinality`, which this design never calls. The 0.6 line adds `num-rational` (also unused at runtime after LTO) and requires rustc 1.88; the repository toolchain is 1.98.1.
- **Implications**: `biodivine-lib-bdd = "0.6"` becomes a runtime dependency of kiro-core (a kiro-core revalidation trigger: "adding a required runtime dependency"); the search runs identically in the plugin and in cli-gate; the "small ROBDD in core" and "CLI-only" options are rejected. The node cap is applied through `binary_op_with_limit` on every conjunction; the variable cap is checked before any diagram is built. The plugin's component size is recorded before and after in the README (same procedure as vocabulary-grounding).

### kiro-core extension points for annotations and findings
- **Sources Consulted**: `.kiro/specs/kiro-core/design.md` (RequirementsParser grammar and `Criterion`/`Requirement` shapes, MarkdownScanner annotation rule, Finding catalogue and doc test, Workspace, dependency direction), `.kiro/specs/kiro-core/requirements.md` 4.8, `.kiro/specs/kiro-core/research.md` (annotations captured generically "so a later parser change is avoided").
- **Findings**: an annotation line is `_Key: value_` whose key has no spaces; the value is the raw text between `: ` and the closing `_`; lines indented under a numbered criterion attach to the criterion, others to the requirement; `Blank` closes a criterion, so an annotation must directly follow the criterion's lines. `Criterion.text` has continuation lines joined with one space. `Finding::new(code, file, message)` takes severity from `CODES`; `lib.rs`'s doc test ties the documented table to the catalogue. The dependency direction ends at `source` → {`resolve`, `vocab`}.
- **Implications**: `checks` sits right of `vocab` (`source` → `vocab` → `checks`) and imports `finding`, `requirements`, `spec`, `source`, `vocab`; never `resolve`, `render` or `join`. The documentation states the placement rule (annotation lines directly under the criterion, indented, before any blank line). Values containing `_` are fine because the scanner takes the closing `_` at the line end. Severity overrides are applied after `Finding::new` through `Finding::severity`.

### vocabulary-grounding API consumption
- **Sources Consulted**: `.kiro/specs/vocabulary-grounding/design.md` (VocabScope, VocabLoader, VocabModel, VocabCommands `Scoped` helper, finding catalogue), its research.md (generalisation note: declared-before-use is the same scoped query).
- **Findings**: `resolve` deduplicates hits by term, prefers `Id` over `Label` over `AltLabel`, and returns `replacement` only when in the searched topics; `Kind::Enum { values }` keeps declaration order; `Kind::Int { min, max }` has optional bounds; `visible(topics)` is ordered by (topic, id); `scope_findings` produces `TOPICS_UNDECLARED` (info, `spec.json`) and `TOPIC_UNKNOWN` (warning, `spec.json`); `Loaded.findings` carries `VOCAB_MISSING` / `VOCAB_INVALID` / semantic findings on the vocabulary file. The plugin's `vocab.rs` copies spec name, path and topics out of the `Feature` before `vocab::load(&mut ctx.workspace)` to avoid the borrow overlap.
- **Implications**: the term check receives `&Loaded` and `topics: &[String]`; the report passes `scope_findings` and `Loaded.findings` through once (1.6, 3.8, 3.9); the plugin module follows the same copy-then-load pattern; `run_checks` loads the vocabulary before borrowing the `Feature`. Term suggestions use `visible(topics)` labels and alt labels, normalised, searched as whole-word phrases in the masked criterion text.

### plugin-core extension points and the command budget
- **Sources Consulted**: `.kiro/specs/plugin-core/design.md` (Registry, Args, Present, Context, Commands, budget worked example, message catalogue), `.kiro/specs/vocabulary-grounding/design.md` (VocabCommands as the reference feature module with degradation), `.kiro/specs/spec-navigation/design.md` (budget projection).
- **Findings**: a module implements `FeatureModule`, declares a static `COMMANDS`, is appended to `modules()` and to `plugin.toml`; `Positional::Spec` takes the first token as the spec (4.1); insert is handled by `deliver` with the full text; unresolved spec errors come from `ctx.resolve`; `level_of` maps severities. Budget: base ids ws 2 + nav 6 + viz 3 + vocab 3 + check 4 = 18, insert ids 7 with `check.spec.insert`, spec-addressable 17 → 25 + 17·n; the 7 roadmap specs need 144 ids; the 256 limit is reached at the 14th spec (13 specs fit).
- **Implications**: four `CommandSpec`s, one insert variant, the `check` module file `plugins/kiro/src/check.rs`; outputs use `Output::with_findings` with an intro made of the summary lines; degradation mirrors `vocab.rs` but never short-circuits — the report already contains the explanatory findings, and the module appends the documentation pointer.

### Prior art: `ears-sdd` EARS, guard and BDD modules
- **Sources Consulted**: `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\ears.rs`, `guard.rs`, `bdd.rs`, `docs/concepts/contradictions.md`.
- **Findings**: v1 masked balanced quoted literals (straight, typographic, backtick; single quotes only when not word-internal), ignored modals adjacent to a hyphen, took the last comma before `shall` as the clause split, required `, then ` for `If`, and reported `EARS_SHALL`, `EARS_MODAL`, `EARS_INCOMPLETE`, `EARS_CLAUSE`, `EARS_UNWANTED`, `EARS_PREFIX`. Its guard grammar is `or`/`and`/`not`/parentheses/term/`term op literal` with literals only (comparing two terms is rejected with "literal constants only"; `=` alone is rejected with a hint), kebab-case words. Its BDD encoding is one-hot per term with contiguous blocks declared in vocabulary order, an exactly-one domain constraint conjoined to every query, witness decoding by the first true bit per block, and a scaling test that the domain stays linear in the number of terms. Its documentation insists on a budget in sizes, never seconds, and on reporting an exceeded budget as unchecked work.
- **Implications**: this design keeps the masking rules and the hyphen rule, relaxes the `If` rule (cc-sdd's own template uses both `If …, then` and `If …, the`), splits v1's `EARS_SHALL` into `EARS_NO_SHALL`/`EARS_MULTI_SHALL` with different severities (two `shall`s are common in cc-sdd specs and are a style warning, not a gate failure), accepts `=` as equality and bare enum values because the brief writes guards that way (`mode = safe`), and keeps the BDD encoding rules verbatim as requirements 5.2 and 5.6. v1's `regex` dependency is not needed: whole-word scanning over the masked text is a few `char_indices` loops, consistent with kiro-core's "no regex" rule.

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Pure check functions over `Feature` + `Loaded` behind one `CheckRunner` (chosen) | `check_feature(&Feature, &Loaded, &CheckConfig) -> CheckReport`; `run_checks(&mut Workspace, name, cfg)` is a thin loader on top | Identical results in plugin and CLI; testable without I/O; `CheckReport` carries per-check status for the frontends' summary lines | Every call rebuilds the vocabulary index and the BDD encoding (acceptable: per-command rebuild is the plugin's pattern) | Mirrors kiro-core's "plain data + findings" style |
| Checks as methods on `Workspace` | `ws.check(name)` | Fewer parameters | Couples `source` to `vocab` and `checks` (dependency inversion); harder to test from constructed documents | Rejected |
| `biodivine-lib-bdd` in core (chosen) | Library ROBDD, limited apply, deterministic witness | Proven in v1; no reordering; builds for wasm with no import delta | +5 transitive crates in the wasm build; size recorded | Steering's preferred option, now evidenced |
| Small ROBDD written in core | ~400 lines: unique table, apply with cache, witness | No dependency | New code on the most delicate algorithmic path; v1 had no such code to lift | Rejected because the library builds |
| Search only in cli-gate | Plugin reports where to run it | Smallest plugin | IDE loses the check that motivates the project; two frontends diverge | Rejected |
| One BDD variable set per feature (chosen) over "per topic" | Terms used by accepted guards/effects of the feature, ordered by (topic, id), contiguous blocks | One encoding, one domain constraint; same results as per-topic because pairs never involve terms outside the declared topics and the order is fixed by the vocabulary | A spec declaring many topics encodes them together (the caps bound it) | The brief's "per topic" is honoured at the level that matters: no term outside the declared topics is ever encoded |

## Design Decisions

### Decision: `biodivine-lib-bdd` 0.6 in kiro-core
- **Context**: Steering and the brief defer the choice to a build experiment.
- **Alternatives Considered**: 1. Library in core. 2. Own ROBDD in core. 3. Search only in cli-gate.
- **Selected Approach**: Option 1, pinned `biodivine-lib-bdd = "0.6"` (default features; no `serde`), consumed only in `checks/bdd.rs`.
- **Rationale**: Builds for both targets (see Research Log); limited apply and deterministic witness are library features; no host import delta; plugin and CLI stay identical.
- **Trade-offs**: ~5 extra crates in the wasm build and a size increase to be measured; a dependency with its own panics on misuse (ruled out by one variable set per feature and a cap far below `u16::MAX`).
- **Follow-up**: task records the plugin component size before and after; `cargo tree -p kiro-core --target wasm32-wasip2` is asserted free of `std::fs` users; steering `tech.md`'s conditional sentence is amended to the decision as an integration touch.

### Decision: Annotation set `_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_`
- **Context**: The brief names three annotations and says responses are incompatible by `_Conflicts:_` "or opposite literal on the same term"; a criterion has no place to state the literal it sets.
- **Alternatives Considered**: 1. Encode the response literal inside `_Conflicts:_`. 2. A fourth annotation `_Effect: term = literal_`. 3. Parse the response prose.
- **Selected Approach**: Option 2. `_Effect:_` uses the guard's term and literal rules with equality only; two effects on the same term with different literals are incompatible; `_Conflicts:_` stays a plain symmetric list of criterion ids.
- **Rationale**: Keeps every annotation single-purpose and refusable; matches v1's "effect with conflicts" model without a project-level effects table.
- **Trade-offs**: One more convention to document; absence of it costs nothing (no annotations, no failures).

### Decision: Pairs are compared when incompatible, never otherwise
- **Context**: v1 grouped requirements by shared guard terms and once lost the pair it was looking for; product.md says the search "compares only requirements that share a topic".
- **Selected Approach**: A pair is considered exactly when it is incompatible under 4.10 (declared conflict or clashing effects) and both criteria's guards and effects were accepted. Effect-derived pairs share the effect term's topic by construction; declared pairs are explicit. Every term resolved inside the spec's declared topics, so no comparison crosses an undeclared topic.
- **Rationale**: The question is "can both apply while required outcomes differ"; guards sharing no term is not a reason to skip (the state setting both guards true still forces both effects).
- **Trade-offs**: A spec with many `_Conflicts:_` pairs runs one conjunction per pair; bounded by the annotation count, and each conjunction is node-capped.

### Decision: Caps in sizes, applied per operation
- **Context**: Requirement 5.7/5.8: refuse by sizes, never by time; the plugin has a 5 s budget.
- **Selected Approach**: `max_variables` (default 512 one-hot values) is checked before any diagram exists; `max_nodes` (default 100 000) is passed to `binary_op_with_limit` for the domain constraint, each guard encoding step, each guard-with-domain conjunction and each pair conjunction. A refused operation marks the affected pairs (all pairs when the domain or the variable cap refuses) and produces one `SEARCH_REFUSED` naming the cap, the measured size and the largest blocks.
- **Rationale**: Deterministic on every machine; the library enforces the cap inside the operation, so memory never grows past it.
- **Trade-offs**: A refused pair costs the capped work before refusing; acceptable because the cap bounds it.

### Decision: EARS severities and the `If` rule
- **Context**: Existing specs in this repository routinely write "shall … and shall …" and both `If …, then` and `If …, the`.
- **Selected Approach**: `EARS_NO_SHALL` error; `EARS_MULTI_SHALL`, `EARS_MODAL`, `EARS_NO_TRIGGER`, `EARS_UNPARSED` warning; `EARS_PATTERN` info and off by default (on in `check.ears`); both `If` forms accepted. Teams raise severities through the configuration's overrides.
- **Rationale**: The gate must be adoptable on the existing corpus; cli-gate's `--fail-on` decides what blocks.
- **Trade-offs**: A stricter team configures overrides rather than getting strictness by default.

### Decision: Term suggestions are per requirement and term, info only
- **Context**: The seed vocabulary's labels ("spec", "task") occur in nearly every criterion; per-criterion suggestions would flood the output.
- **Selected Approach**: One `TERM_SUGGESTED` per (requirement, term) at the requirement heading line, listing the criteria where the label occurs; enabled by `suggest_terms` (default true); the known-good fixture declares every term it mentions so it stays finding-free.
- **Trade-offs**: `check.spec` on an unannotated spec with the seed vocabulary shows several info lines; they are the on-ramp to `_Terms:_`.

## Synthesis Outcomes
- **Generalisation**: masking and whole-word scanning serve both the EARS check and the term suggestions (one `CriterionText` component); term-reference matching (4.2) serves guards and effects alike; `binary_op_with_limit` serves the domain, the guards and the pairs.
- **Build vs adopt**: adopt `biodivine-lib-bdd` (evidenced) and `winnow` (adopted 2026-11-04 per `tech.md`; already transitive via `toml` → `toml_edit`, used for the guard/effect combinator lexer and parser with `MAX_GUARD_DEPTH = 64` for the wasm stack); build the masker (no regex) and the finitisation (v1 logic re-specified).
- **Simplification**: no per-topic encodings; no effects table; no cross-spec merge (deferred wave); no clock; no `Scope` object — topics travel as `&[String]`; `check_feature` is the single pure entry point and `run_checks` is a loader around it.

### Decision: Pass-through of vocabulary-grounding findings
- **Context**: Requirement 1.6/3.8/3.9 — which upstream findings a check report carries.
- **Selected Approach**: exactly `VOCAB_MISSING`/`VOCAB_INVALID` (when the vocabulary is not loaded) and `TOPICS_UNDECLARED`/`TOPIC_UNKNOWN` (scope findings), each once and unchanged; semantic vocabulary-file findings are not included because `vocab.topics` owns them and the loaded vocabulary still answers.
- **Rationale**: the report explains why a check was skipped without duplicating the vocabulary command's output.

### Independent design review (pass 1)
- A fresh reviewer applied `design-review-gate.md` against the draft: all 63 criterion ids traced, boundary and file plan complete, upstream contracts conform. Six should-fix items were repaired in one pass: the fixture witness now follows the region rule and the `(topic, id)` witness order (`depth` unbounded above → `depth >= 1001, mode = safe`); criteria referenced only by `_Conflicts:_` are prepared as unconditional so a declared conflict is never lost; `Encoding::new` clamps the variable cap at `VARIABLE_CEILING = 65 000` so the library's `u16` limit can never panic; the pass-through decision above is stated; suggestions exclude per-criterion declared terms (3.7 literally); pattern classification is defined when the trigger comma is missing. Nits (modified upstream paths, fixture wording, constants from accepted criteria only, summary-table alignment, `Nodes` refusal without a measured size, skip-reason precedence, plugin imports, diagram edge, manual self-corpus list) were folded into the same pass.

## Risks & Mitigations
- Plugin size growth from the BDD crate — measured before/after; `opt-level = "s"` + LTO drop `num-bigint`/`rand` code paths that are never called.
- A guard conjunction refused at the cap on a legitimate spec — the finding names the largest blocks; the documentation gives the levers (narrow int constants, split declared conflicts, raise caps in configuration deliberately).
- Annotation placement mistakes (a blank line between the criterion and its `_Guard:_`) attach the guard to the requirement, where it is ignored — the documentation states the placement rule and the known-bad fixture includes the case; a requirement-level `_Guard:_`/`_Effect:_`/`_Conflicts:_` yields `GUARD_UNPARSED` ("belongs under a criterion") so the mistake is visible.
- `EARS_MODAL` on legitimate prose ("may" in a response) — warning severity; overrides available.
- Determinism across targets — all ordering is `BTreeMap`/sorted; the witness comes from a deterministic walk; a test runs the search twice and compares reports byte for byte.

## References
- `.kiro/specs/kiro-core/design.md`, `.kiro/specs/vocabulary-grounding/design.md`, `.kiro/specs/plugin-core/design.md` — consumed contracts.
- `.kiro/specs/cli-gate/brief.md` — expects `run_checks` in kiro-core.
- `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\{ears,guard,bdd}.rs`, `docs/concepts/contradictions.md` — prior art (read-only).
- `biodivine-lib-bdd` 0.6.3 source in the cargo registry (`_impl_boolean_ops.rs`, `_impl_util.rs`, `_impl_bdd_variable_set.rs`) — limited apply, witness, variable limit.
- Scratch crates `bddprobe`, `bddprobe6`, `baseprobe` in the session scratchpad — the build experiment (not part of the repository).
