# Research & Design Decisions

## Summary
- **Feature**: `kiro-core`
- **Discovery Scope**: New Feature (greenfield engine crate; full discovery against local sources, no external services)
- **Key Findings**:
  - The host sandbox is the binding constraint: `../openidea/wit/plugin.wit` offers only `read-file` (absolute or project-relative, confined to the canonicalized project root, 16 MiB, unsaved buffer wins), `active-file` (absolute path), `workspace-root`; no listing, no writes. Everything the engine needs must therefore arrive as strings keyed by path, and known feature names must be supplied by the caller.
  - Real cc-sdd requirements documents use four coexisting grammars (`ears-sdd/src/ccsdd.rs`): canonical `### Requirement N` + `#### Acceptance Criteria` `M.`; self-numbered dotted `N.M` items; `### R1 — title` with `- R1.1` bullets; `### Amendment A1 — title (extends Requirement 4)`. All normalise to dotted ids. The superseded → successor rule (`ears-sdd/src/kiro.rs`) resolves successor names from `spec.json.notes` against known feature names, whole-word, longest first.
  - This repo's templates and skills fix the task and design grammar: `- [ ] N.M (P) text` with `- [ ]*` optional marker, `_Requirements:_`/`_Depends:_`/`_Boundary:_` italics lines, a required `## File Structure Plan` in `design.md`, and `## Specs (dependency order)` roadmap lines `- [ ] name -- desc. Dependencies: a, b`. `kiro-spec-status` counts `- [x]` vs `- [ ]`; `kiro-impl` marks tasks `[x]`.
  - A component → file mapping has no single canonical shape in the design template (it shows a directory tree with `#` comments plus `#### Component` blocks). The parser therefore accepts three shapes (table, bold-bullet, tree) and this spec's own `design.md` uses the table shape so the design is a fixture for its own parser.

## Research Log

### Host read semantics and path shapes
- **Context**: `Source::read` takes a project-relative path; the plugin's `active-file` is absolute (Windows-style on this machine); the host accepts both absolute and relative paths in `read-file`.
- **Sources Consulted**: `D:\Source\openidea\wit\plugin.wit`; `D:\Source\openidea\crates\ide-plugin\src\sandbox.rs` (`resolve_in_root`, `MAX_READ_BYTES = 16 << 20`, `read_text`); `D:\Source\openidea\crates\ide-plugin\src\runtime.rs` (`read_file` returns `buffer_text` before disk; `active_file` closure; notification body clipped at 16 KiB; insert clipped at `MAX_TEXT`).
- **Findings**:
  - The host canonicalizes and requires the real path to start with the root; a missing file is an `Err("…: no such file")`, not a trap. The plugin's `Source` impl will map `Err` to `None`, so core must treat `None` as "absent" with a finding, never as fatal.
  - Absolute active-file paths must be made project-relative in core (the only place path logic lives per the brief), comparing the root prefix case-insensitively and treating `\` and `/` as equal; a path not under the root is "outside", not guessed.
  - The 16 MiB read cap bounds the input size the parsers must handle in one call (5 s wall clock, ~5·10⁹ fuel).
- **Implications**: `Source` is `fn read(&self, project_relative_path: &str) -> Option<String>`; `Workspace` caches each read; path normalisation is a pure function in `resolve`; parsers are single-pass.

### cc-sdd requirements grammars
- **Context**: The brief mandates accepting all four grammars from `ccsdd.rs` as the test matrix.
- **Sources Consulted**: `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\ccsdd.rs` (parser + unit tests); `D:\Source\spec-kit-ears-tdd\conformance\cases\kiro-*\project\.kiro\specs\*\requirements.md` (regression inputs); this repo's `.kiro/settings/templates/specs/requirements.md`.
- **Findings**:
  - Criterion recognition has two gates: dotted self-identifying markers (`1.1 text`, `- R1.1 text`) anywhere in a requirement block; bare `M.`/`M)` only under an `Acceptance Criteria` heading or inside an amendment block.
  - Continuation lines join into one logical criterion; a blank line closes the criterion but not the requirement; headings of any level other than the recognised ones close the requirement block.
  - Fences: three or more backticks or tildes; closing fence must be the same character and at least as long.
  - The template adds `**Objective:**` lines and this project adds `_Key: value_` annotations (spec-constraints will define `_Terms:_`, `_Guard:_`, `_Conflicts:_`). Capturing annotations generically in core avoids a later parser change.
- **Implications**: Port the recognition rules (not the code: it carries `PathBuf` and feature-qualified ids) into `requirements.rs`; lift the conformance inputs into `fixtures/grammars/`.

### Successor rule for superseded specs
- **Context**: Visualization wants a superseded → successor chain; the brief says to carry the `kiro.rs` rule forward, resolved only against known names.
- **Sources Consulted**: `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\kiro.rs` (`names_in`, `contains_name`, `classify_phase`, `KIRO_SUPERSESSION_DANGLING`, `KIRO_SUPERSESSION_UNRECIPROCATED`).
- **Findings**: Successors are known names occurring as whole words (name chars: ASCII alphanumerics, `-`, `_`) in `notes`; names sorted longest-first so `feature-two` is not shadowed by `feature`. The "unreciprocated" check scans other specs' prose for the lemma "supersede" — that is a cross-spec check, not model building.
- **Implications**: Core implements successor resolution, chain following with cycle detection, and `SPEC_SUPERSEDED_DANGLING`. The pure name extraction (`successors(meta, known)`) lives in `spec.rs` so `Workspace::load` can emit the dangling finding without importing `resolve` (which sits above `source` in the dependency direction); `resolve` re-exports it. The unreciprocated check is deferred to spec-constraints (it is a check over prose, and it needs every spec loaded).

### Task and design grammar in this repo
- **Context**: `tasks.md` and `design.md` were never parsed by v1.
- **Sources Consulted**: `.kiro/settings/templates/specs/tasks.md`, `.kiro/settings/templates/specs/design.md`, `.opencode/skills/kiro-spec-tasks/rules/tasks-generation.md`, `kiro-spec-status/SKILL.md`, `kiro-impl/SKILL.md`, `kiro-spec-batch/SKILL.md`, `kiro-validate-impl/SKILL.md`.
- **Findings**:
  - Task line: `- [ ] 2.1 (P) description`; optional marker is `- [ ]*`; `(P)` sits outside the brackets; max two levels; `_Requirements: X.X_` lists numeric ids only; `_Depends: 1.2, 2.3_`; `_Boundary: AuthService, API Layer_` names design components.
  - Status/impl/batch skills only distinguish `[x]` and `[ ]`; `[X]` should be accepted as checked for robustness.
  - Design template: `## File Structure Plan` with a `### Directory Structure` fenced tree (comments after `#`) and `### Modified Files` bullets; components live under `## Components and Interfaces` as `#### Name` headings and in a summary table with a `Component` column. No template form names the component of each file explicitly, so an explicit mapping shape must be specified: a table with `Component` and `File(s)`/`Path` headers, or `- **Component**: `path`, `path`` bullets. Tree entries without a mapping are "unassigned design files".
- **Implications**: `tasks.rs` and `design.rs` grammars as recorded in `design.md`; this spec's own `design.md` uses the table shape and becomes a fixture.

### Dependencies for the wasm target
- **Context**: The crate must build for `wasm32-wasip2` with no fs/net/thread/clock.
- **Sources Consulted**: steering `tech.md` (serde, serde_json, toml listed as wasm-safe; `biodivine-lib-bdd` deferred to spec-constraints); `plugins/build.sh` conventions in `../openidea`.
- **Findings**: Only `serde` (derive) and `serde_json` are needed here; both are pure Rust with no platform I/O and are already used by the host's own plugin fixtures. No regex crate: hand-written, line-anchored scanners keep the fuel budget predictable and avoid a large dependency in the wasm build. Running tests on wasm needs a runner (`wasmtime`), configured per target in `.cargo/config.toml` so native builds are unaffected.
- **Implications**: Dependency list fixed at `serde`, `serde_json`; `#![forbid(unsafe_code)]`; clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` in library code.

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Pure engine + Source trait (ports and adapters, chosen) | Parsers and model are pure functions over `&str`; one `Source` port; frontends are adapters | Sandbox-safe by construction; identical results in IDE and CI; trivially testable with an in-memory source | Caller must know feature names; no directory walking in core | Matches steering "engine first, frontends thin" |
| Streaming event parser (pulldown-cmark style) | Full Markdown AST, then model built from events | Handles every Markdown corner case | Large dependency; AST does not know cc-sdd markers, so a second pass is needed anyway; harder to keep line spans | Rejected: marker-driven line scanner is enough and smaller |
| Regex-driven line matching | `regex` crate with anchored patterns per marker | Quick to write | Adds a large crate to the wasm build; backtracking risk on long lines; brief prefers hand scanners | Rejected |
| Model inside the plugin crate | Parse in `plugins/kiro` directly | One crate fewer | CLI would duplicate parsing; untestable natively without the host | Rejected by roadmap decision |

## Design Decisions

### Decision: One shared Markdown line scanner
- **Context**: Four Markdown parsers need the same fence tracking, heading detection, list-item and annotation recognition with identical line numbering (3.1–3.6).
- **Alternatives Considered**:
  1. Each parser re-implements fences/headings (as `ccsdd.rs` and `kiro.rs` each did).
  2. One `md` module producing a typed line stream (`Heading`, `ListItem`, `Annotation`, `TableRow`, `FenceOpen/Close`, `Text`, `Blank`) that the document parsers consume.
- **Selected Approach**: Option 2. `md::scan(text) -> Scan { lines: Vec<ScannedLine>, outline, fences, findings }` in one forward pass; document parsers iterate `ScannedLine`s.
- **Rationale**: Guarantees identical fence and line semantics across documents; keeps each document parser about its grammar only.
- **Trade-offs**: One intermediate vector per document (bounded by input size, acceptable under 16 MiB).
- **Follow-up**: Measure the 16 MiB parse time in release (12.3).

### Decision: Generic annotations instead of per-key parsing
- **Context**: Tasks need `_Requirements:_`, `_Depends:_`, `_Boundary:_` now; spec-constraints will add `_Terms:_`, `_Guard:_`, `_Conflicts:_` on requirements and criteria.
- **Selected Approach**: The scanner recognises any `_Key: value_` line as an `Annotation { key, value, line }`; the tasks parser interprets three keys, the requirements parser interprets none and stores all.
- **Rationale**: Generalisation lens — later specs consume annotations without touching the parser.
- **Trade-offs**: Values stay raw strings; splitting on commas happens in the consumer (tasks parser does it for its three keys).

### Decision: Cross-document link findings live in `join`, invoked by `Workspace::load`
- **Context**: `TASK_BOUNDARY_UNKNOWN`, `TASK_REQ_UNKNOWN`, `TASK_DEPENDS_UNKNOWN` need requirements, design and tasks together.
- **Alternatives Considered**: compute lazily on demand; compute in the tasks parser given the other documents.
- **Selected Approach**: `join::link(reqs, design, tasks) -> Vec<Finding>` is pure over the three document values; `Workspace::load` calls it once after loading a feature and stores the findings on the `Feature`.
- **Rationale**: Keeps parsers single-document; keeps the dependency direction one-way (`join` below `source`).

### Decision: Renderers are generic over a graph model
- **Context**: spec-visualization owns which nodes/edges appear; cli-gate wants the same output natively; core owns Mermaid safety and caps.
- **Selected Approach**: `render::Graph { nodes, edges }` with `mermaid()` and `text()` backends, `Caps`, `findings_text()`, `table()`, `clip_lines()`. No spec-specific graph builders in core.
- **Rationale**: Simplification lens — the only shared concern is syntax safety and truncation.
- **Trade-offs**: spec-visualization must build graphs itself; it was already planning to.

### Decision: Build (not adopt) the parsers
- **Context**: Build-vs-adopt for Markdown and JSON.
- **Selected Approach**: Adopt `serde_json` for `spec.json` (standard, wasm-safe); build the Markdown marker scanner (no crate knows cc-sdd markers; `ears-sdd` parsers are Spec Kit-shaped and carry `std::fs`/`PathBuf`).
- **Rationale**: Dependency weight in the wasm build and the need for line spans on cc-sdd-specific markers.

### Decision: Generic `Workspace<S: Source>` with an in-memory `MemorySource`
- **Context**: Tests, the plugin and the CLI all need a workspace.
- **Selected Approach**: `Workspace<S>` owns its source; `MemorySource` (a sorted map of path → text) ships in the library for tests and for frontends' unit tests.
- **Rationale**: Static dispatch is simplest in wasm; the in-memory source removes the need for a test-only shim in three crates.

## Risks & Mitigations
- Grammar drift in future cc-sdd template updates — fixtures are copied from this repo's templates and this spec's own documents; any template change breaks a golden test and is caught in review.
- Tree-form file plans are ambiguous about component ownership — recorded as unassigned files with an `info` finding; the table shape is the recommended form and the one this project uses.
- Performance on 16 MiB inputs — single-pass scanner with `Vec` preallocation by line count; a release-mode timing test guards 12.3.
- wasm test execution requires a `wasmtime` runner — the build is mandatory in CI; the wasm test run is a documented optional step until the toolchain is confirmed on the CI image.
- Path semantics differ between hosts (Windows drive letters, case) — normalisation lives in one function with explicit tests for `C:\…`, `/…`, mixed separators and root-prefix case differences.

## References
- `D:\Source\openidea\wit\plugin.wit` — host contract (`read-file`, `active-file`, `workspace-root`)
- `D:\Source\openidea\crates\ide-plugin\src\sandbox.rs`, `runtime.rs` — path confinement, 16 MiB cap, buffer-first reads, clipping limits
- `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\ccsdd.rs` — the four requirement grammars (prior art)
- `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\kiro.rs` — successor rule (prior art)
- `D:\Source\spec-kit-ears-tdd\conformance\cases\kiro-*` — regression inputs to lift into fixtures
- `.kiro/settings/templates/specs/*.md`, `.opencode/skills/kiro-spec-tasks/rules/tasks-generation.md` — the cc-sdd grammar the parsers must accept
- `.kiro/steering/{product,tech,structure,roadmap}.md` — boundaries and crate layout
