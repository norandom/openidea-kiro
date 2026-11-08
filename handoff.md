# Handoff — 2026-11-08

## Where things stand

The project is **v2 of spec constraint checking** (successor to the retired `spec-kit-ears-tdd`), on the cc-sdd layout, with one pure engine crate (`crates/kiro-core`) and two frontends (`plugins/kiro` and `crates/kiro-cli`). All seven specs have completed the 3-phase approval workflow (`requirements` → `design` → `tasks`), and **Wave 1 (`kiro-core`) is 100% implemented and verified** (`2ee9b8d`..`c4c442a`).

| Wave | Spec | Req / AC | Components | Sub-Tasks | Spec Phase | Implementation Status |
|---|---|---|---|---|---|---|
| **1** | `kiro-core` | 12 / 100 | 16 | 20 / 20 | `tasks-generated`, approved | **Implemented & verified** (`crates/kiro-core`, 25/25 items `[x]`) |
| **2** | `plugin-core` | 10 / 62 | 12 | 16 / 16 | `tasks-generated`, approved | **Implemented & verified** (`plugins/kiro`, 16/16 items `[x]`, junction active) |
| **2** | `vocabulary-grounding` | 11 / 72 | 13 | 12 / 12 | `tasks-generated`, approved | **Implemented & verified** (`crates/kiro-core`, `plugins/kiro`, 12/12 items `[x]`) |
| **2** | `spec-navigation` | 8 / 57 | 8 | 13 / 13 | `tasks-generated`, approved | **Implemented & verified** (`plugins/kiro`, 17/17 items `[x]`) |
| **2** | `spec-visualization` | 8 / 50 | 7 | 10 / 10 | `tasks-generated`, approved | **Implemented & verified** (`crates/kiro-core`, `plugins/kiro`, 14/14 items `[x]`) |
| **3** | `spec-constraints` | 8 / 66 | 16 | 0 / 24 | `tasks-generated`, approved | Ready (after `vocabulary-grounding`) |
| **4** | `cli-gate` | 13 / 81 | 15 | 0 / 20 | `tasks-generated`, approved | Ready (after `spec-constraints` & `spec-visualization`) |

Roadmap (`.kiro/steering/roadmap.md`) marks `[x] kiro-core`, `[x] plugin-core`, `[x] vocabulary-grounding`, `[x] spec-navigation`, and `[x] spec-visualization`, and keeps the remaining two specs `[ ]` until their implementation waves complete.

## Architecture decisions locked in

- **Engine first, frontends thin**: `crates/kiro-core` (pure, `std`-only, no fs/net/threads/clock, builds for native + `wasm32-wasip2`) ← `plugins/kiro` (OpenIDEA wasm component) and `crates/kiro-cli` (CI gate). Only I/O seam is the `Source` trait.
- **Fresh cc-sdd parsers**; `ears-sdd` is prior art only (never a dependency). `_Boundary:_` names design **components**, not files; file ownership = file → File Structure Plan component → task boundary (`kiro_core::join`).
- **Topic-scoped vocabulary**: `.kiro/vocabulary.toml` terms carry `topic` + `kind`; `spec.json` declares `topics`; every resolving API takes `topics`; cross-topic hits are `Lookup::NotInTopic` → `TERM_TOPIC_MISMATCH`, never a match. This is the fix for v1's cross-topic matching.
- **Annotations proposed for requirements.md**: `_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_` (spec-constraints). Absent annotations → no failures (`CHECK_COVERAGE` info finding reports coverage transparently).
- **BDD in core**: `biodivine-lib-bdd = "0.6"` proven to build and link for `wasm32-wasip2` (spec-constraints `research.md` has the evidence; `deny.toml` permits `wrappers = ["biodivine-lib-bdd", "rand"]`).
- **Host facts verified against `../openidea` source** (recorded in `tech.md`): the IDE passes `arg = null` from every UI surface and **discards the command's `Ok` message**; activation has no project context. Hence: notification is the primary output; feature selection = active-file resolution, then per-spec variant commands `<group>.<name>.<spec>` registered during command calls; budget = 25 + 17·n ids, 13 specs fit under 256.
- **Auto Ontology exports no SKOS**: only JSON (`GET /api/terms` → `TermsPageResponse { terms: [TermListItem], total, … }`; `related_terms` only on the detail endpoint) and YAML. `kiro-cli vocab import --format auto-ontology-json --topic <t>` is the primary path; `related_terms` is never mapped to `broader`. SKOS Turtle is a secondary input.
- `../openidea/.openidea/plugins/` does not exist yet; dev-plugin install uses `mklink /J`; `build.sh` needs a POSIX path (`/c/Users/mariu/Source/openidea-kiro/plugins/kiro` or `/d/Source/openidea-kiro/plugins/kiro`).

## Cross-spec review & Kiro flow amendment history

- **Round 1 & Round 2 (2026-11-04 .. 2026-11-05) — completed**:
  - Issue 7 closed: `phase_label(Option<&SpecMeta>) -> String` moved to `kiro_core::spec` and re-exported by `ws` (plugin-core) and `viz` (spec-visualization).
  - Issue 8 closed: `kiro_core::join::Coverage.unknown_boundary_tasks` traced through `kiro-core` (`9.10`, `design.md`, Task `4.1`) and consumed by `spec-navigation` and `spec-visualization`.
  - `clip_lines(text, max_bytes, hint)` and `deliver(host, output, insert, insert_id)` signatures unified between `kiro-core` and `plugin-core`.
  - `CHECK_COVERAGE` (`info`, AC `1.8`) and `VOCAB_LABEL_CROSS_TOPIC` (`info`, AC `2.15`) reconciled across fixture expectations.
  - `winnow = "0.7"` (with `MAX_GUARD_DEPTH = 64`), `insta`, `deny.toml` (`wrappers = ["biodivine-lib-bdd", "rand"]`), and Wave 1 vs Wave 4 CI steps aligned.

- **Post-Wave-1 review & Kiro-flow amendments (2026-11-08) — completed**:
  1. **Single-subcommand Cargo aliases (`cli-gate` Task 7.4 & `design.md`, `tech.md`)**: Replaced the impossible multi-command Cargo `[alias]` chain with `kiro-check = "run -p kiro-cli -- check --project . --all --fail-on error"` alongside `gate = "nextest run"` in `.cargo/config.toml`; the multi-step sequence lives in `.github/workflows/ci.yml`.
  2. **Headless deferral for manual GUI host checks (`plugin-core` 4.3, `vocabulary-grounding` 5, `spec-constraints` 7)**: Added the same non-blocking deferral clause already present in `spec-navigation` 4.3 and `spec-visualization` 4 so agentic/headless execution builds `dist/plugin.wasm`, verifies the junction and host-mock tests, and records manual GUI verification as deferred when the OpenIDEA GUI is not running.
  3. **Standalone `plugins/kiro` workspace CI gates (`plugin-core` 4.4 & `cli-gate` 7.4)**: Added explicit `--manifest-path plugins/kiro/Cargo.toml` invocations for `clippy`, `test`, and `cargo deny check` alongside the `wasm32-wasip2` release build, since `plugins/kiro` is excluded from the root workspace.
  4. **Ecosystem library reuse (`cli-gate` 5.1 & 5.2, `tech.md`)**: Permitted `toml_edit` / `toml` value formatting (already in `Cargo.lock`) in `crates/kiro-cli` for TOML string escaping and append-only merge.
  5. **Task & roadmap completion rules (`kiro-core/tasks.md`, `workflow.md`, `roadmap.md`)**: Checked off the 5 completed major task headers in `.kiro/specs/kiro-core/tasks.md` so `TasksDoc.counts` reports `25/25 (100%)`, and aligned `.kiro/steering/roadmap.md` checkboxes with actual implementation completion.
  6. **Self-fixture & design filename sync (`kiro-core/design.md`, `crates/kiro-core/fixtures/self/*`)**: Aligned `fixtures/golden/{graph.mermaid,graph.text,graph-truncated.mermaid,findings.text}` and `tests/grammar.rs` in `kiro-core/design.md`, and synced `crates/kiro-core/fixtures/self/*` with the current `kiro-core` spec and `roadmap.md` files.

## Protocol for in-flight spec corrections under the Kiro flow

When an implementation or review step uncovers a spec gap, tooling constraint, or cross-spec friction:
1. **Update the owning spec first (`requirements.md` → `design.md` → `tasks.md`)** before changing code, keeping the 3-phase traceability intact (`_Requirements:_`, `_Boundary:_`, `_Depends:_`, and the `Files by Component` table).
2. **Keep steering & self-fixtures synchronized**:
   - If project-wide policy or dependency rules change, update `.kiro/steering/{tech,structure,roadmap,workflow}.md`.
   - Whenever `.kiro/specs/kiro-core/{spec.json,requirements.md,design.md,tasks.md}` or `.kiro/steering/roadmap.md` is edited, copy the updated file into `crates/kiro-core/fixtures/self/` and re-run `cargo gate` so `self_fixture_components_have_files_and_clean_boundaries` and `self_roadmap_fixture_parses_cleanly` validate the live spec documents.
3. **Checkbox hygiene**:
   - Mark leaf sub-tasks `- [x] N.M` only after TDD (`RED → GREEN → REFACTOR → VERIFY`) and inline `kiro-review` pass.
   - When all sub-tasks under major task `N.` are `[x]`, mark `- [x] N.` in the same commit so `TasksDoc.counts` reflects 100%.
   - When a spec's final validation task completes, mark its entry `- [x]` in `.kiro/steering/roadmap.md` (and sync `crates/kiro-core/fixtures/self/roadmap.md`).

## Suggested order next

1. Run `/kiro-impl spec-constraints` in **foreground manual mode** (per `.kiro/steering/workflow.md`: no subagent dispatch unless explicitly requested) (Wave 3, depends on `vocabulary-grounding`).
2. Finish with Wave 4:
   - `/kiro-impl cli-gate` (Wave 4, depends on `spec-constraints` & `spec-visualization`)

## Useful pointers

- Skills: `.opencode/skills/kiro-*/SKILL.md` (follow inline in the main session per `.kiro/steering/workflow.md`).
- Prior art (read-only): `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\` (`ears.rs`, `ccsdd.rs`, `kiro.rs`, `bdd.rs`, `vocabulary.rs`), `docs/concepts/contradictions.md`.
- Host: `../openidea/wit/plugin.wit`, `crates/ide-plugin/src/{runtime,lib,sandbox}.rs`, `src-tauri/src/commands/plugin.rs`, `src/features/plugins/bridge.ts`, `plugins/{README.md,hello,build.sh}`.
