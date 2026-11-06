# Brief: cli-gate

## Problem

Checks that only run inside the IDE are advice; checks that run in CI are a gate. v1's value came from `ears-sdd validate` blocking merges. v2's engine lives in `kiro-core` with no I/O; without a native frontend there is no way to run it over a whole `.kiro` tree, in CI, or to perform tasks the sandbox forbids (walking directories, importing an Auto Ontology term export into `vocabulary.toml`).

## Current State

- `kiro-core` exposes `Source`, `Workspace`, parsers, `run_checks`, and renderers; `spec-constraints` defines finding codes; `vocabulary-grounding` defines the TOML format and a normative SKOS → TOML mapping, and documents (verified against the Auto Ontology README and OpenAPI spec) that **Auto Ontology exports no SKOS**: its term surface is JSON — `GET /api/terms` returning `name`, `description`, `synonyms`, `related_terms`, `zones` (no hierarchy, no topic) — plus a YAML model export. vocabulary-grounding's docs currently say the JSON → SKOS step is "manual or maintainer-scripted" with no tool in this project; this spec closes that gap by importing the JSON directly.
- The plugin is the only frontend; it cannot list specs or read RDF.
- `ears-sdd`'s CLI is prior art for UX (`validate --project . --all`, `doctor`, exit codes, `report.rs` text output).

## Desired Outcome

A `crates/kiro-cli` binary `kiro-cli` with:
- `check --project <dir> [--feature <name>]... | --all [--format text|json] [--fail-on error|warning]` — walks `.kiro/specs`, builds a `Workspace` over `std::fs`, runs the same checks as the plugin, prints findings, exits `0` (clean), `1` (findings at or above `--fail-on`), `2` (usage/IO error).
- `status`, `graph`, `tasks` — the visualization renders, natively (optional, cheap because core owns them).
- `vocab import <file> --format auto-ontology-json|skos-turtle --topic <t> [--kind entity] [--out .kiro/vocabulary.toml] [--merge]` — two input formats, one output shape (vocabulary-grounding's TOML):
  - `auto-ontology-json` (primary, the format that exists): the `/api/terms` JSON array. Mapping: `name` → id (normalised per vocabulary-grounding's id rule) and `label`; `description` → `definition` (empty → finding, term still emitted); `synonyms` → `alt_labels`; `zones` → recorded as a comment/`source` note, not a topic; `related_terms` → not mapped to `broader` (it is not a hierarchy) — emitted as a `# related:` comment so a maintainer can promote relations by hand; `topic` from `--topic` (required), `kind` from `--kind` (default `entity`). Duplicate names after normalisation → finding, first wins.
  - `skos-turtle` (secondary, generic): per vocabulary-grounding's normative SKOS → TOML mapping, via `oxttl`/`oxrdf`.
  - `--merge` keeps existing terms and definitions, adds new ones, and reports conflicts instead of overwriting; without it the importer refuses to overwrite an existing `--out`.
- `vocab validate` — runs vocabulary-grounding's loader findings over a file.
- A documented GitHub Actions / generic CI snippet.

## Approach

- `FsSource` implementing `Source` with root confinement (reject paths escaping the project, like the plugin host does); feature discovery by `read_dir` over `.kiro/specs/*/` (directories containing `spec.json` or `requirements.md`).
- `clap` for arguments; text reporter mirrors the plugin's notification format (shared through core's text renderers) so IDE and CI say the same thing; JSON reporter emits the `Finding` struct verbatim.
- No logic duplication: anything the CLI needs that is not I/O goes into core.

## Scope

- **In**: crate, `FsSource`, discovery, `check`/`status`/`graph`/`tasks`/`vocab import`/`vocab validate` subcommands, reporters, exit-code contract, integration tests over fixture trees in temp dirs, CI docs.
- **Out**: any parsing/checking/rendering logic (core); plugin packaging; Spec Kit layouts; running Auto Ontology.

## Boundary Candidates

- `FsSource` + discovery — the only `std::fs` code in the project.
- Reporters (text/json) — thin over `Finding`.
- `vocab import` — the only RDF/Auto-Ontology-JSON adapter code in the project; both formats feed one TOML writer.

## Out of Boundary

- IDE presentation (plugin-core `present`).
- Editing specs or vocabulary beyond writing the import output file explicitly requested with `--out`.

## Upstream / Downstream

- **Upstream**: kiro-core, spec-constraints (checks), vocabulary-grounding (mapping), spec-visualization (render shapes).
- **Downstream**: CI pipelines; developers seeding `.kiro/vocabulary.toml`.

## Existing Spec Touchpoints

- **Extends**: none. **Adjacent**: plugin-core (the other frontend; output formats aligned through core), vocabulary-grounding (importer contract).

## Constraints

- Native only (not built for wasm); `edition = 2024`; clippy/fmt clean.
- Deterministic output ordering; no timestamps in any report, including the JSON report (per requirement 5.3).
- Exit codes are part of the contract and tested.
- Root confinement for all reads; the importer writes only the `--out` path.
