# Brief: code-constraints

## Problem

Kiro / cc-sdd keeps the *specification* honest: requirements are EARS-shaped, terms resolve in a topic-scoped vocabulary, contradictions are searched with witnesses, and `kiro-cli check` gates all of it in CI. Nothing in the workflow keeps the *implementation* honest. Over a sequence of agent-driven tasks the code base degrades in ways no single task review catches: routines grow past what a reader can hold, one file accretes everything a spec touches, the same routine is written again under another name, a constant or a set of state literals is copied into a dozen files, and data code addresses columns by position. Each piece reads fine on its own; the cost lands on the next agent, whose working set no longer fits, and on the human reviewer, who cannot read every line.

`../scitools-hook` solved this for repositories measured by SciTools Understand: a commit-time gate with per-entity limits, a "never worse than before" ratchet, lexical duplication rules, a scattered-definitions rule, remediation hints an agent can act on, and a rules block written into the agent's instructions. It is Python, licence-bound, and knows nothing about specs. This project needs the same mechanism inside the kiro workflow: measured against the spec that owns the file, gated per task before `[x]`, and fed back to the agent through steering.

## Current State

- `kiro-core` owns the file ↔ component ↔ task join (`join::owners_of`, `join::coverage`), the one `Finding` shape, the code catalogue, text/JSON rendering and a `Source` read port; `kiro-cli` owns a confined filesystem adapter, spec discovery, the `GateReport` envelope, exit codes `0/1/2` and the Dagger CI gate. None of them reads source code.
- The roadmap (amendment 2026-11-04) deferred "idiomatic naming in data code" with the decision that it returns as **vocabulary topics**, not engine machinery. This spec is where it returns: column and field names in data code are grounded in a configured data topic of `.kiro/vocabulary.toml`.
- `../scitools-hook` provides a measured rule catalogue (`docs/reference/rules.md`), measured defaults, algorithms for duplicate blocks and similar-routine families (`analysis/lean/{duplicates,similar}.py`), the scattered-definitions rule (`analysis/structure/definitions.py`), the ratchet argument and the agent contract (`agent-rules`, five skills). It has **no** rule for a state machine spread across files and **no** rule for positional versus role-named data access; both are new here.
- The research log (`research.md`) maps every scitools-hook feature to an open-source pendant, a custom rule, or an explicit deferral.

## Desired Outcome

A code gate, run as `kiro-cli code check`, that:

- measures routines, types and files natively for the project's languages (Rust, Python, TypeScript/JavaScript bundled) and accepts reports from analyzers a team already runs (scitools-hook JSON as the full-fidelity licensed source; jscpd JSON for duplicates in other languages);
- enforces the complexity gate (cyclomatic strict, nesting, lines, statements, parameters; shrink signals as warnings) and the container limits that stop god files and god types, with the ratchet deliberately off for container counts so a split is never refused;
- finds copies and renamed copies: duplicate blocks of twelve or more normalised lines, and one finding per **family** of routines whose token shapes match at 0.9, with the documented blind spot that a re-implementation in another shape is not found and embedding tools remain a periodic report, never a gate;
- finds scattered state: a module-level name bound to the same value in many files (scattered definition), a set of literal states tested in many files (scattered state machine), and a column-name literal used in many files (scattered schema), each with a `collect:` hint naming one home;
- enforces idiomatic data access: columns, fields and axes of arrays and dataframes selected by a role name, never by an integer position, and those names resolving in a configured vocabulary data topic;
- blocks only regression: a per-entity ratchet against a committed baseline or a git revision; pre-existing debt is reported and never blocks; a net statement line per run;
- attributes every finding to the owning spec, component and tasks through the engine's join, offers `--spec` and `--task` selections so an implementer gates one task before marking it complete, and reports `CODE_COVERAGE` so a clean report is never mistaken for an unchecked one;
- writes the effective rules into `.kiro/steering/code-rules.md` (drift-free, regenerated from configuration), opens every hint with one tag (`simplify:`, `split:`, `delete:`, `shrink:`, `collect:`, `name:`, and `fix:` for findings about the run itself), requires a written reason on every relaxation, and amends the `kiro-impl`, `kiro-review` and `kiro-validate-impl` skills so agents run the gate and never configure it away;
- gates this repository itself in CI.

## Approach

Hybrid, in the shape scitools-hook itself uses (a measuring worker that never judges, an analysis layer that never measures):

- **Custom native measurement** where no pendant exists or where the gate must work with no external tool: a tree-sitter based measurer in a native crate for the base metrics of the bundled languages, the token shapes behind the family rule (a port of scitools-hook's documented algorithm with its two exact pruning bounds), module-level bindings for scattered definitions, literal comparison sets for scattered state, and the positional/role-named access patterns.
- **Open-source and existing pendants as report sources**: scitools-hook JSON (every Understand metric including `Essential` and `CountPath`, twelve languages, a licence the user already holds), jscpd JSON (duplicates, 200+ languages). Linters that report breaches without values (ruff, clippy) are documented as complementary and importable later; they cannot feed a ratchet.
- **Pure judgement in the engine**: rule evaluation over measured facts, the ratchet with the stable entity key, configuration parsing, the hint catalogue, finding codes and report shapes live in `kiro-core` as plain data in → findings out, so the plugin can later read a committed code report per spec through `host::read-file`. Measurement stays native-only and never enters the wasm build.

Why this and not the alternatives: adapters alone leave the gate dependent on a licence or on tools that do not exist for the three new rules; a fully custom measurer re-implements twelve languages scitools-hook already covers; a plugin-first design cannot read source trees. Measured defaults are taken from scitools-hook's recorded measurements where the metric is the same and ship as warnings where no measurement exists yet (requirement 2.2).

## Scope

- **In**: native measurer for Rust, Python, TypeScript/JavaScript; the two report adapters; routine, type and file limits with path scopes; duplicate blocks and similar-routine families; scattered definition, scattered state, scattered schema; positional access and column grounding; ratchet, baseline file, `--changed <rev>`, net line; spec/component/task attribution and `--spec`/`--task` selection; `CODE_COVERAGE`, parse-failure handling, source-failure handling; configuration file with reasons; steering block generation; hint tags; JSON and text reports, optional SARIF; skill amendments; fixtures, tests, docs, self-gating in CI.
- **Out**: dependency structure rules (file/architecture cycles, layers, fan-in/out, coupling: a later `structure-constraints` spec; pendants recorded in research); dead-code rules (unused parameter/class/variable, pass-through, single implementation, over-export: a later `lean-code` spec); `recommend` (measure-and-propose limits); the plugin readout of a code report (later; shapes are prepared for it); auto-fixing; any SciTools Understand invocation (the adapter reads scitools-hook's report, it never runs Understand); correctness, security or style checks.

## Boundary Candidates

- **Measurer** (native, tree-sitter): facts only, never a verdict; one adapter per bundled language behind one fact model.
- **Report adapters**: scitools-hook JSON, jscpd JSON → the same fact/finding model; each documents what it cannot supply.
- **Rules** (pure): limits, containers, duplication, scattered-*, data access; each rule owns its "does not report" list and its hint.
- **Ratchet** (pure): entity key, before/after pairing including the signature-change pairing, pre-existing classification, net line, baseline read/write.
- **Attribution** (pure): findings → owning spec/component/tasks via `kiro_core::join`; selection by spec and task.
- **Reports and steering**: text/JSON/SARIF, `CODE_COVERAGE`, the steering block; CLI subcommands `code check|baseline|rules|config`.
- **Agent contract**: skill amendments, documentation of the ladder and the honest limits.

## Out of Boundary

- Changing the `Finding` shape or the IDE host; writing to spec, design or tasks documents; re-deriving file ownership outside `kiro_core::join`; running analyzers (the gate reads their reports).

## Upstream / Downstream

- **Upstream**: kiro-core (join, Finding, catalogue growth path, renderers), cli-gate (`FsIo`, discovery, `GateReport`, exit model, Dagger CI), vocabulary-grounding (scoped resolution for data topics), spec-constraints (`CHECK_COVERAGE` precedent for trust signals; `CheckConfig` precedent for configuration), steering `agent-guidance.md` (the (c) contract this gate extends from specs to code).
- **Downstream**: CI pipelines of kiro projects; the `kiro-impl` loop; a later plugin readout; later `structure-constraints` and `lean-code` specs reusing the measurer and the ratchet.

## Existing Spec Touchpoints

- **Extends**: cli-gate (new `code` subcommand tree reuses its adapter, report envelope and exit codes); kiro-core (catalogue gains the `CODE_*` codes; a pure `code` module). **Adjacent**: vocabulary-grounding (data topic resolution; no change to the TOML format), agent-guidance steering (amended to name the code gate).

## Constraints

- Measurement is native-only; `kiro-core` stays `std`-only, wasm-safe and I/O-free; `deny.toml` bans remain.
- Every default is measured or is a warning; every relaxation carries a reason; an agent may run the gate and must not configure it.
- Deterministic output: no timestamps, `/` paths, sorted findings; byte-identical across runs and platforms.
- Exit codes identical to `kiro-cli check`: `0` clean, `1` findings at or above `--fail-on`, `2` usage/configuration/IO error.
- Lint table of the engine applies (no `unwrap`/`expect`/`panic`/indexing in command paths).
