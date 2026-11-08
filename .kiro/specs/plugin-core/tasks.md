# Implementation Plan

- [x] 1. Foundation: package scaffold and host seam
- [x] 1.1 Create the plugin package scaffold and exclude it from the root workspace
  - Prerequisite: `crates/kiro-core` is implemented and builds (kiro-core spec), and `rustup target add wasm32-wasip2` has been run
  - Package manifest with its own standalone workspace, both library crate types (component plus native library for tests), the engine dependency by path, the bindings generator as a wasm32-only target dependency, the TOML parser as a dev-dependency, the host's release profile (size optimisation, link-time optimisation, symbol stripping, single codegen unit, abort on panic) and the same lint table as the engine with unsafe code denied
  - Crate root with crate-level docs placeholder, every `pub mod` declaration of the File Structure Plan (host, registry, args, host_source, present, context, module, ws, commands) backed by a documented empty stub (ownership of each stub passes to its component task), and the wasm32-only bindings module declared but empty until 3.5
  - PowerShell build script that ensures the sandbox target, builds the release component and copies it to the package's `dist/plugin.wasm`
  - Root workspace manifest gains the exclusion of the plugin package
  - Observable: from `plugins/kiro`, native build, clippy with warnings denied and fmt check pass on the empty crate; from the repository root, `cargo test` runs without touching the plugin package
  - _Boundary: PluginScaffold_
  - _Requirements: 1.2, 1.7, 10.4_

- [x] 1.2 Implement the host port and its in-memory fake
  - Trait over the eight host functions (the WIT's `register-language`/`register-theme` are deliberately excluded) with the plugin's log and notification level enums and the constant list of the three required permissions
  - In-memory fake host holding a root, an active file and project files, recording every log, registration, notification, insertion, replacement and read as events, with query helpers for registered commands, notifications, inserts and reads
  - Fake read emulation faithful to the host: empty path rejected, no root means "no project is open", backslashes normalised, absolute paths accepted only under the root (case-insensitive) and made relative, `..` segments rejected as outside the root, missing files reported as "no such file"
  - Observable: unit tests cover each read outcome and prove that every call is recorded as an event
  - _Boundary: HostApi_
  - _Requirements: 6.5, 8.2, 10.2_

- [ ] 2. Core: registry, arguments, source adapter, presentation
- [x] 2.1 Implement command declarations, the id grammar and the static command list
  - Command groups (`ws`, `nav`, `viz`, `vocab`, `check`) and the per-command declaration (name, title, spec-addressable, insert-capable, positional policy) with derived base, insert and per-spec variant ids and titles; title rule (`Kiro:` prefix, at most 128 bytes) and category `Kiro`
  - Id grammar: parse `<group>.<name>`, `<group>.<name>.insert` and `<group>.<name>.<spec>` into base, insert flag and spec; validators for host command ids (64-byte limit), spec id segments (no dots, not `insert`) and spec names (no path separators, no `..`)
  - Static command list = every base command followed by the insert variant of each insert-capable one, in declaration order
  - Observable: unit tests cover the id grammar table (valid, uppercase, leading dot, 65 characters, four segments, `insert` as spec), each validator, derived variant ids and titles, and the static list order
  - _Boundary: Registry_
  - _Requirements: 2.1, 2.2, 4.3, 4.5, 5.7, 9.2, 10.1_

- [x] 2.2 Implement the registry state, registration guard and per-spec variant synchronisation
  - Plugin-wide registry of registered ids behind a poison-tolerant mutex; registration validates id and title and the 256 budget before any host call and records the id
  - Variant synchronisation over the known specs in roadmap order: every composed variant id of a spec validated against the host id rule before any host call; all variants of a spec or none; invalid spec names or over-long composed ids skipped and reported; budget exhaustion skipping that spec and all later ones and reporting them; re-synchronisation reporting unchanged entries only
  - Observable: unit tests cover an invalid id refused without a host call, a budget that fits two of three specs, idempotent re-sync, an invalid name reported, and a name valid as a segment but yielding a 65-byte composed id reported with no registration event
  - _Boundary: Registry_
  - _Requirements: 2.5, 5.1, 5.3, 5.4, 5.5, 8.5, 10.1_

- [ ] 2.3 Implement argument parsing into a request
  - Blocked by 2.1: consumes the command declaration and parsed-id types, so it is not parallel with its preceding peers
  - Whitespace tokenisation; every `insert` token sets the flag; the explicit spec from the id modifier or, for commands whose first token names a spec, from the first remaining token; the remaining tokens passed through unchanged; the raw argument kept
  - Errors as single lines: spec named twice with different names, invalid spec name (no file is read for it), insertion requested on a command that is not insert-capable
  - Observable: unit tests cover no argument, `insert` in any position, spec from id and token agreeing and conflicting, pass-through commands keeping all tokens, and the insertion error
  - _Boundary: Args_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 7.4, 10.1_

- [ ] 2.4 (P) Implement the engine source adapter over the host read
  - Engine read port implemented over the host port: project-relative paths passed straight through; empty, absolute, backslash or `..` paths refused without a host call; any host error logged at debug level with path and reason and turned into "absent"
  - Observable: unit tests prove guarded paths cause no read event, a host error yields `None` plus one debug log event, and a successful read returns the text
  - _Boundary: HostSource_
  - _Depends: 1.2_
  - _Requirements: 6.1, 6.2, 6.6_

- [ ] 2.5 (P) Implement presentation: output shape, clipping budgets and delivery
  - Output value with title, level and complete text; constructor from an intro plus findings rendered with the engine's findings text; level mapped explicitly from the finding severities (any error, else any warning, else info)
  - Clipping with the engine's line clipper below the host's limits: notification body under 16 KiB (with trailer `… N more lines — run <id>.insert for the full text` when clipping occurred and an insert variant id is supplied, or `… N more lines` otherwise; amendment 2026-11-04), insertion under 1 MiB, title at 256 bytes on a character boundary; error messages flattened to one line and capped at 1 KiB
  - Delivery: takes the optional insert-variant id, emits exactly one notification (clipped title and body), one insertion of the clipped full text when requested, and returns the complete text as the command message
  - Observable: unit tests show a 20 KiB text clipped at a line boundary with the variant-naming trailer (when an insert id is given) or the plain trailer (when absent) and a body at or under 16 KiB, the insertion budget respected, a multibyte title cut on a character boundary, newlines flattened in error lines, finding output byte-identical to the engine's findings text, the level mapping, and the delivered events (one notify, at most one insert)
  - _Boundary: Present_
  - _Depends: 1.2_
  - _Requirements: 7.1, 7.2, 7.3, 7.5, 7.6, 7.7, 10.1_

- [ ] 3. Integration: project context, dispatch, own commands, manifest, bindings
- [ ] 3.1 Implement the command context: root, active file, known specs, workspace and resolution
  - Fails with "no project is open" when the host reports no root; normalises the reported active file against the root with the engine's path normalisation, treating absent or outside files as no active file
  - Builds a fresh engine workspace over the source adapter, reads the roadmap once and takes its names in document order (not the workspace's sorted list), records whether the roadmap is absent, has no specs section (the engine's finding code) or is present with a count, adds the active-file spec and the explicit spec, and keeps each known spec with the sources it came from, deduplicated, in roadmap order
  - Resolution through the engine (explicit, active file, only known) with the two unresolved messages (candidates plus the per-spec command hint; no spec known plus the roadmap hint); cached loading of a spec and a document-readability helper
  - Observable: unit tests cover the known-spec union with sources, each roadmap state, an outside and an absent active file, both unresolved messages, the no-root error, and that repeated loads of a spec cause no repeated read events
  - _Boundary: Context_
  - _Depends: 2.2, 2.4_
  - _Requirements: 3.1, 3.2, 4.6, 4.7, 4.8, 4.9, 6.3, 6.4, 8.3, 9.3, 10.1_

- [ ] 3.2 Implement dispatch: activation, deactivation, command run and the feature-module contract
  - Feature-module trait (group, command declarations, run with context and request returning an output) in its own module placed left of the feature modules in the dependency direction, and the module table in the dispatch module, empty until 3.3 adds the first module
  - Activation: verify the three permissions against the granted list and fail with a message naming the missing ones; register the static command list through the registry; keep the granted permissions in the plugin-wide static; log the command count. Deactivation logs only
  - Command run in order: parse the id (unknown id, a per-spec id on a non-spec-addressable command, or a known group with no module, are errors), parse arguments, build the context, reject an explicitly named spec whose metadata is unreadable (synchronising variants for roadmap/active-file specs first without registering variants for the unreadable spec), synchronise per-spec variants (logging skipped specs) before anything else runs, run the module, deliver the output (passing `cmd.insert_id().as_deref()` for the notification trailer), return the complete text; every error passes through the single-line error formatter and shows no notification
  - Observable: tests with the empty module table prove activation registers exactly the static list and fails with all three permission names when none is granted, an unknown id and a known group with no module (using the `run_command_with` seam with a partial table) produce their messages, and no event except a log occurs on deactivation
  - _Boundary: Commands_
  - _Depends: 2.3, 2.5, 3.1_
  - _Requirements: 1.6, 2.3, 2.4, 2.6, 5.2, 5.6, 7.7, 8.1, 8.4, 8.5, 9.1, 9.3, 9.4_

- [ ] 3.3 Implement the `ws.specs` and `ws.current` commands and add the module to the table
  - Command declarations: list specs (not spec-addressable, insert-capable, pass-through tokens) and current spec (spec-addressable, not insert-capable, first token names the spec)
  - `ws.specs`: known-spec counts by source, roadmap state line, a table of spec, sources, spec.json readability and phase (via `kiro_core::spec::phase_label`) in known order, the variant synchronisation line with skipped specs; a warning-level message naming the roadmap path and the active-file rule when nothing is known
  - `ws.current`: resolved spec with how it was resolved, phase label (`kiro_core::spec::phase_label`), presence of the four documents, finding counts per severity, the findings text; level from the findings
  - Integration touch: one entry in the dispatch module table
  - Observable: running both commands through dispatch over the fake host with a two-spec tree yields the documented text shapes and titles, and the per-spec variants of the current-spec command appear in the fake's registrations
  - _Boundary: WsCommands, Commands_
  - _Requirements: 2.7, 3.2, 3.3, 3.4, 5.4, 5.5, 7.8_

- [ ] 3.4 Create the plugin manifest and the manifest consistency test
  - Manifest with id `kiro`, name, version, description, API version `0.1`, the component path, exactly the three permissions, and one static command block per entry of the static command list (base commands and insert variants) with category `Kiro`
  - Test that parses the manifest and asserts its static command ids, titles and categories equal the registry's static list for all declared modules
  - Observable: the manifest test passes; the manifest contains only keys the host accepts
  - _Boundary: Manifest, PluginTests_
  - _Requirements: 1.1, 2.3, 5.7, 10.3_

- [ ] 3.5 (P) Implement the wasm bindings and build the component
  - Prerequisite: the sibling checkout `../openidea` is present with `wit/plugin.wit` (the bindings are generated from it at `../../../openidea/wit` relative to the package) and `plugins/build.sh`
  - Wasm32-only module with unsafe code and the restriction lints allowed locally: bindings generated from the host's WIT world, a host adapter mapping the port one-to-one onto the host functions and enums, and the exported guest (`export!(Kiro with_types_in self)`) delegating activation, deactivation and command runs to dispatch
  - Build the release component with the PowerShell script and with the host repository's build script given the package's absolute path
  - Observable: `dist/plugin.wasm` exists after either build; clippy for the wasm target passes with warnings denied; no module other than the bindings references the generated code
  - _Boundary: Bindings_
  - _Depends: 3.3_
  - _Requirements: 1.2, 8.2, 10.4_

- [ ] 4. Validation
- [ ] 4.1 Write the end-to-end dispatch tests over the fake host
  - Fake host loaded with the engine's minimal fixture tree (embedded text, root `C:\proj`) and driven through activation and command runs
  - Scenarios: activation registers exactly the static command list and nothing project-dependent; current spec from an active design file notifies the spec and phase and returns the full text; afterwards both per-spec variants exist and the superseded spec's variant reports it; no active file with two known specs fails listing both candidates; a variant for an unknown spec fails with "not found" and does not register per-spec variants for that unknown spec; listing with `insert` produces one insertion and one notification; insertion on the current-spec command fails; spec named twice; invalid spec name causes no read; a command of a group without a module (through the `run_command_with` seam with a partial table) fails with "not available in this build"; a 20 KiB finding list is clipped in the notification (ending with the variant-naming trailer for an insert-capable command and the plain trailer for a command without an insert variant) but complete in the return value; each path is read at most once per command; "no project is open" without a root
  - Observable: all scenarios pass natively with `cargo test` from the package directory
  - _Boundary: PluginTests_
  - _Depends: 3.3_
  - _Requirements: 1.6, 2.3, 2.4, 2.6, 4.3, 4.4, 4.5, 4.8, 4.9, 5.1, 5.2, 5.6, 6.3, 7.1, 7.2, 7.3, 7.4, 7.8, 8.1, 8.4, 9.4, 10.2_

- [ ] 4.2 (P) Write the package README
  - Sections: the three build routes (host build script with the absolute package path, PowerShell script, plain cargo); Windows dev-plugin installation (create the open project's `.openidea\plugins` directory, create the junction to the package, trust the project, open Settings › Plugins or run Reload Plugins); which project to open (the one whose `.kiro` tree should be read: this repository for its own specs, the host repository when testing the host); installation from disk and re-installing after rebuilds; the reload step; the command table with spec-addressable and insert-capable flags; the three-tier output contract (notification = summary with `<id>.insert` trailer when clipped, `.insert` = full payload in editor buffer, `kiro-cli` JSON = durable record); the argument grammar for programmatic callers; the host limits (16 KiB notification, 1 MiB insertion, 256 commands, 5 s)
  - Observable: the README exists in the package with every section above and the commands it lists equal the manifest's static list
  - _Boundary: PluginScaffold_
  - _Depends: 3.4_
  - _Requirements: 1.4, 1.5, 7.2_

- [ ] 4.3 Verify the component in the host as a dev plugin
  - Prerequisite: the OpenIDEA host application is built and launchable from the `../openidea` checkout (when no GUI host is running, build `dist/plugin.wasm`, create/verify the dev-plugin junction, run host-side integration tests if `../openidea` is present, and record the manual GUI check as deferred in the completion note — matching `spec-navigation` 4.3 and `spec-visualization` 4)
  - Build the component from the final source (3.3 merged) with the PowerShell build script before creating the junction, so `dist/plugin.wasm` is not stale from an earlier `(P)` task
  - Junction the package into this repository's `.openidea\plugins\kiro`, open the repository in OpenIDEA, trust it, reload plugins; open a spec document and run the current-spec command; run the list command and confirm the per-spec commands appear in Search Everywhere
  - Reload again, close all spec documents and run the current-spec command so it fails with the unresolved message, then confirm the per-spec commands still appear (error-path visibility through the frontend's refresh-on-failure)
  - Rebuild the component, run Reload Plugins and confirm the rebuilt component runs without restarting the IDE
  - Observable: `dist/plugin.wasm` is freshly built and the junction is in place; when the GUI host is available, the plugin is listed as Active with a dev tag, the current-spec notification names the open spec, and the per-spec commands are searchable after both the successful and the failing run (when no GUI host is available, the manual GUI check is recorded as deferred in the completion note)
  - _Boundary: PluginScaffold_
  - _Depends: 3.3, 3.4, 3.5, 4.2_
  - _Requirements: 1.2, 1.3, 1.4, 10.5_

- [ ] 4.4 Run the final quality gate on both targets
  - From the package directory (`--manifest-path plugins/kiro/Cargo.toml`): native tests (`cargo test`), clippy for the native and wasm targets with warnings denied (`cargo clippy --all-targets -- -D warnings` and `cargo clippy --target wasm32-wasip2 -- -D warnings`), fmt check, `cargo deny --manifest-path plugins/kiro/Cargo.toml check`, and release component build (`cargo build --release --target wasm32-wasip2`); from the repository root: tests and clippy unaffected by the plugin package
  - Crate-level docs completed with the command grammar and output channels
  - Observable: every command above succeeds and the root workspace build does not require the sandbox target
  - _Boundary: PluginScaffold_
  - _Depends: 4.1, 4.3_
  - _Requirements: 1.7, 10.4_
