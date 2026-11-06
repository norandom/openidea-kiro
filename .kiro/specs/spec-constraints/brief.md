# Brief: spec-constraints

## Problem

The core idea of this project is that specs are checked, not just written: acceptance criteria hold a fixed shape, the words they use are declared and mean one thing, and two requirements that cannot both hold are found before implementation. v1 (`ears-sdd`) did this for Spec Kit and failed in one specific way — term references were matched textually across unrelated topics, so the contradiction search compared requirements that were not about the same thing and the gate lost trust. v2 must re-implement the checks on the cc-sdd layout, on top of topic-scoped vocabulary, with contradiction search that refuses rather than guesses.

## Current State

- `kiro-core` provides criteria `N.M` with text and line spans; `vocabulary-grounding` provides topic-scoped term resolution and term `kind`s.
- cc-sdd has no convention for declaring the terms a requirement is about, nor for guards (the conditions under which a criterion applies).
- `ears-sdd` prior art: `ears.rs` (EARS form; fixed false positives for quoted modals, hyphenated compounds, possessive apostrophes; clause split on the last comma before `shall`), `bdd.rs` (one-hot encoding per term, contiguous blocks, fixed variable order, no dynamic reordering, witness extraction), `docs/concepts/contradictions.md` (bounded-size discipline: refuse, never hang; guards compare a term to a literal only).

## Desired Outcome

`kiro_core::checks` producing findings with stable codes, and `plugin.kiro.check.*` commands surfacing them:

1. **EARS shape** per criterion: exactly one `shall`, one subject, one trigger clause where the pattern requires it; no competing modals; recognised patterns (ubiquitous / event / state / unwanted / optional / complex) reported as info. Codes `EARS_NO_SHALL`, `EARS_MULTI_SHALL`, `EARS_MODAL`, `EARS_NO_TRIGGER`, `EARS_UNPARSED`.
2. **Declared-before-use**: each `### Requirement N` block may carry `_Terms: term-a, term-b_`; terms resolve through the spec's declared topics. Codes `TERM_UNDECLARED` (used in `_Terms:_` but absent from the vocabulary in those topics), `TERM_TOPIC_MISMATCH` (label exists only in a topic the spec did not declare — the v1 failure, now visible), `TERM_DEPRECATED`, `TERM_SUGGESTED` (info: a vocabulary label appears in criterion text but is not declared — suggestion only, never a failure).
3. **Contradiction search**: criteria may carry `_Guard: mode = safe and depth > 1000_` over declared terms of kinds `bool|enum|int`; guards are compiled into a BDD per topic; pairs of criteria in the same topic whose guards are jointly satisfiable while their responses are declared incompatible (`_Conflicts: 3.2_` or opposite literal on the same term) are reported with a witness state. Codes `CONTRADICTION`, `GUARD_UNPARSED`, `GUARD_KIND_MISMATCH`, `SEARCH_REFUSED` (size cap hit — reported as a warning, never as "no contradiction").
4. **Coverage signal** (amendment 2026-11-04): the report states how many criteria carry annotations, how many pairs were considered, found and refused, and records `CHECK_COVERAGE` (info) on the requirements document whenever the term or contradiction check ran, so an unannotated document is visibly unchecked rather than silently green; the documentation states that the search compares only pairs within the spec's declared topics (cross-topic contradictions are a documented blind spot).

Commands: `check.spec [feature]` (all checks, findings as severity-tagged notification + full list in the message), `check.ears`, `check.terms`, `check.contradictions`.

## Approach

- Checks are pure functions over the core model + vocabulary; the plugin and the CLI call the same `run_checks(workspace, feature, config) -> Vec<Finding>`.
- EARS rules written fresh with `ears-sdd`'s documented false positives as regression tests; operate on criterion text with fenced code and quoted literals masked.
- Topic scoping is enforced by construction: the checker asks the vocabulary with the spec's topics only; a spec without `topics` yields `TOPICS_UNDECLARED` and skips term and contradiction checks.
- BDD layer: `biodivine-lib-bdd` if it builds for `wasm32-wasip2`; otherwise a small ROBDD in core, or the search runs only in cli-gate and the plugin reports where to run it — decide in design with a build experiment. Node/variable caps from config; exceeding them → `SEARCH_REFUSED`.
- Conventions (`_Terms:_`, `_Guard:_`, `_Conflicts:_`) are documented as additions to the cc-sdd requirements template; absence of annotations means the corresponding check reports nothing (info), so existing specs keep passing.

## Scope

- **In**: `checks::{ears, terms, contradiction}` with finding codes above; `_Terms:_`/`_Guard:_`/`_Conflicts:_` parsing (in core's requirements parser, specified here); check configuration (caps, severity overrides); `check.*` plugin commands; docs for the annotation conventions; fixtures with known-good, known-bad, and refused cases.
- **Out**: vocabulary loading (vocabulary-grounding); CLI reporting/exit codes (cli-gate); v1's intentions/precedence, supersession envelopes, production-code separation, and test traceability (future spec candidates); auto-fixing or editing specs.

## Boundary Candidates

- `checks::ears` — text-only, no vocabulary needed.
- `checks::terms` — vocabulary + topics.
- `checks::contradiction` — guards + BDD; the only module with a size budget.
- Plugin `check.*` layer — presentation only.

## Out of Boundary

- Rendering or navigation.
- Any cross-topic comparison, ever.

## Upstream / Downstream

- **Upstream**: vocabulary-grounding (scoped resolution, kinds), kiro-core (model), plugin-core (dispatch).
- **Downstream**: cli-gate (runs these checks in CI); future specs for intentions/precedence and envelope comparison.

## Existing Spec Touchpoints

- **Extends**: none. **Adjacent**: vocabulary-grounding (data contract), cli-gate (reporting), kiro-core (annotation parsing lands in its requirements parser).

## Constraints

- No panics; every check is total over arbitrary input and returns findings instead of errors.
- Contradiction search must be bounded (variables/nodes/time budget inside the 5 s plugin call) and deterministic (fixed variable order from the vocabulary, no reordering).
- Findings point at `file:line` of the criterion or annotation; codes are stable and documented.
- Guards compare a term to a literal only (no term-vs-term); `int` kinds are region-split at the constants mentioned.
