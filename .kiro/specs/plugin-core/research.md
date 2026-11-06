# Research & Design Decisions

## Summary
- **Feature**: `plugin-core`
- **Discovery Scope**: New Feature (first plugin package of this repository) with integration-focused discovery against the host repository `D:\Source\openidea` (WIT contract, plugin host crate, frontend bridge, sample plugin, build script) and the upstream `kiro-core` design.
- **Key Findings**:
  - **The IDE UI never passes an argument to a plugin command.** The production caller is `src-tauri/src/commands/plugin.rs::plugin_run_command(plugin_id, command_id, arg: Option<String>)`, reached from `src/features/plugins/bridge.ts::pluginActions`, whose action `run` forwards `arg` only when it is a string — which happens only when another frontend feature calls `executeAction("plugin.kiro.<id>", "<string>")` programmatically. Search Everywhere, keymap shortcuts and the Plugins page `Run` button all invoke with `null`. The fallback required by the brief (active-file resolution plus per-spec command variants) is therefore mandatory, and the argument grammar remains for programmatic callers and host tests.
  - **The IDE discards a command's `Ok` return message.** `runPluginCommand` in `src/features/plugins/store.ts` returns the value, but every caller (`bridge.ts` action `run`, `PluginsPage.tsx`) ignores it; only `Err` becomes a notification (`<title> failed: <message>`). The notification is the primary user-visible channel; the return message serves host tests and programmatic callers. (`plugins/README.md` and steering `tech.md` state that the message is shown; the current frontend does not.)
  - **Activation has no project context.** `Runtime::activate` in `crates/ide-plugin/src/runtime.rs` runs with `Invocation::default()`: `workspace-root` returns `None`, `read-file` returns `Err("no project is open")`, `active-file` returns `None`. Roadmap-driven registration cannot happen at activation; it must happen during a command call. `register-command` is allowed in any call and its `commands_changed` effect makes the UI re-fetch the command list.
  - **`register-command` traps on an invalid id or beyond 256 commands**, and gated host functions trap when the permission is missing. The plugin must validate ids and count registrations itself and verify granted permissions at activation.
  - **Dev plugins load from `<open project>/.openidea/plugins/*/`** (`ide_core::project_dir::path(root, "plugins")`), scanned by `PluginHost::sync_project` when the UI fetches the plugin list and only when the project is trusted; directory entries starting with `.` are skipped; a junction works because the scan only checks `<dir>/plugin.toml` is a file. The plugin reads the `.kiro` tree of the project that is open in the IDE, so to exercise kiro specs the junction belongs under the project that owns the specs (this repository), while a junction under `../openidea/.openidea/plugins/` exercises the plugin when the host repository itself is open.

## Research Log

### Argument channel for `run-command`
- **Context**: The roadmap and brief flag `arg` delivery as unverified; the design must confirm or replace it.
- **Sources Consulted**: `D:\Source\openidea\wit\plugin.wit` (`run-command: func(id: string, arg: option<string>) -> result<option<string>, string>`); `D:\Source\openidea\src-tauri\src\commands\plugin.rs` (lines 130–151, `plugin_run_command`); `D:\Source\openidea\src\features\plugins\bridge.ts` (lines 16–35, `pluginActions`); `D:\Source\openidea\src\features\plugins\store.ts` (lines 171–185, `runPluginCommand`); `D:\Source\openidea\src\features\plugins\PluginsPage.tsx` (line 315); `D:\Source\openidea\src\platform\actions.ts` (lines 28–53 `Action`, 158–183 `executeAction(id, arg?)`); `D:\Source\openidea\plugins\README.md` (line 149); `D:\Source\openidea\crates\ide-plugin\tests\host.rs` (lines 129, 151, 374).
- **Findings**:
  - Only one production path reaches `PluginHost::run_command`: the Tauri command `plugin_run_command`, called from the frontend action for each plugin command. The action passes `typeof arg === "string" ? arg : null`.
  - `executeAction(id, arg?)` is the only way to supply a string; no UI element prompts for one. Keymap and Search Everywhere call `executeAction(id)` without an argument.
  - The host test `host.rs` passes `Some("src/notes.txt")` to the misbehave fixture, which is the only in-repo caller supplying an argument.
  - The return value is ignored by every frontend caller; errors surface as `notifications().notify({ level: "error", title: "<title> failed", body })` unless the plugin trapped (then the backend already announced the failure).
- **Implications**: Spec selection must work with no argument: active file first, then per-spec command variants whose id carries the spec name. The notification is the primary output; the return message is kept for tests and programmatic callers. Any command that needs free text (e.g. a vocabulary term id) is reachable only programmatically unless its owning spec designs another input (out of scope here).

### Activation context and command registration
- **Context**: The brief proposes registering per-feature commands "at activation" from the roadmap.
- **Sources Consulted**: `D:\Source\openidea\crates\ide-plugin\src\runtime.rs` (lines 345–365 `activate`/`run_command`, 376–405 `call`, 195–210 `register_command`, 143–153 `require`, 232–255 `workspace_root`/`active_file`/`read_file`); `D:\Source\openidea\crates\ide-plugin\src\lib.rs` (lines 551–582 `start`, 606–609 `apply_effects`); `D:\Source\openidea\crates\ide-plugin\src\manifest.rs` (lines 159–179 id rules).
- **Findings**:
  - `activate` is called with `Invocation::default()` (no client, no root, no buffer function); `read_file` answers `Err("no project is open")`; `workspace_root` answers `None`.
  - `register_command` validates the id (`[a-z0-9][a-z0-9._-]*`, ≤ 64 chars), clips the title to 256 bytes, rejects an empty title, and returns a host error (a trap for the guest) when a new id would exceed 256 registered commands. Re-registering an identical declaration is a no-op; a changed title updates in place. There is no unregister.
  - Registrations made during `run-command` are stored by the host immediately. When the guest returns `Ok`, `CallEffects::commands_changed` → `PluginEvent::CommandsChanged` → frontend refresh. When the guest returns `Err`, `Runtime::run_command` (`runtime.rs` lines 363–364) drops the effects and no event is emitted; the registrations are nevertheless visible after the frontend's `runPluginCommand` calls `refreshPlugins()` in its error handler (`store.ts` line 178). The independent design review flagged this; the design records it as a revalidation trigger and adds a manual host step for it.
  - Permission checks (`require`) return a host error, i.e. a trap that marks the plugin Failed; permissions granted equal the manifest's list.
- **Implications**: Register project-independent commands at activation (identical to the manifest's static list); register per-spec variants during command calls, after the roadmap has been read; track the registered id set locally and never exceed 256; verify the three permissions at activation and fail activation with a clear message if any is missing.

### Dev-plugin loading and build
- **Sources Consulted**: `D:\Source\openidea\crates\ide-plugin\src\lib.rs` (lines 14–15, 45–46 `DEV_PLUGINS_DIR`, 186–213 `sync_project`, 492–499 `get` with project policy, 720–733 `package_dirs`); `D:\Source\openidea\crates\ide-core\src\project_dir.rs`; `D:\Source\openidea\src-tauri\src\commands\plugin.rs` (lines 69–85 `plugin_list`); `D:\Source\openidea\plugins\build.sh`; `D:\Source\openidea\plugins\hello\{Cargo.toml,plugin.toml,src/lib.rs}`; `D:\Source\openidea\plugins\README.md` (sections 2, 5, 6); `D:\Source\openidea\plugins\.gitignore`.
- **Findings**:
  - `plugin_list` calls `sync_project(root)` for the window's project; the project policy requires the project to be trusted (otherwise dev plugins are unloaded and `get` answers "Project is in Restricted Mode").
  - `package_dirs` lists subdirectories containing `plugin.toml` and skips names starting with `.`; a junction named `kiro` passes.
  - `build.sh <abs path>` runs `cargo build --manifest-path <dir>/Cargo.toml --release --target wasm32-wasip2` with `CARGO_TARGET_DIR=<dir>/target` and copies `target/wasm32-wasip2/release/<name with - replaced by _>.wasm` to `<dir>/dist/plugin.wasm`. It needs bash (Git Bash on Windows); the same two steps are reproducible in PowerShell.
  - The hello plugin: `edition = "2024"`, own `[workspace]`, `crate-type = ["cdylib"]`, `wit-bindgen = "0.63"`, release profile `opt-level = "s"`, `lto`, `strip`, `codegen-units = 1`, `panic = "abort"`; `wit_bindgen::generate!({ world: "plugin", path: "../../wit" })` with the path relative to the crate root; static `[[contributes.commands]]` repeated by `register_command` in `activate`.
  - `../openidea/.openidea/plugins/` does not exist and must be created; `plugins/.gitignore` in the host repo ignores `*/target/` and keeps `dist/`.
- **Implications**: The package follows the hello layout with `kiro-core` added by path and `crate-type = ["cdylib", "rlib"]` (the `rlib` is needed so native integration tests in `tests/` can link the crate; the component file name is unchanged). Documentation covers both junction locations, trust, and Reload Plugins.

### Native testability of a wit-bindgen crate
- **Context**: Steering requires native `cargo test` for the plugin's glue; wit-bindgen output is wasm-oriented and contains `unsafe` code.
- **Sources Consulted**: hello plugin source; `wit-bindgen` 0.63 generated-code conventions (extern blocks with `wasm_import_module` link attributes, `export!` macro producing `#[unsafe(export_name)]` functions); steering `tech.md` (`#![deny(unsafe_code)]`).
- **Findings**: Generated bindings compile only meaningfully for `wasm32`; linking a native test binary that references the host imports would fail. `#![forbid(unsafe_code)]` cannot be overridden by the generated module, but `deny` can be relaxed with `#[allow(unsafe_code)]` on one module.
- **Implications**: Put `generate!`, `export!` and the host adapter in one module compiled only under `#[cfg(target_arch = "wasm32")]`, with `#[allow(unsafe_code)]` on that module and `#![deny(unsafe_code)]` at the crate root. Every other module talks to the host through a small trait (`HostApi`) that has a wasm implementation and an in-memory fake, so dispatch, resolution and presentation are tested natively end to end. `wit-bindgen` is declared as a `wasm32`-only target dependency so native builds do not compile it.

### Read semantics and path handling
- **Sources Consulted**: `D:\Source\openidea\crates\ide-plugin\src\sandbox.rs` (`resolve_in_root`, `read_text`, `MAX_READ_BYTES`); `runtime.rs` `read_file` (buffer text first); kiro-core design (`Source`, `resolve::to_project_relative`, `feature_of_path`, `Workspace`).
- **Findings**: The host canonicalises the root and the requested path (absolute or root-relative), rejects escapes, missing files (`no such file`), non-files, files above 16 MiB and non-UTF-8, and prefers the unsaved buffer. kiro-core's `Source::read` receives project-relative `/` paths that core builds itself from feature names and fixed file names; `to_project_relative` turns the absolute active file into a relative path or `Outside`.
- **Implications**: `HostSource` passes relative paths straight through, maps every `Err` to `None` after a debug log, and refuses (without calling the host) empty, absolute, backslash or `..` paths as a second line of defence. Spec names from arguments are validated before any path is built from them.

### kiro-core contracts consumed
- **Sources Consulted**: `.kiro/specs/kiro-core/design.md` (Workspace, Resolver, Finding, TextRenderer, RenderGraph interfaces; module paths) and `requirements.md` (1.x, 7.x, 8.x, 10.x, 11.x).
- **Findings**: `kiro_core::source::{Source, Workspace, Feature}` with `Workspace::new(source, known)`, `adopt_roadmap_features()`, `load_roadmap()`, `add_known()`, `load()`, `known_features()`, `findings()`, `resolve_current(&ResolveInput)`; `kiro_core::resolve::{ResolveInput, Resolution, ResolvedVia, ProjectPath, to_project_relative, feature_of_path}`; `kiro_core::spec::{Phase, Doc, SpecMeta, phase_label}`; `kiro_core::finding::{Finding, Severity}`; `kiro_core::render::clip_lines(text, max_bytes, hint)` and `kiro_core::render::text::{findings_text, table}`. No public function returns `Result`; absent documents are `None` plus `SPEC_DOC_MISSING`.
- **Implications**: The plugin needs no parsing, resolution or rendering code of its own; it adapts I/O and presentation only.

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Host trait seam + feature-module table (chosen) | One `HostApi` trait with a wasm adapter and a fake; dispatcher routes by id group to modules that declare their commands | Natively testable end to end; sibling specs add a module and a manifest block; no host dependency in logic | One extra indirection; plugin-wide state lives in two statics | Mirrors kiro-core's ports-and-adapters |
| Direct host calls everywhere (hello style) | Modules call `host::*` directly | Smallest code | Untestable natively; every module repeats clipping and error mapping | Rejected |
| Per-spec variants encoded in command ids (chosen for spec selection) | `<group>.<name>.<spec>` registered after the roadmap is read | Works from Search Everywhere and the keymap; no host change | Up to 256 commands; variants cannot be removed until reload | The only channel the host offers |
| Rely on the active file only | No variants; commands fail when the file does not identify a spec | Simplest | Unusable from code files or with no editor | Rejected as sole mechanism |
| Project-level "current spec" file | Read `.kiro/current` as the explicit spec | Stable selection | The plugin cannot write it; the user would maintain it by hand | Rejected |

## Design Decisions

### Decision: Per-spec command variants replace the missing argument channel
- **Context**: The IDE UI passes no argument (see Research Log).
- **Alternatives Considered**: 1. Active-file resolution only. 2. A hand-maintained current-spec file. 3. Variants per spec encoded in the command id.
- **Selected Approach**: Base commands plus, for every spec-addressable command and every known spec, a variant `<group>.<name>.<spec>` (title `<base title> — <spec>`), registered during command calls once the roadmap is known, within a local budget of 256 ids. Insert variants `<group>.<name>.insert` are project-independent and declared statically.
- **Rationale**: Search Everywhere and the keymap can only select by command; the id is the only data the UI sends. Registration during a call is allowed and triggers the UI refresh.
- **Trade-offs**: Command list grows with the roadmap (18 base + 7 insert = 25 static + 17 spec-addressable × specs; 13 specs fit); stale variants remain until Reload Plugins; a spec named `insert` or containing `.` gets no variant.
- **Follow-up**: Keep the budget arithmetic in one place with a unit test; log skipped specs.

### Decision: Notification is the primary channel; return message secondary
- **Context**: The frontend discards the `Ok` message.
- **Selected Approach**: Every successful command shows exactly one notification (body clipped below 16 KiB at a line boundary with an omitted-lines trailer), optionally inserts the full text (below 1 MiB) when the insert flag is set, and returns the full text.
- **Trade-offs**: Long outputs are only fully visible through insertion; the summary is what most users see.

### Decision: `HostApi` trait with wasm adapter and in-memory fake
- **Context**: Native tests, `unsafe` in generated code, single WIT seam.
- **Selected Approach**: `host.rs` defines the trait and the fake; `bindings.rs` (wasm-only, `#[allow(unsafe_code)]`) holds `generate!`, the adapter and `export!`.
- **Rationale**: Keeps the no-panic, no-unsafe rules enforceable on every module that contains logic.

### Decision: Fresh workspace per command, two plugin-wide statics
- **Context**: Unsaved buffers change between calls; the host gives no invalidation signal.
- **Selected Approach**: Each `run-command` builds a new `Workspace<HostSource>`; only the granted permissions and the registered id set persist in statics (poison-tolerant mutexes).
- **Trade-offs**: The roadmap and the resolved spec are re-read on every command (a handful of reads; well inside the budget).

### Decision: Own commands `ws.specs` and `ws.current`
- **Context**: The dispatch path must be testable in the host before sibling specs land, and the brief's outcome (which spec is active, in which phase) needs a first command.
- **Selected Approach**: `ws.specs` (not spec-addressable, insert-capable, pass-through arguments) lists known specs, roadmap state and the variant sync report; `ws.current` (spec-addressable, not insert-capable) reports the resolved spec, phase, document availability and load findings. `viz.status` (spec-visualization) will extend this with approvals, task counts and the successor chain.

### Decision: `crate-type = ["cdylib", "rlib"]` and wasm-only `wit-bindgen`
- **Context**: Integration tests under `tests/` must link the crate; `wit-bindgen` is irrelevant natively.
- **Selected Approach**: Both crate types; `wit-bindgen` under `[target.'cfg(target_arch = "wasm32")'.dependencies]`. The component file name stays `openidea_plugin_kiro.wasm`, so `build.sh` works unchanged.

### Independent design review (fresh reviewer, one repair pass)
- **Verdict**: NEEDS_FIXES → repaired → finalized. Findings and fixes: (1) `FeatureModule` defined in `commands.rs` created a `ws` ↔ `commands` cycle — moved to `module.rs`, left of the feature modules; (2) a spec segment can be valid while the composed variant id exceeds 64 bytes — `sync_variants` now validates every composed id before any host call and routes such specs to `skipped_invalid`; (3) the `CommandsChanged` claim was imprecise on the error path — corrected, recorded as a revalidation trigger and as a manual host step; (4) precision: `resolve_current(&ResolveInput)`, `RoadmapState::NoSpecsSection` derived from `ROADMAP_NO_SPECS_SECTION`, roadmap order from `RoadmapDoc::names()` (not the sorted `known_features()`), explicit `level_of` mapping (kiro-core's `Severity` orders `Error < Warning < Info`), clippy restriction-lint allowances on `bindings.rs`.

## Risks & Mitigations
- Command budget exhaustion in projects with many specs — budget enforced locally in roadmap order; skipped specs reported in `ws.specs` and the log; specs stay addressable by argument.
- A trap caused by a host-side validation (invalid id, empty title, budget) — ids and titles validated locally before every registration; a unit test covers the grammar.
- Stale per-spec variants after switching projects in the same IDE session (installed plugin) — variants resolve to "spec not found in this project"; Reload Plugins clears them.
- `toml` dev-dependency version drift — any 0.8+ release suffices for the manifest consistency test; pinned at implementation time.
- `wit-bindgen` 0.63 macro API changes — the hello plugin in the host repository is the reference; the build task verifies against it.

## References
- `D:\Source\openidea\wit\plugin.wit` — the only contract with the IDE.
- `D:\Source\openidea\crates\ide-plugin\src\{runtime.rs,lib.rs,sandbox.rs,manifest.rs,types.rs}` — limits, activation context, dev-plugin loading, id rules.
- `D:\Source\openidea\src-tauri\src\commands\plugin.rs`, `D:\Source\openidea\src\features\plugins\{bridge.ts,store.ts,PluginsPage.tsx}`, `D:\Source\openidea\src\platform\actions.ts` — the production invocation path.
- `D:\Source\openidea\plugins\{README.md,build.sh,hello/*}` — package layout and build.
- `.kiro/specs/kiro-core/design.md` — consumed engine contracts.
