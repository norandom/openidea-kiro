# Project Structure

## Organization Philosophy

Two cooperating roots:

1. **This repo (`openidea-kiro`)** is the kiro/cc-sdd workspace (`.kiro/steering/`, `.kiro/specs/<feature>/`), the home of the engine and CLI (`crates/`), and the home of the plugin package (`plugins/kiro/`). Specs are the source of truth for what the code must do.
2. **The sibling repo (`../openidea`)** is the host IDE: the plugin package is installed there as a dev plugin (junction `../openidea/.openidea/plugins/kiro` → `plugins/kiro/` of this repo) and its sandbox is tested by `crates/ide-plugin/tests/`.

Spec-driven: code changes in `crates/` and `plugins/kiro/` are made against approved tasks in a spec here.

## Directory Patterns

### Kiro workspace (this repo)
**Location**: `.kiro/`
**Purpose**: steering (`steering/*.md`, including the discovery `roadmap.md`), specs (`specs/<feature>/{brief.md, spec.json, requirements.md, design.md, tasks.md}`), project vocabulary (`vocabulary.toml`).
**Example**: `.kiro/specs/spec-navigation/tasks.md`, `.kiro/vocabulary.toml`

### User-facing documentation
**Location**: `docs/`
**Purpose**: Guides, format specifications, ontology mappings, and external integration procedures (e.g. `docs/vocabulary.md`).
**Example**: `docs/vocabulary.md`

### Engine crate
**Location**: `crates/kiro-core/`
**Purpose**: pure library — `src/` modules per seam, `fixtures/` with cc-sdd sample trees used by tests. No `std::fs`, no network, no threads.
**Example**: `crates/kiro-core/src/tasks.rs` parses `tasks.md` text into a task tree; `crates/kiro-core/fixtures/minimal/.kiro/specs/demo/tasks.md`

### CLI crate
**Location**: `crates/kiro-cli/`
**Purpose**: binary `kiro-cli` — `Source` over `std::fs`, tree walking, report printing, exit codes, vocabulary import (Auto Ontology JSON, SKOS Turtle). Thin: no parsing or checking logic of its own.
**Example**: `crates/kiro-cli/src/main.rs`

### Plugin package (this repo, installed as dev plugin)
**Location**: `plugins/kiro/`
**Purpose**: one self-contained plugin package — `plugin.toml` manifest (static `[[contributes.commands]]` for every command), `Cargo.toml` (own `[workspace]`, `kiro-core` by path), `src/`, built `dist/plugin.wasm`. Thin: host wrapper, `Source` over `read-file`, dispatch, presentation.
**Example**: `plugins/kiro/plugin.toml` declares `id = "kiro"`, `openidea-api = "0.1"`, `permissions = ["project-read", "editor-read", "editor-write"]`.

## Naming Conventions

- **Plugin/command ids**: kebab-case, matching the host regex `[a-z0-9][a-z0-9._-]*`; user-facing actions are `plugin.kiro.<command-id>` grouped by prefix: `nav.*`, `viz.*`, `vocab.*`, `check.*`.
- **Spec/feature names**: kebab-case directory names under `.kiro/specs/`.
- **Finding codes**: `SCREAMING_SNAKE` (e.g. `EARS_NO_SHALL`, `TERM_UNDECLARED`, `TERM_TOPIC_MISMATCH`, `CONTRADICTION`), stable across frontends.
- **Rust**: standard `rustfmt` style; modules snake_case; crate names `kiro-core`, `kiro-cli`, `openidea-plugin-kiro`.

## Import Organization

```rust
// kiro-core: one module per seam, re-exported from lib.rs
pub mod source;      // Source trait + Workspace (known features, reads)
pub mod md;          // typed line scanner shared by all parsers
pub mod spec;        // spec.json, phases, topics
pub mod requirements;// ### Requirement N / #### Acceptance Criteria -> N.M
pub mod tasks;       // - [ ] N.M (P), _Requirements_, _Depends_, _Boundary_
pub mod design;      // headings, File Structure Plan, Mermaid blocks
pub mod roadmap;     // ## Specs (dependency order) list
pub mod resolve;     // current-spec resolution
pub mod join;        // file<->component<->task join (File Structure Plan + _Boundary:_)
pub mod vocab;       // vocabulary.toml, topics, term resolution
pub mod checks;      // ears, declared-before-use, contradiction
pub mod render;      // mermaid + text renderers, clip_lines
pub mod viz;         // graph shapes consumed by spec-visualization
pub mod finding;     // Finding { feature, severity, code, file, line, message }

// plugin: WIT bindings first, then host, then local glue
wit_bindgen::generate!({ world: "plugin", path: "../../../openidea/wit" });
use openidea::plugin::host;
mod host_source;   // Source over host::read_file
mod commands;      // dispatch by id prefix -> kiro_core calls
mod present;       // findings/graphs -> notification | insert | message
```

**Path note**: from `plugins/kiro/` of this repo, the host's WIT is at `../../../openidea/wit` (`D:\Source\openidea\wit`); `path:` in `generate!` is relative to the crate root.

## Code Organization Principles

- One module per responsibility seam in core; frontends only adapt I/O and presentation.
- Parsers take `&str` and return plain data — no I/O — so they are unit-testable natively and wasm-safe; they never panic on malformed input (return partial data + findings).
- All content enters core through `Source`; the plugin's `Source` cannot list, so core APIs take an explicit set of known feature names.
- Keep the plugin within sandbox limits: no threads, no fs, no network; output via notifications / editor insert (the IDE discards the command's `Ok` message — never make it the only channel), clipped and summarised before the host clips it.

---
_Document patterns, not file trees. New files following patterns shouldn't require updates_
