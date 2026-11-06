# Handoff — 2026-11-03 (end of day)

## Where things stand

The project was re-framed today as **v2 of spec constraint checking** (successor to the retired `spec-kit-ears-tdd`), on the cc-sdd layout, with one pure engine crate and two frontends. All steering was rewritten and **all seven specs were generated and fast-track approved** via the spec-batch workflow. No code exists yet — the repo is greenfield (`crates/`, `plugins/` do not exist).

| Spec | Req / AC | Components | Tasks | State |
|---|---|---|---|---|
| kiro-core | 12 / 98 | 16 | 21 | tasks-generated, approved |
| plugin-core | 10 / 62 | 12 | 16 | tasks-generated, approved |
| vocabulary-grounding | 11 / 71 | 13 | 17 | tasks-generated, approved |
| spec-navigation | 8 / 57 | 8 | 12 | tasks-generated, approved |
| spec-visualization | 8 / 50 | 7 | 10 | tasks-generated, approved |
| spec-constraints | 8 / 65 | 16 | 18 | tasks-generated, approved |
| cli-gate | 13 / 81 | 15 | 21 | tasks-generated, approved |

Roadmap (`.kiro/steering/roadmap.md`) has all seven marked `[x]`. Each spec dir has `brief.md`, `spec.json`, `requirements.md`, `design.md`, `tasks.md`, `research.md`.

## Architecture decisions locked in today

- **Engine first, frontends thin**: `crates/kiro-core` (pure, `std`-only, no fs/net/threads/clock, builds for native + `wasm32-wasip2`) ← `plugins/kiro` (OpenIDEA wasm component) and `crates/kiro-cli` (CI gate). Only I/O seam is the `Source` trait.
- **Fresh cc-sdd parsers**; `ears-sdd` is prior art only (never a dependency). `_Boundary:_` names design **components**, not files; file ownership = file → File Structure Plan component → task boundary (`kiro_core::join`).
- **Topic-scoped vocabulary**: `.kiro/vocabulary.toml` terms carry `topic` + `kind`; `spec.json` declares `topics`; every resolving API takes `topics`; cross-topic hits are `Lookup::NotInTopic` → `TERM_TOPIC_MISMATCH`, never a match. This is the fix for v1's cross-topic matching.
- **Annotations proposed for requirements.md**: `_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_` (spec-constraints). Absent annotations → no failures.
- **BDD in core**: `biodivine-lib-bdd = "0.6"` proven to build and link for `wasm32-wasip2` (spec-constraints research.md has the evidence).
- **Host facts verified against `../openidea` source** (recorded in `tech.md`): the IDE passes `arg = null` from every UI surface and **discards the command's `Ok` message**; activation has no project context. Hence: notification is the primary output; feature selection = active-file resolution, then per-spec variant commands `<group>.<name>.<spec>` registered during command calls; budget = 25 + 17·n ids, 13 specs fit under 256.
- **Auto Ontology exports no SKOS**: only JSON (`GET /api/terms` → `TermsPageResponse { terms: [TermListItem], total, … }`; `related_terms` only on the detail endpoint) and YAML. `kiro-cli vocab import --format auto-ontology-json --topic <t>` is the primary path; `related_terms` is never mapped to `broader`. SKOS Turtle is a secondary input.
- `../openidea/.openidea/plugins/` does not exist yet; dev-plugin install uses `mklink /J`; `build.sh` needs a POSIX path (`/d/Source/openidea-kiro/plugins/kiro`).

## Cross-spec review — status

Round 1 ran; 4 critical/important + 8 minor issues found. **Issues 1–6, 9, 10, 11 were fixed** (canonical module table order `ws, nav, viz, vocab, check` + `run_command_with` test seam in plugin-core; corrected budget arithmetic; vocabulary-backed `Labeler` deferred to the roadmap; Allowed Dependencies lines amended once at the source in kiro-core/plugin-core designs with downstream tasks downgraded to "verify"; six-field `Finding` in steering; `DEFAULT_CAPS` const; `ctx.host.log` exception; cli-gate task 7.2 amends vocabulary-grounding req 10.1; supersession prose check deferred).

**Round 2 (2026-11-05) — completed:**
- **Round-2 cross-spec audit and amendments applied** across all 7 specs:
  - Issue 7 closed: `phase_label(Option<&SpecMeta>) -> String` moved to `kiro_core::spec` and re-exported by `ws` (plugin-core) and `viz` (spec-visualization).
  - Issue 8 closed: `kiro_core::join::Coverage.unknown_boundary_tasks` traced through `kiro-core` (`9.10`, `design.md`, Task `4.1`) and consumed by `spec-navigation` and `spec-visualization` (filtered to leaf tasks when `design.md` is present).
  - `clip_lines(text, max_bytes, hint)` and `deliver(host, output, insert, insert_id)` signatures unified between `kiro-core` and `plugin-core`.
  - `CHECK_COVERAGE` (`info`, AC `1.8`) and `VOCAB_LABEL_CROSS_TOPIC` (`info`, AC `2.15`) reconciled across fixture expectations (`good/clean`, `bad/broken`, `bad/wide`, `valid.toml`, `cli-gate` JSON example).
  - Tacked-on `2026-11-04` post-validation tasks folded into their respective Foundation/Core/Validation tasks.
  - `winnow = "0.7"` (with `MAX_GUARD_DEPTH = 64`), `insta`, `deny.toml` (`wrappers = ["biodivine-lib-bdd", "rand"]`), and Wave 1 vs Wave 5 CI steps aligned.
  - Issue 12 (decomposition): spec-constraints and cli-gate remain single specs per roadmap; split only if implementation slips.

## Suggested order tomorrow

1. Re-run the cross-spec review (round 2). If clean or minor-only, proceed.
2. Decide issues 7 and 8 (both are small kiro-core API additions — cheapest to fold in before kiro-core implementation starts; use `/kiro-validate-design kiro-core` after editing its design.md, or just amend design + tasks directly).
3. `/kiro-impl kiro-core` — the engine; everything depends on it. Prerequisite on this machine: `rustup target add wasm32-wasip2` (not installed; `wasmtime` not on PATH — optional, only the wasm *test* run uses it).
4. Then `/kiro-impl plugin-core`, then the three wave-3 specs (independent), then spec-constraints, then cli-gate.

## Useful pointers

- Skills: `.opencode/skills/kiro-*/SKILL.md` (OpenCode-style; not registered as Claude Code slash commands — read SKILL.md and follow it inline, use subagents for the independent reviewer steps).
- Batch lesson: agent-run tasks phases usually end `NEEDS_FIXES` on one prerequisite/wording bullet. Apply the reviewer's minimal fix to the scratchpad draft yourself and run one fresh Step 3.5 review instead of resuming the generating agent — converged every time.
- Prior art (read-only): `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\` (`ears.rs`, `ccsdd.rs`, `kiro.rs`, `bdd.rs`, `vocabulary.rs`), `docs/concepts/contradictions.md`.
- Host: `D:\Source\openidea\wit\plugin.wit`, `crates\ide-plugin\src\{runtime,lib,sandbox}.rs`, `src-tauri\src\commands\plugin.rs`, `src\features\plugins\bridge.ts`, `plugins\{README.md,hello,build.sh}`.
- mmhmem checkpoint for project `openidea-kiro` (task `openidea-kiro-1793723132942`) mirrors this file; `/mmhmem-recap` restores it.
