# Research & Design Decisions: cli-gate

## Summary
- **Feature**: `cli-gate`
- **Discovery Scope**: Extension (light discovery) — a second adapter over the engine whose contracts six upstream designs already fix; plus an external verification of the Auto Ontology JSON term surface and of the RDF crates the secondary importer uses.
- **Key Findings**:
  - `GET /api/terms` does not return a bare array: it returns a `TermsPageResponse` object whose `terms` array holds `TermListItem` objects (`id` required; `name`, `description`, `synonyms`, `zones`, `tags`, `certification`, `owner_id`, `schema_names`, certification flags nullable). `related_terms` exists only on `TermDetail` (`GET /api/terms/{term_id}`) as `TermSummary` objects (`id`, `name`, `description`). `zones` are `ZoneChip` objects (`id`, `name`, `color`, `enabled`). No hierarchy, no topic. The importer therefore accepts both the page object and a bare array, reads objects leniently, and never maps `related_terms` to `broader`.
  - Every behaviour the CLI prints is already specified upstream: `run_checks`/`CheckReport` (spec-constraints), `findings_text` and `Finding` JSON (kiro-core), `viz` views (spec-visualization), `vocab::load`/`parse` (vocabulary-grounding), the `check.spec` notification lines (spec-constraints' plugin module). The CLI adds I/O, argument handling, a report envelope and the importer; the only engine change is four importer finding codes in the catalogue, the growth path both upstream specs used.
  - The retired `ears-sdd` CLI confirms the UX that worked in v1: `validate --project . --all`, exit `2` for anything that is not a verdict, a versioned machine-readable report, and a deliberate "print, do not write" default for vocabulary tooling (kept here as `--out -`).

## Research Log

### Auto Ontology term export surface
- **Context**: The brief fixes the primary importer input as "the `/api/terms` JSON array"; vocabulary-grounding's research recorded the fields but not the page shape.
- **Sources Consulted**: `https://github.com/NVIDIA/auto-ontology` README (`curl -H "x-api-key: …" …/api/terms`, pointer to `docs/openapi/auto-ontology-api.json`); the OpenAPI document (`raw.githubusercontent.com/NVIDIA/auto-ontology/main/docs/openapi/auto-ontology-api.json`, read in four slices): `GET /api/terms` (query `query`, `skip` ≥ 0, `limit` 1–100 nullable — omitted returns all rows; no zone filter) → `TermsPageResponse { terms: [TermListItem], total, column_attribute_counts, sql_attribute_counts, related_counts }`; schemas `Term`, `TermListItem` (`additionalProperties: true`), `TermDetail` (adds `related_terms: [TermSummary]`, `tags: [TagChip]`, `tables`, audit fields), `TermSummary`, `ZoneChip`, `TagChip`; `POST /api/model/export` (`ModelFormat` ∈ `auto_ontology` YAML | `ossie` zip, admin-only).
- **Findings**: No SKOS/RDF export exists; the YAML model export is a different artefact (semantic model, not a term list). Term names are nullable in `TermListItem`. Zones are organisational groupings, not subject domains. Related terms are untyped and only reachable per term.
- **Implications**: The reader is lenient by design: top level is the page object (`terms`) or an array; each item is an object; `name` null/blank → skip with a finding; `zones`/`related_terms` accept objects (`name`, else `id`) or strings; everything else ignored. Multiple input files are accepted so that paged exports (`skip`/`limit`) and per-term detail responses can be concatenated by the maintainer. The brief's phrase "JSON array" is honoured as one of the two accepted shapes.

### Prior art: `ears-sdd` CLI (retired, read-only)
- **Context**: v1's UX is the reference for a CI gate that teams actually ran.
- **Sources Consulted**: `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\main.rs` (clap derive, `validate --project . --feature x --all --json`, exit `2` on unusable root/serialisation failure, `status` = same report always exit 0, `vocab-import` prints TOML to stdout), `report.rs` (`SCHEMA_VERSION`, `Finding` with POSIX project-relative `path`, `Provenance.generated_at`, `Summary` counts with `Option` for "layer did not run").
- **Findings**: Two things to keep: exit-code trichotomy and a schema-versioned report; two to drop: `generated_at` (breaks byte-identical runs; the brief forbids timestamps unless explicit) and the `Option`-laden summary (the engine's `CheckStatus::Skipped { reason }` already says why a check did not run).
- **Implications**: `GateReport` carries `schema_version`, per-spec `CheckStatus` and plain counts; no provenance block.

### RDF crates for the secondary importer
- **Context**: Steering names `oxttl`/`oxrdf`; the versions had to be confirmed.
- **Sources Consulted**: crates.io API for `oxttl` (max 0.2.5, 2026-10-18, MIT OR Apache-2.0, "Parser and serializer for … Turtle …") and `oxrdf` (max 0.3.5, 2026-10-18, MIT OR Apache-2.0, "Basic data structures related to RDF").
- **Findings**: Both are maintained (Oxigraph), permissively licensed, pure Rust. `oxttl 0.2` pairs with `oxrdf 0.3`. The Turtle parser reports syntax errors with positions, which satisfies "line of the first problem".
- **Implications**: `oxttl = "0.2"`, `oxrdf = "0.3"` in the CLI crate only; triples are collected into a vector (document order matters for literal fallbacks) and indexed by subject.

### Upstream contract inventory (what the CLI calls)
- **Context**: The CLI must consume, not re-invent.
- **Sources Consulted**: kiro-core design (`Source`, `Workspace::{new, adopt_roadmap_features, load_roadmap, load, feature, successor_chain}`, `Finding` + JSON shape and field order, `sort_findings`, `render::text::findings_text`, `Severity` ordering Error < Warning < Info, Out of Boundary "workspace member list grows when those specs land"); spec-constraints design (`CheckConfig` fields and defaults, `CheckKind::ALL`/`name`, `CheckStatus`, `CheckReport`, `CHECK_CODES`, `run_checks`, the plugin `scope_line`/`status_line` wording, "cli-gate constructs `CheckConfig` from its own flags"); vocabulary-grounding design (`VOCABULARY_PATH`, `Kind`, `Term`, `Topic.line: Option`, `Vocabulary::{resolve, preferred_label, topics, topic, len}`, `Loaded`/`Status`, `vocab::{load, parse}`, `normalize_label`, `is_id`, the normative SKOS → TOML table, the `kiro:` namespace `urn:openidea-kiro:vocab:`, Non-Goal "no JSON → SKOS tool; a cli-gate adapter is a possible follow-up"); spec-visualization design (`viz::{CAPS, RawLabeler, Format, render_body, compose}`, `status_view`/`status_text`, `graph_view`/`graph_summary`, `tasks_view`/`tasks_summary`, the plugin command bodies and error messages, revalidation trigger "cli-gate adopting the views: byte-identical"); plugin-core design (`findings_text` lines "identical to cli-gate's text reporter", host read limits 16 MiB/UTF-8/root-confined).
- **Findings**: The per-spec header lines of the IDE's `check.spec` notification are private functions of the plugin's `check.rs`; they are specified as literal formats in spec-constraints' design but not exported by the engine. `Vocabulary`'s fields are private; id/label existence is answerable through the scoped public API (`preferred_label(id, all_topics)`, `resolve(label, [topic])`).
- **Implications**: The CLI reproduces the header lines from the same literal formats and pins them with a unit test (the documented-duplication pattern spec-visualization uses for `phase_label`); moving them into the engine is a follow-up for spec-constraints, not a change this spec makes. Merge conflict detection uses only the public scoped API.

### Consulted skills and guidance
- `kiro-spec-design` rules (principles, light discovery, synthesis, review gate); `kiro-spec-requirements` EARS rules; no UI/accessibility skills apply (terminal text only, no colour by requirement).

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Thin adapter over the engine (chosen) | One `Source` over `std::fs`, clap front, orchestration per subcommand, reporters, importer | Identical results to the IDE by construction; every finding comes from the engine; small surface | Header-line duplication for the text report; the CLI cannot change check semantics even when CI would like to | Matches steering "engine first, frontends thin" |
| Report-building moved into the engine (`checks::report_text`) | Engine renders the whole check report text | No duplication; plugin and CLI share bytes | Changes spec-constraints' CheckCommands ownership and its approved design; delays this spec | Recorded as follow-up |
| CLI config file (`.kiro/check.toml`) | Caps, overrides and selections in a committed file | One place for CI and developers | Second configuration surface with no requirement behind it; precedence questions | Rejected (out of scope); flags only |
| JSON → SKOS → TOML (brief's earlier path) | Reuse the SKOS reader for Auto Ontology by converting first | One reader | Two conversions, a fictional intermediate, the exact gap the brief closes | Rejected; both readers feed one writer |

## Design Decisions

### Decision: Two readers, one `ImportedTerm` model, one TOML writer
- **Context**: Two input formats must produce the same TOML shape, and `--merge` must work for both.
- **Alternatives Considered**: 1. Convert JSON to SKOS then import; 2. Separate writers per format; 3. Shared model + shared writer (chosen).
- **Selected Approach**: `import::read(format, inputs, defaults) -> Imported` dispatches to `auto_ontology::read` or `skos::read`; both return `ImportedTerm`s (built from `kiro_core::vocab::Kind`) plus provenance comment lines; `render` and `merge` are format-agnostic.
- **Rationale**: One place for id derivation, deduplication, escaping, ordering and validation; readers stay small.
- **Trade-offs**: `ImportedTerm` duplicates `Term`'s fields (needed: `Term` has no comments and is constructed only by the engine's index).
- **Follow-up**: `derive_id` must satisfy the three CamelCase examples of the mapping table (`InsiderThreat`, `HTTPSProxy`, `Level2Cache`) and the space/underscore cases from Auto Ontology names.

### Decision: `--merge` is append-only
- **Context**: Merging must keep existing terms and definitions and report conflicts rather than overwrite (brief).
- **Alternatives Considered**: 1. Parse existing file to a model and re-emit the union (loses comments and hand formatting); 2. Append only new `[terms.*]`/`[topics.*]` tables after the existing text (chosen).
- **Selected Approach**: The existing text is kept byte-for-byte; a merge header comment and the new tables are appended; a `[topics.<t>]` table is appended only when the existing vocabulary has no declared table for it (`Topic.line.is_none()`), avoiding a TOML redefinition; conflicts (id exists, or label/alt label resolves in the same topic) are `IMPORT_MERGE_CONFLICT` findings and the imported term is dropped. The merged text is then validated by the engine loader (`vocab::parse`).
- **Rationale**: Reviewable diffs (only additions), no loss of maintainer curation, deterministic.
- **Trade-offs**: An existing file that is invalid cannot be merged into (exit 2 with the `VOCAB_INVALID` line) — acceptable, it must be fixed first.
- **Follow-up**: Integration test: merge into a file with comments and a deprecated term; assert the prefix is unchanged and only additions follow.

### Decision: Importer diagnostics on standard error, text only
- **Context**: `--format` on `vocab import` names the input format (steering fixes `--format auto-ontology-json`), so it cannot also select a report format; `--out -` puts TOML on standard output.
- **Selected Approach**: Import and output-validation findings are printed in the `findings_text` form on standard error; standard output carries only the TOML when `--out -`, nothing otherwise.
- **Rationale**: One rule for both output modes; the importer is a maintainer tool, not a CI consumer.
- **Trade-offs**: No JSON diagnostics for import; `vocab validate --format json` over the written file covers the machine-readable need.

### Decision: Explicit file arguments are unconfined; engine reads are confined
- **Context**: Root confinement is a requirement for engine reads (parity with the plugin host); export files and vocabularies under validation routinely live outside the project.
- **Selected Approach**: `FsSource::read` rejects absolute paths, `..` segments and anything outside the root lexically (no canonicalisation per read; symlinks inside the tree are the maintainer's); `read_argument_file` reads operator-given paths as given. All `std::fs` use is confined to one module.
- **Rationale**: Mirrors the plugin host for the engine's reads while keeping the maintainer tools usable.
- **Trade-offs**: A symlink inside `.kiro` pointing outside the project is followed; documented.

### Decision: `check` reports structural findings and vocabulary/roadmap findings once
- **Context**: The IDE's `check.spec` shows only the check report; `ws.current` shows structural findings; a CI gate needs both and must not repeat vocabulary findings per spec.
- **Selected Approach**: Per spec: `feature.findings` ∪ `CheckReport.findings` minus findings whose file is `.kiro/vocabulary.toml` (those are the pass-through of the vocabulary's own status and are printed once in the vocabulary section), merged and sorted with `sort_findings`. Roadmap findings once from `load_roadmap()`. Vocabulary findings once from `vocab::load`.
- **Rationale**: Requirement 3.2/3.3; deterministic; no new codes.
- **Trade-offs**: A reader comparing with `check.spec` sees extra structural lines per spec; the header lines make the source obvious (`file` differs).

### Decision: Importer codes live in the engine catalogue
- **Context**: `Finding.code` must be one of `CODES` (kiro-core invariant and test); the importer produces findings.
- **Selected Approach**: Four codes (`IMPORT_INPUT_INVALID` error, `IMPORT_TERM_SKIPPED` warning, `IMPORT_DUPLICATE_ID` warning, `IMPORT_MERGE_CONFLICT` warning) appended to `finding::CODES`/`codes` and the `lib.rs` table — the growth path vocabulary-grounding (15 codes) and spec-constraints (19 codes) used. The engine never emits them; their "Emitted by" column says `kiro-cli importer`.
- **Trade-offs**: The engine's catalogue names a frontend; accepted because the alternative (a second catalogue) breaks the single-shape invariant.

### Decision: Header lines reproduced, pinned by test; views printed unchanged
- **Context**: Text parity with the IDE (Requirement 4.2, 7.1–7.3).
- **Selected Approach**: `report::scope_line`/`status_line` implement the literal formats of spec-constraints' CheckCommands; a unit test asserts the strings for a constructed report. Views are the engine's `status_text`/`compose` output unchanged; a test compares `status demo` with kiro-core's `viz-status-demo.txt` golden file (the same file the plugin test uses).
- **Follow-up**: If spec-constraints later moves the line formatters into the engine, the CLI switches to them and the duplication test disappears.

### Synthesis outcomes
- **Generalisation**: one `Failure(String)` → exit 2 path for every command; one `Exit` enum; one `fail_on` rule (`meets(fail_on, severity)`) shared by `check` and `vocab validate`.
- **Build vs adopt**: adopt `clap` (derive), `serde_json` (lenient `Value` reading and the report envelope), `oxttl`/`oxrdf` (Turtle), `tempfile` (tests). Build: a 40-line TOML writer with escaping (comments interleaved with tables rule out the `toml` serializer); `derive_id` (the mapping's rule, nothing standard).
- **Simplification**: no `Reader` trait (an enum dispatch with two arms); no config file; no colour; no `generated_at`; no per-command `--project` duplication (one global option); no `info` level for `--fail-on` (brief: `error|warning`).

## Risks & Mitigations
- Auto Ontology changes its page shape — mitigation: both shapes accepted, unknown properties ignored, fixtures copied from the OpenAPI examples, the docs name the OpenAPI path to re-check.
- Header-line drift between plugin and CLI — mitigation: unit test with the literal strings from spec-constraints' design; follow-up to move them into the engine.
- Byte-identical views across platforms — mitigation: `\n` only, `/` paths, no locale-dependent formatting; the self-tree determinism test runs on Windows (dev) and Linux (CI).
- `oxttl` 0.2 API drift at implementation time — mitigation: the reader is one file; pin the minor version; the Turtle fixtures exercise every mapped predicate.
- Large trees in CI — mitigation: the engine's per-document caps; the release-only 20-spec timing test.

## References
- [NVIDIA Auto Ontology README](https://github.com/NVIDIA/auto-ontology) — `GET /api/terms`, API token header.
- [Auto Ontology OpenAPI](https://raw.githubusercontent.com/NVIDIA/auto-ontology/main/docs/openapi/auto-ontology-api.json) — `TermsPageResponse`, `TermListItem`, `TermDetail`, `TermSummary`, `ZoneChip`, `ExportRequest`.
- [oxttl on crates.io](https://crates.io/crates/oxttl), [oxrdf on crates.io](https://crates.io/crates/oxrdf) — versions and licences.
- `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\{main.rs,report.rs}` — prior art for the CLI UX and report envelope.
- `.kiro/specs/{kiro-core,spec-constraints,vocabulary-grounding,spec-visualization,plugin-core}/design.md` — consumed contracts.
