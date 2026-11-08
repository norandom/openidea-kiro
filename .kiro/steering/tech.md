# Technology Stack

## Architecture

Three Rust packages, one engine:

- **`crates/kiro-core`** — the engine. Pure, `std`-only, **no filesystem, no network, no threads, no clock**. Takes strings through a `Source` trait (`fn read(&self, project_relative_path: &str) -> Option<String>`), returns plain data and `Finding`s. Compiles for native and `wasm32-wasip2`. Everything that parses, resolves, checks or renders lives here.
- **`plugins/kiro/`** — OpenIDEA plugin: a **WebAssembly component** (component model, WASI preview 2, wasmtime host) in the style of Zed extensions. The only contract with the IDE is `../openidea/wit/plugin.wit` (`openidea:plugin@0.1.0`). Sandboxed; implements `Source` over `host::read-file`; maps commands to core calls and core output to notifications / editor insertion / command messages.
- **`crates/kiro-cli`** — native CLI gate. Implements `Source` over `std::fs`, walks `.kiro/specs`, runs the same checks, prints text/JSON, exits non-zero on failures. Also hosts tooling the sandbox cannot (Auto Ontology JSON / SKOS Turtle → `vocabulary.toml` import).

Host API (v0.1) and its constraints:

- Always available: `log`, `register-command` (becomes action `plugin.<plugin-id>.<id>`, max 256), `show-notification` (body clipped at 16 KiB), `register-language`, `register-theme`.
- `project-read`: `workspace-root`, `read-file` (absolute or project-relative path, confined to project root, UTF-8, ≤ 16 MiB; unsaved buffer wins over disk).
- `editor-read`: `active-file` (returns an **absolute** path).
- `editor-write`: `insert-text`, `replace-selection` (≤ 1 MiB per call).
- **Not available**: directory listing, file writing, opening files in editors, network. Navigation/visualization are built on read + editor-insert + notifications + command messages.
- Per-call limits: ~5·10⁹ fuel, 5 s wall clock, 256 MiB memory. A trap/permission violation marks the plugin Failed for the session.
- `run-command(id, arg)` returns `result<option<string>, string>` — **verified against `../openidea/src-tauri/src/commands/plugin.rs` and `src/features/plugins/bridge.ts`**: the UI passes `arg = null` from Search Everywhere, the keymap and the Plugins page (only another feature calling `executeAction("plugin.kiro.<id>", "<string>")` can supply one), and it **discards the `Ok` message** (only `Err` is shown as "<title> failed: msg"). Consequences: the notification is the primary output channel; feature selection uses active-file resolution first, then per-spec variant commands `<group>.<name>.<spec>` registered during command calls (activation runs with no project context) under a locally enforced 256-id budget. `register-command` traps on invalid ids or over budget and gated host calls trap without permission — validate ids and check permissions before any host call. See plugin-core design.

## Core Technologies

- **Language**: Rust, `edition = 2024`; core and CLI in a root Cargo workspace; the plugin crate is `crate-type = ["cdylib", "rlib"]` (rlib so native integration tests can link it), target `wasm32-wasip2`, with its own `[workspace]` (excluded from the root workspace) and `kiro-core = { path = "../../crates/kiro-core" }`.
- **Bindings**: `wit-bindgen` (`wit_bindgen::generate!({ world: "plugin", path: "../../../openidea/wit" })` from `plugins/kiro/`).
- **Build**: `rustup target add wasm32-wasip2`, then `../openidea/plugins/build.sh <abs path to plugins/kiro>` → `dist/plugin.wasm` (`opt-level = "s"`, `lto`, `strip`, `panic = "abort"`).
- **Host-side tests**: the IDE's sandbox integration tests live in `../openidea/crates/ide-plugin/tests/host.rs`. Our plugin is exercised as a dev plugin (`../openidea/.openidea/plugins/kiro`, directory must be created) with _Reload Plugins_ after each rebuild; host tests are only touched if the WIT itself changes.

## Key Libraries

- `wit-bindgen` — component bindings (plugin only).
- `serde` + `serde_json` — `spec.json`; `toml` — `vocabulary.toml` (both wasm-safe).
- `biodivine-lib-bdd = "0.6"` — contradiction search in core, confined to `checks/bdd.rs`. **Decided with evidence (spec-constraints research)**: 0.5.27 and 0.6.3 compile for `wasm32-wasip2`; a release `cdylib` with the plugin profile links into a valid component whose `wasi:*` imports equal a dependency-free baseline (the `rand`/`getrandom` path is never linked). Uses `binary_op_with_limit` for per-operation node caps and deterministic `sat_witness`; variable cap clamped at 65 000 (library `u16` limit).
- `clap`, `oxttl`/`oxrdf`, `toml_edit` (optional, already in lockfile via `toml`) — CLI only (argument parsing; secondary SKOS Turtle import; format-preserving TOML serialization/append).
- **Prior art, not a dependency: `D:\Source\spec-kit-ears-tdd` (`ears-sdd` crate, retired — this project is its successor).** Consult it for edge cases and test ideas; lifting its regression cases into our fixtures is encouraged since it no longer evolves — `ears.rs` (EARS false positives: quoted modals, hyphenated compounds, possessive apostrophes), `ccsdd.rs` (the four cc-sdd requirement grammars seen in the wild), `kiro.rs` (superseded → successor rule), `bdd.rs` (one-hot encoding, contiguous blocks, no dynamic reordering), `vocabulary.rs` (TOML shape, `normalized_label`). Do not port its Spec Kit parsers, its `traceability.rs` (a TOML req → tests map) or anything using `std::fs`. Its `domain` field is a value kind (`entity|bool|enum|int`) — v2 calls that `kind` and reserves `topic` for the subject domain.
- **NVIDIA Auto Ontology** (`github.com/NVIDIA/auto-ontology`): vocabulary *source of truth*, consumed as an exported artifact converted to `.kiro/vocabulary.toml` by `kiro-cli`; never a runtime service. **Verified (vocabulary-grounding research): it exports no SKOS/RDF** — its term surface is JSON (`GET /api/terms`: `name`, `description`, `synonyms`, `related_terms`, `zones`; no hierarchy, no topic) plus a YAML model export. The importer therefore reads that JSON directly (`--format auto-ontology-json`, topic and kind supplied on the command line); SKOS Turtle remains a generic secondary input.

## Development Standards

### Type Safety
- `#![deny(unsafe_code)]`; no `unwrap`/`expect`/indexing panics in core or command paths (a sandbox panic poisons the plugin); errors as `Result<_, String>` at the plugin boundary so the IDE shows "<title> failed: msg" and the plugin keeps running.

### Code Quality
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

### Testing
- `kiro-core`: unit tests against fixtures under `crates/kiro-core/fixtures/` copied from this repo's cc-sdd templates and real specs; golden outputs for renderers; property-ish tests that parsers never panic on arbitrary text.
- Plugin: `cargo test` natively for its thin glue; component-level via the host (`cargo test -p ide-plugin` in `../openidea` after rebuilding `dist/plugin.wasm`).
- CLI: integration tests over fixture trees in a temp dir; exit-code contract tested.
- **Parallel test execution (required, adopted 2026-11-04 round-2 review)**: CI runs the native test suites with `cargo nextest run` (process-per-test parallelism — the Rust equivalent of pytest-xdist) plus `cargo test --doc` separately, because nextest does not run doctests. Locally `cargo test` remains valid. Golden renderer/parser fixtures use `insta` (dev-dependency only — dev-dependencies never link into the wasm `cdylib`).

## Development Environment

### Required Tools
- `rustup` with `wasm32-wasip2` target; `cargo`; PowerShell on Windows (paths are Windows-style; symlinks need Developer Mode — use `mklink /J` junctions for the dev plugin).

### Common Commands
```powershell
# Core + CLI (root workspace)
cargo test
cargo run -p kiro-cli -- check --project . --all
# Plugin component (from ../openidea; build.sh is bash and only recognises POSIX-style absolute paths)
bash plugins/build.sh /d/Source/openidea-kiro/plugins/kiro
# or directly, in plugins\kiro of this repo
cargo build --release --target wasm32-wasip2
# Dev plugin: junction ../openidea/.openidea/plugins/kiro -> this package, then Reload Plugins in the IDE
mklink /J D:\Source\openidea\.openidea\plugins\kiro D:\Source\openidea-kiro\plugins\kiro
# Host-side sandbox tests (in ../openidea)
cargo test -p ide-plugin
```

## Key Technical Decisions

- **Engine first, frontends thin**: all parsing/checking/rendering in `kiro-core`; the plugin and the CLI only adapt I/O. This is what makes the plugin sandbox-safe and the CLI gate identical to the IDE result.
- **Fresh cc-sdd parsers**: written against this repo's `.opencode/skills/kiro-*` templates, with fixtures as the contract. `ears-sdd` is prior art.
- **Sandbox-first discovery**: in the plugin, feature names come from `.kiro/steering/roadmap.md`'s `## Specs (dependency order)` list, the active file's path under `.kiro/specs/<feature>/`, and command args — never `read_dir`. The CLI walks the tree.
- **`_Boundary:_` names components**: file ownership is `file → File Structure Plan component → task _Boundary:_`; core owns that join.
- **Topic-scoped vocabulary**: `.kiro/vocabulary.toml` terms carry `topic`; `spec.json` declares `topics`; resolution, declared-before-use and contradiction search never cross topics. An undeclared topic disables term checks for that spec with an info finding — it never produces a cross-topic match.
- **Vocabulary via Auto Ontology as data**: `/api/terms` JSON export → `kiro-cli vocab import --format auto-ontology-json --topic <t>` → TOML committed in the project (SKOS Turtle as an alternative input). Hierarchy (`broader`) and topics are curated by maintainers in the TOML; Auto Ontology does not supply them.
- **Adopted tooling (2026-11-04, round-2 review; amended 2026-11-08)**: `winnow` in `kiro-core` for the guard/effect/annotation grammar (already transitive via `toml` → `toml_edit` → `winnow`; wasm-safe, panic-free positioned errors; cap parser nesting depth for the wasm stack); `insta` as a dev-dependency for golden fixtures; `cargo nextest run` for parallel CI tests (see Testing); `cargo-deny` in CI with `wasm32-wasip2` in `[graph] targets` so `getrandom`/`rand`/fs stay out of the wasm tree automatically; single-subcommand Cargo aliases in `.cargo/config.toml` (`cargo gate` for `nextest run`, plus `cargo kiro-check` once `cli-gate` lands for `run -p kiro-cli -- check --project . --all --fail-on error`, since Cargo `[alias]` entries cannot chain multiple commands) with the full multi-step sequence in `.github/workflows/ci.yml` (`dtolnay/rust-toolchain`, `Swatinem/rust-cache`, wasm target, standalone `plugins/kiro` checks, CLI gate). Kept hand-rolled on purpose — do not relitigate without profiling evidence: the cc-sdd line scanners, the EARS masker (no regex; `memchr` only if the 16 MiB release perf test gets tight), `biodivine-lib-bdd` 0.6, Mermaid emission + lint, `toml` in core, `oxttl`/`oxrdf` for SKOS, `clap` + manual exit codes, fixed-seed test loops and manual release-only timing.

---
_Document standards and patterns, not every dependency_
