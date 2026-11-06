# Research & Design Decisions

## Summary
- **Feature**: `vocabulary-grounding`
- **Discovery Scope**: Extension (light discovery) — adds the `vocab` module to kiro-core and the `vocab` command group to plugin-core through the extension points both upstream designs define; plus an external check of NVIDIA Auto Ontology's export surface.
- **Key Findings**:
  - **Auto Ontology does not export SKOS.** Its README (`github.com/NVIDIA/auto-ontology`) and `docs/openapi/auto-ontology-api.json` expose terms as JSON (`GET /api/terms`, `GET /api/terms/{id}`: `id`, `name`, `description`, `synonyms`, `related_terms`, `zones`, `tags`) and a model export (`POST /api/model/export`, YAML in `auto_ontology` format or an Apache Ossie zip with `include_ontology`). No SKOS, RDF, Turtle, OWL or JSON-LD appears anywhere; no `broader`/`narrower`, `definition`, `domain` or `topic` field exists on a term. The brief's statement that Auto Ontology "exports SKOS/OWL/RDF" is not supported by the current documentation.
  - **Consequence**: the importer contract stays SKOS Turtle → TOML (standard interchange, the shape v1 already round-tripped, and what any thesaurus tool can produce), and the documentation describes the Auto Ontology side as "verified JSON surface → SKOS conversion the maintainer confirms against the installed version". Requirement 10.1 already demands this verified/unverified split. The JSON → SKOS correspondence (`name` → `skos:prefLabel`, `description` → `skos:definition`, `synonyms` → `skos:altLabel`, `related_terms` → `skos:related`, never `broader`; `zones` → candidate `kiro:topic`) is recorded as a convention, not a contract.
  - **The upstream extension points are sufficient.** kiro-core: `Workspace::read` (cached raw read), `SpecMeta.topics: Vec<String>`, `Finding::new` with the `CODES` catalogue and the `lib.rs` doc test that asserts documentation and catalogue agree; plugin-core: `FeatureModule` (`group`, `commands`, `run`), `CommandSpec` with `Positional::PassThrough`, `Request.rest`, `CommandContext::{resolve, load, workspace}`, `Output::{info, with_level, with_findings}`, `render::text::{table, findings_text}`. No upstream API is missing; three upstream files are modified at the documented extension points (`finding.rs` catalogue, `lib.rs` module list and docs, kiro-core `Cargo.toml` dependency; plugin `commands.rs` module table, `lib.rs`, `plugin.toml`, `README.md`).
  - **Adding `toml` to kiro-core is a documented revalidation trigger** ("adding a required runtime dependency") and is sanctioned by steering (`toml` listed as the vocabulary parser, wasm-safe). `toml` 0.8 is pure Rust, builds for `wasm32-wasip2`, and provides `toml::Spanned<T>` for line numbers and `de::Error::span()` for the first structural error.
  - **The IDE passes no argument** (plugin-core research). `vocab.term` therefore degrades to a scoped term list with a usage line when no query is given, and all three commands are spec-addressable so the active file or a per-spec variant selects the scope.

## Research Log

### Auto Ontology export surface
- **Context**: The brief assumes an exported SKOS/OWL/RDF artifact seeds `.kiro/vocabulary.toml`.
- **Sources Consulted**: `https://github.com/NVIDIA/auto-ontology` (README: product description, `docker compose up -d --build`, `curl -H "x-api-key: $AUTO_ONTOLOGY_API_TOKEN" …/api/terms`, pointer to `docs/openapi/auto-ontology-api.json`); the OpenAPI document (read in full, 437 k characters): paths `/api/terms`, `/api/terms/{term_id}`, `/api/terms/{term_id}/column-attributes`, `/api/terms/{term_id}/sql-attributes`, `/api/exploration/terms/{term_id}/details`, `/api/exploration/semantic-graph`, `/api/model/export`, `/api/model/import`, `/api/acronyms`, `/api/tags`, `/api/zones`, `/api/search/global-search`; schemas `Term`, `TermDetail`, `TermSummary`, `TermsPageResponse`, `ExportRequest` (`databases`, `format` ∈ `auto_ontology | ossie`, `include_ontology`), `SemanticGraphNode` (`synonyms`, `description`).
- **Findings**: Terms carry `name`, `description`, `synonyms` (strings), `related_terms` (untyped relation), `zones`, `tags`, `certification`, `owner_id`. There is no hierarchy field, no definition/label distinction, and no subject-domain field; `zones` are organisational groupings. Model export is YAML (`auto_ontology`) or an Ossie ontology zip; neither is an RDF serialisation.
- **Implications**: The SKOS mapping is the importer's contract and must not depend on Auto Ontology specifics. The documentation gives the maintainer: (a) the verified way to pull terms (`GET /api/terms?query=&skip=&limit=` with the API token), (b) a conversion convention from that JSON to SKOS Turtle with `kiro:topic`/`kiro:kind` annotations, marked as unverified against a live instance, and (c) the verified import step (`kiro-cli vocab import`, cli-gate) and acceptance check (loader findings). The seed vocabulary for `sdd` is hand-authored in the importer's output shape, with its header stating that, so it can be replaced by a real import without changing consumers.

### kiro-core extension points
- **Sources Consulted**: `.kiro/specs/kiro-core/design.md` (Boundary Commitments, Finding, SpecMetaParser, Workspace, Resolver, TextRenderer, finding catalogue, lints), `.kiro/specs/kiro-core/requirements.md` (2.1, 2.6 on `topics`).
- **Findings**: `SpecMeta.topics` is already parsed (empty when absent, no finding). `Workspace::read(path) -> Option<&str>` caches raw reads including `None`. `Finding::new(code, file, message)` takes severity from `CODES`; a unit test iterates `codes::*` against `CODES`; `lib.rs` docs list the codes and a doc test compares them to the catalogue. Dependency direction: `finding` → `md` → parsers → `join` → `source` → `resolve`. kiro-core's boundary explicitly leaves `vocab` to this spec and says nothing is reserved for it.
- **Implications**: `vocab` sits right of `source` (it uses `Workspace::read`, `Feature`, `SpecMeta`) and imports nothing from `resolve`. New codes are appended to `CODES` and `codes`, and to the `lib.rs` table. `toml` is added to `[dependencies]`.

### plugin-core extension points and output budget
- **Sources Consulted**: `.kiro/specs/plugin-core/design.md` (Registry, Args, Present, Context, Commands, WsCommands, budget worked example, message catalogue), `research.md` (argument channel, Ok message discarded).
- **Findings**: `Group::Vocab` exists; `CommandSpec { spec_addressable, insert_capable, positional }`; `Request.rest` holds pass-through tokens; `CommandContext::resolve(req.spec)` and `load(name)`; `Output::with_findings` renders findings exactly as the CLI; `deliver` clips at 15 KiB. The worked budget assumes `vocab` contributes 3 base commands. `ctx.load` borrows the workspace immutably while `vocab::load(&mut ctx.workspace)` needs it mutably.
- **Implications**: Three commands, one insert variant (`vocab.list.insert`), matching the budget example. The module copies the spec's topics out of the loaded `Feature` into an owned `Vec<String>` before calling `vocab::load`, so no borrow overlaps. A 200-term listing is ≈ 20–30 KiB and relies on `deliver`'s line clipping; `vocab.list.insert` carries the full table.

### Prior art: `ears-sdd` `vocabulary.rs` and `skos.rs`
- **Sources Consulted**: `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\vocabulary.rs` (TOML shape, `normalized_label`, `check_labels`, `check_broader`, `reaches_itself`), `skos.rs` (`export`/`import`, `local_name` CamelCase → kebab-case, narrower inversion, `ears:` namespace, empty definitions on import).
- **Findings**: v1 keyed terms globally (`[terms.<id>]`), normalised labels by whitespace collapse + lower-case, detected cycles per term by DFS, treated `domain` as the value kind. Its SKOS bridge is the only concrete mapping in the codebase and already handles narrower inversion, CamelCase ids, and lossless round-trip of kinds via a private namespace.
- **Implications**: v2 keeps the global term id (a join key), the label normalisation and the per-term cycle report; renames `domain` → `kind`; adds `topic` and the rule that `broader`/`replaced_by` stay inside the topic; adopts the SKOS mapping with `kiro:` replacing `ears:` and `kiro:topic` added. `std::fs`, feature-local vocabularies, `TERM_UNUSED` and the scaffold command are not carried (out of scope or CLI-only).

### `toml` crate on `wasm32-wasip2`
- **Sources Consulted**: steering `tech.md` (`toml` listed as wasm-safe); `toml` 0.8 API (`toml::from_str`, `toml::Spanned<T>`, `toml::de::Error::span() -> Option<Range<usize>>`); plugin-core `Cargo.toml` already uses `toml` 0.8 as a dev-dependency.
- **Findings**: Pure Rust (`toml_edit` + `serde_spanned` underneath), no I/O, builds for wasm. `Spanned<T>` on map values gives each term table's byte span; `de::Error::span()` gives the first structural error's position. Line numbers are derived by counting `\n` up to the byte offset.
- **Implications**: Strict serde parsing (`#[derive(Deserialize)]`, unknown keys allowed, `default` for optional fields) with `Spanned` wrappers; any structural error → `VOCAB_INVALID` with a line, no partial vocabulary. Semantic problems (definitions, kinds, labels, relations) are findings on a loaded vocabulary. This matches requirement 2.3 and is simpler than field-by-field degradation, which is right for a curated schema file (unlike `spec.json`).

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Scoped query API on an indexed value (chosen) | `Vocabulary` is built once from the file with per-topic label indexes and the broader closure; every query takes `topics: &[String]` and returns a `Lookup` enum | The topic rule is enforced by the type: no entry point without topics; outcomes are explicit (`NotInTopic`, `Ambiguous`, `Unscoped`); pure and wasm-safe | Index built on every command (no caching across calls — same as the rest of the plugin) | Mirrors kiro-core's "plain data + findings" style |
| Global lookup + caller-side topic filter | `term_by_label(label)` returns the term; callers check `term.topic` | Smaller API | Exactly v1's bug: a caller that forgets the check gets a cross-topic match; cannot distinguish `NotInTopic` from `Unknown` without a second pass | Rejected (violates 4.9) |
| One vocabulary file per topic (`.kiro/vocab/<topic>.toml`) | Topic = file | Natural scoping | The plugin cannot list directories, so it could only read topics a spec declares; cross-topic `NotInTopic` diagnostics become impossible | Rejected |
| Field-by-field tolerant parse via `toml::Table` | Like `spec.rs` | Keeps partial data on wrong types | Loses spans, doubles the code, and a curated file should fail loudly at the line | Rejected in favour of strict parse + semantic findings |
| Per-term command variants (`vocab.term.<id>`) for the missing argument | Each term becomes a command | Works from Search Everywhere | 256-id budget; a 50-term vocabulary with 7 specs exhausts it | Rejected; `vocab.term` without a query lists visible terms instead |

## Design Decisions

### Decision: Topic scoping enforced by the query API, not by callers
- **Context**: The hard rule of the spec — no API may resolve a term outside the declared topics; cross-topic hits are a distinct result.
- **Alternatives Considered**: 1. Global lookup with caller filtering. 2. A `Scope` object constructed from a spec that owns a filtered copy of the vocabulary. 3. Every query takes `topics` and returns a `Lookup` enum.
- **Selected Approach**: Option 3. `Vocabulary::resolve(query, topics)`, `preferred_label(id, topics)`, `visible(topics)`; the only unscoped reads are topic-level (`topics()`, `topic(id)`) and the broader closure of a term already obtained from a `Found` outcome. `Lookup::Unscoped` is returned for an empty topic list before any index is consulted.
- **Rationale**: A reviewer can grep for `resolve(` and see the topics argument at every call site; `NotInTopic` carries only `(topic, id)` hits so the "found elsewhere" diagnostic never leaks definitions across topics.
- **Trade-offs**: `Scope` objects (option 2) would avoid passing the list repeatedly; rejected as a second way to hold topics that could drift from `spec.json`.
- **Follow-up**: The independent design review found that a `broader_of(&Term)` entry point would let a caller forge a `Term` (public fields) and read another topic's closure; the closure therefore travels inside `Lookup::Found { broader }` and no public function returns `&Term` without a `topics` parameter (documented in `lib.rs`; checked by API review).

### Decision: Term ids are global, labels are unique per topic
- **Context**: v1 keyed terms globally; v2 needs the same label in two topics ("task" in `sdd` and in `pm`).
- **Alternatives Considered**: 1. Ids scoped per topic (`[topics.sdd.terms.task]`). 2. Global ids, per-topic label uniqueness.
- **Selected Approach**: Option 2. `[terms.<id>]` with `topic = "…"`; `TERM_DUPLICATE_LABEL` is checked per topic; the same label in another topic is allowed and is what `NotInTopic` reports.
- **Rationale**: Ids are join keys for `broader`, `replaced_by`, `_Requirements`-style annotations and the SKOS local name; a flat id space keeps those references unambiguous and the TOML flat and diff-friendly. A duplicate TOML key is a parse error, so duplicate ids cannot occur.
- **Trade-offs**: Two topics cannot both have a term literally named `task` as id; the convention is `task` and `pm-task` (ids are machine names; labels are the human names and may both be "Task").
- **Follow-up**: Document the id convention in `docs/vocabulary.md`.

### Decision: Strict parse, semantic findings
- **Context**: Requirement 2 distinguishes `VOCAB_INVALID` (structure) from per-term findings.
- **Selected Approach**: `toml::from_str::<RawFile>` with `Spanned` wrappers; a deserialisation error becomes one `VOCAB_INVALID` at the error's line with status `Invalid` and an empty vocabulary; everything after a successful parse is a semantic finding on a `Loaded` vocabulary. Ids that violate the id rule are structural (`VOCAB_INVALID`) because they would be unusable as references.
- **Rationale**: The file is curated and small; a clear "fix line N" beats partial data. Semantic findings keep the term so that the rest of the vocabulary stays usable.
- **Trade-offs**: One wrong-typed field hides the rest of the file's problems until it is fixed.

### Decision: `vocab.term` without a query lists the visible terms
- **Context**: The IDE passes no argument; `vocab.term` needs a query.
- **Alternatives Considered**: 1. Error "argument required". 2. Per-term variants. 3. Usage line + the `vocab.list` body.
- **Selected Approach**: Option 3 (requirement 6.5). The command is `Positional::PassThrough` and spec-addressable, so `vocab.term.<spec>` with an argument works programmatically and `vocab.term` from Search Everywhere still shows something useful.
- **Trade-offs**: Two commands produce the same table in one situation; accepted for usability.

### Decision: SKOS → TOML remains the importer contract; Auto Ontology JSON → SKOS is a documented convention
- **Context**: Auto Ontology does not export SKOS (Research Log).
- **Selected Approach**: `docs/vocabulary.md` defines the SKOS → TOML mapping normatively (what cli-gate implements and tests), then describes the verified Auto Ontology JSON surface and a conversion convention to SKOS with explicit "confirm against your installed version" markers.
- **Rationale**: The importer must not depend on a vendor's JSON that this project cannot verify; SKOS is stable and tool-neutral. The seed vocabulary is hand-authored in the importer's output shape so a future real import is a drop-in replacement.
- **Follow-up**: When cli-gate is designed, decide whether a `--from auto-ontology-json` adapter is worth adding; nothing here precludes it.

### Synthesis outcomes
- **Generalisation**: `resolve`, `preferred_label`, `vocab.term` and spec-constraints' declared-before-use are all the same scoped query; one `Lookup` enum serves them all (navigation labels use `preferred_label`, which is `resolve` restricted to id matches).
- **Build vs adopt**: `toml` adopted for parsing; SKOS adopted as the interchange model; nothing else exists to adopt for topic-scoped resolution (a thesaurus library would bring RDF into the wasm build).
- **Simplification**: no `Scope` object, no feature-local vocabularies, no caching layer, no `TERM_UNUSED` (spec-constraints may add it), no scaffold command (cli-gate may add it), no `Deserialize` on model types (raw types are private).

## Risks & Mitigations
- `toml` crate size in the wasm component — `toml` 0.8 with default features pulls `toml_edit`; mitigation: enable only `parse` (features `["parse"]`, `default-features = false`) and measure `dist/plugin.wasm` before and after.
- Large vocabularies and the 16 KiB notification — mitigation: `vocab.list` relies on `deliver`'s line clipping with a trailer; the insert variant carries the full table; the definition column is shortened to one line.
- Topic drift between `spec.json` and the vocabulary — mitigation: `TOPIC_UNKNOWN` on `spec.json` and the "not declared" list in `vocab.topics` make drift visible from both sides.
- The brief's Auto Ontology premise — mitigation: documentation separates verified facts from conventions; the seed is hand-authored in the import shape.
- Borrow conflicts in the plugin module between `ctx.load` and `vocab::load(&mut ctx.workspace)` — mitigation: copy `topics` into an owned vector first; covered by the end-to-end tests compiling.

## References
- [NVIDIA Auto Ontology README](https://github.com/NVIDIA/auto-ontology) — product, `GET /api/terms`, API token header.
- [Auto Ontology OpenAPI](https://raw.githubusercontent.com/NVIDIA/auto-ontology/main/docs/openapi/auto-ontology-api.json) — `Term` schema, `/api/model/export` formats.
- [SKOS Reference](https://www.w3.org/TR/skos-reference/) — `prefLabel`, `altLabel`, `definition`, `broader`/`narrower`, `inScheme`.
- `D:\Source\spec-kit-ears-tdd\crates\ears-sdd\src\{vocabulary.rs,skos.rs}` — prior art (read-only).
- `.kiro/specs/kiro-core/design.md`, `.kiro/specs/plugin-core/design.md` — upstream contracts consumed exactly.
