# Brief: kiro-core

## Problem

Every v2 capability — navigation, visualization, vocabulary, constraint checks, the CI gate — needs the same thing: a reliable, panic-free model of a cc-sdd `.kiro` tree built from strings. v1 (`ears-sdd`) mixed parsing with `std::fs` and targeted Spec Kit tables, so it cannot run in the OpenIDEA wasm sandbox and does not parse this project's cc-sdd layout (`_Requirements:_`, `_Depends:_`, `_Boundary:_`, File Structure Plan, `spec.json` phases). Without one shared engine, the plugin and the CLI would drift.

## Current State

- cc-sdd v3 templates in this repo (`.opencode/skills/kiro-spec-*/`) define the layout: `### Requirement N` + `#### Acceptance Criteria` numbered `M.` (criterion `N.M`); tasks `- [ ] N.M (P) …` with `_Requirements: N.M_`, `_Depends: N.M_`, `_Boundary: Component, Other_` (design **component names**, optional for sequential tasks); `design.md` with a required File Structure Plan (component → concrete file paths) and Mermaid blocks; `spec.json.phase` ∈ `initialized | requirements-generated | design-generated | tasks-generated` with `approvals`; `.kiro/steering/roadmap.md` with `- [ ] name -- desc. Dependencies: a, b` lines.
- `ears-sdd` (`D:\Source\spec-kit-ears-tdd`) is prior art: `ccsdd.rs` documents four requirement grammars seen in real cc-sdd specs (canonical, dotted `N.M` markers, `R1.1` bullets, amendments); `kiro.rs` has the superseded → successor rule (successor named in the spec's notes, matched against known feature names). Neither parses tasks or design; `tasks.rs`/`traceability.rs` are Spec Kit specific.
- The OpenIDEA host provides only string reads of known paths; the CLI can walk directories.

## Desired Outcome

A `crates/kiro-core` library crate, `std`-only, compiling for native and `wasm32-wasip2`, exposing: a `Source` trait and a `Workspace` (known feature names + reads); parsers for `spec.json`, `requirements.md`, `tasks.md`, `design.md` and `roadmap.md`; a spec model (features, phases/approvals/topics, requirements → criteria with line spans, task tree with checkbox/`(P)`/requirements/depends/boundary, components → files, Mermaid blocks); the file ↔ component ↔ task join; current-spec resolution (explicit feature, active file under `.kiro/specs/<feature>/`, roadmap list); a `Finding` type; Mermaid and text renderers. All parsers return partial data plus findings on malformed input and never panic.

## Approach

- Write parsers fresh against fixtures copied from this repo's templates and real specs; use `ears-sdd`'s `ccsdd.rs` grammar list as the test matrix for requirements, and its `kiro.rs` successor rule for `superseded` specs (resolved only against the `Workspace`'s known names).
- Line-oriented, marker-driven parsing (headings, list markers, `_Key: value_` italics lines); fenced code blocks are skipped; every model element records `file` and 1-based `line`/`end_line` for findings and navigation.
- `Workspace::new(source, known_features)` → `load(feature)` reads the four documents lazily; `resolve_current(arg, active_path, workspace_root)` normalises absolute/Windows paths to project-relative before matching.
- Join: `design.file_structure` gives component → files; task `_Boundary:_` gives task → components; `owners_of(file)` returns tasks (and their `_Requirements:_` → criteria) by suffix/normalised-path match, with "component without files" and "task boundary names unknown component" as findings.
- Renderers are `model → String` with size caps (max nodes/edges, ellipsis node), Mermaid-safe ids (no `.` in node ids, quoted labels, no raw `<`/`>`).

## Scope

- **In**: crate layout and root workspace; `source`, `spec`, `requirements`, `tasks`, `design`, `roadmap`, `resolve`, `render`, `finding` modules; fixtures and unit tests (parsers, join, resolution, golden renders, no-panic on garbage input); API docs with the cc-sdd grammar it accepts.
- **Out**: vocabulary model and checks (vocabulary-grounding, spec-constraints); any I/O implementation (plugin-core, cli-gate); command/UX design; writing files.

## Boundary Candidates

- Parsers (`spec`, `requirements`, `tasks`, `design`, `roadmap`): one file type each, `&str` in, model + findings out.
- `resolve`: workspace, current-spec resolution, file/component/task join — the only place path normalisation lives.
- `render`: Mermaid/text backends behind one API, consumed by spec-visualization.

## Out of Boundary

- Term resolution, EARS, contradictions (vocabulary-grounding, spec-constraints) — they consume this model.
- Presentation (notification clipping, insertion) — frontends.

## Upstream / Downstream

- **Upstream**: cc-sdd v3 templates in `.opencode/skills/`; `ears-sdd` as prior art.
- **Downstream**: plugin-core, cli-gate (frontends); vocabulary-grounding, spec-navigation, spec-visualization, spec-constraints (consumers of the model, join and renderers).

## Existing Spec Touchpoints

- **Extends**: none (greenfield; replaces the "kiro model" portion of the former plugin-core brief).
- **Adjacent**: plugin-core (implements `Source` over `read-file`), cli-gate (implements `Source` over `std::fs`).

## Constraints

- No `std::fs`, `std::net`, `std::thread`, `std::env`, `std::time` use; `#![deny(unsafe_code)]`; `cargo build --target wasm32-wasip2` must pass for the crate alone.
- Parsing a 16 MiB document must stay well inside the plugin's 5 s / 5·10⁹ fuel budget — linear passes, no backtracking regexes on whole files (prefer hand-written scanners; if `regex` is used, compile once and keep patterns anchored to single lines).
- Deterministic output: stable ordering (BTreeMap/sorted), no randomness, no timestamps.
- Accept all four `ccsdd.rs` requirement grammars; reject nothing silently — unparsed structure becomes a finding.
