# Brief: plugin-core

## Problem

Developers who run cc-sdd/kiro specs in OpenIDEA (sibling repo `../openidea`) cannot see spec state inside the IDE: which feature is active, what phase it is in, what the tasks and boundaries are. They must switch to a terminal agent. OpenIDEA has a sandboxed WASM plugin API (`wit/plugin.wit`, `openidea:plugin@0.1.0`) but no kiro awareness, and no plugin bridges `.kiro` trees to the editor.

## Current State

- `../openidea` loads plugin packages (`plugin.toml` + `dist/plugin.wasm`, component model, wasmtime). Host functions: `log`, `register-command` (→ action `plugin.<id>.<cmd>`), `show-notification`, `workspace-root`, `active-file` (absolute path), `read-file` (absolute or project-relative, project-confined, unsaved buffer wins), `insert-text`, `replace-selection`, `register-language`, `register-theme`. Permissions: `project-read`, `editor-read`, `editor-write`. No directory listing, no file writes, no open-file, no network; ~5 s / 5·10⁹ fuel / 256 MiB per call; a trap marks the plugin Failed.
- `run-command(id, arg) -> result<option<string>, string>`: verified against `../openidea/src-tauri/src/commands/plugin.rs` and `src/features/plugins/bridge.ts` (see tech.md): every UI surface (Search Everywhere, keymap, Plugins page) passes `arg = null` and the `Ok` message is **discarded** — only `Err` is shown as "<title> failed: msg". Only `crates/ide-plugin/tests/host.rs` passes an `arg`.
- The sample `plugins/hello` declares its commands statically via `[[contributes.commands]]` and registers them again in `activate`.
- `../openidea/.openidea/plugins/` does not exist yet; dev plugins are loaded from `<project>/.openidea/plugins/*/`.
- `kiro-core` (sibling spec) provides the model, resolution, join and renderers over a `Source` trait.

## Desired Outcome

A `plugins/kiro/` package that builds to a wasm component (`../openidea/plugins/build.sh <abs path>` or `cargo build --release --target wasm32-wasip2`), installs as a dev plugin (junction `../openidea/.openidea/plugins/kiro`), declares and registers every `plugin.kiro.*` command, implements `kiro_core::Source` over `host::read_file`, builds the `Workspace` with known features from `.kiro/steering/roadmap.md` + the active file + arg, dispatches `run-command` by id prefix (`nav.`, `viz.`, `vocab.`, `check.`) to feature modules, and maps every failure to `Err(String)` — never a trap.

The output contract is three-tier by design (amendment 2026-11-04): the **notification** is the summary (clipped below 16 KiB), the **`<cmd>.insert` variant** delivers the full payload into the editor buffer (≤ 1 MiB), and the **command return message / `kiro-cli` JSON** is the durable full record — the IDE discards the `Ok` message, so the insert variant is the only in-IDE way to obtain full output. When a notification is clipped and the command has an insert variant, the clipping trailer names that variant (e.g. `… 12 more lines — run <id>.insert for the full text`).

## Approach

- Rust `cdylib`, `wit-bindgen` against `../../../openidea/wit`, own `[workspace]`, `kiro-core = { path = "../../crates/kiro-core" }`.
- `host_source.rs`: `Source` impl calling `read_file` with project-relative paths; permission errors and missing files become `None`/`Err` not panics. `workspace_root` + `active_file` → normalised relative path for resolution.
- `commands.rs`: `activate` registers all commands (also listed statically in `plugin.toml`); `run_command` parses `arg` (feature name and flags like `insert`), resolves the current spec, calls the owning module, returns a message.
- `present.rs`: findings/lists/graphs → notification (clip below 16 KiB with "… and K more"), `insert-text` (only when asked, ≤ 1 MiB), or the command return message.
- The `arg` question is settled (verified, tech.md): no UI channel passes an argument, so feature selection is active-file-driven resolution first, then per-spec variant commands `<group>.<name>.<spec>` registered during command calls (bounded by the 256-command budget, 25 + 17·n).

## Scope

- **In**: `plugin.toml` (id `kiro`, `openidea-api = "0.1"`, permissions `project-read`/`editor-read`/`editor-write`, static command list); crate layout; WIT bindings; `host_source`, `commands`, `present` modules; error mapping; `activate` with roadmap-based feature discovery; native unit tests for arg parsing, path normalisation and clipping; build + dev-plugin install docs (Windows `mklink /J`, creating `.openidea/plugins`, _Reload Plugins_).
- **Out**: parsing or model (kiro-core); the `nav.*`/`viz.*`/`vocab.*`/`check.*` behaviours (sibling specs register through this dispatch); host changes; spec-file writing.

## Boundary Candidates

- `host_source`: all WIT calls in one place.
- `commands`: registration + routing only; no feature logic.
- `present`: all output clipping/formatting.

## Out of Boundary

- Navigation, visualization, vocabulary, checks (sibling specs).
- Any write to project files.

## Upstream / Downstream

- **Upstream**: `../openidea` host (WIT + sandbox + dev-plugin loading); kiro-core.
- **Downstream**: spec-navigation, spec-visualization, vocabulary-grounding, spec-constraints register commands and use `host_source`/`present`.

## Existing Spec Touchpoints

- **Extends**: none.
- **Adjacent**: kiro-core (model), cli-gate (the other `Source` frontend — keep presentation formats aligned via core's text renderers).

## Constraints

- Sandbox limits and no-panic rule; `panic = "abort"`, `opt-level = "s"`, `lto`, `strip`.
- `read-file` ≤ 16 MiB UTF-8; commands ≤ 256; ids match `[a-z0-9][a-z0-9._-]*` (plugin id itself: `[a-z0-9][a-z0-9_-]*`); titles ≤ 256 chars; notification body clipped at 16 KiB; insert/replace clipped at 1 MiB.
- No `read_dir`: known features = roadmap list ∪ active-file feature ∪ arg feature.
