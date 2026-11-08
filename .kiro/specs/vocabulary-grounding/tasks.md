# Implementation Plan

- [ ] 1. Foundation: engine integration points, fixtures and model
- [x] 1.1 Integrate the vocabulary seam into the engine crate
  - Prerequisite: `crates/kiro-core` is implemented and builds for native and the sandbox target (kiro-core spec), and `plugins/kiro` is implemented and builds its component (plugin-core spec) so the size baseline below can be taken
  - Record the current size of the built plugin component as the baseline before the engine gains a new dependency; the figure is reported by task 5
  - Add the TOML parser as a runtime dependency with default features off and only parsing enabled; declare the vocabulary module with one documented empty stub per file of the File Structure Plan (model, parse, index, scope, loader) so later tasks own their files
  - Append the sixteen vocabulary finding codes (including `VOCAB_LABEL_CROSS_TOPIC`) to the finding catalogue and the constants module with the default severities and descriptions of the design's Supporting References; extend the crate documentation's code table and add the vocabulary grammar subsection so the existing doc test keeps documentation and catalogue in agreement
  - Explicit integration touch: verify the kiro-core design document's Allowed Dependencies line already names the TOML parser (amended during cross-spec review)
  - Observable: native build, sandbox-target build, clippy with warnings denied, fmt check and the existing catalogue tests pass; the dependency tree for the sandbox target shows no filesystem-dependent crate; the baseline size is noted
  - _Boundary: CoreIntegration_
  - _Requirements: 2.14, 2.15, 11.1, 11.4_

- [x] 1.2 (P) Create the vocabulary fixtures and the fixture workspace tree
  - Five TOML fixtures as listed in the design: a valid two-topic vocabulary (hierarchy of depth three, alternative labels, a deprecated term with replacement, enum and int kinds, the label "Task" in both topics `sdd` and `pm`, which yields zero warning/error findings and one `VOCAB_LABEL_CROSS_TOPIC` info finding on `pm-task`), a syntax-error file, a structure-error file (wrong-typed field and an invalid id), a semantic file exercising every semantic finding code once, and a cycle file with a three-term cycle, a self-loop and a term pointing into the cycle
  - A workspace tree with a copy of the valid vocabulary, a roadmap listing four specs, and four `spec.json` files: topics `sdd`; topics `sdd` and `pm`; no topics; topics `sdd` and an unknown topic
  - Each fixture carries a header comment stating the findings it is expected to produce and the lines they land on
  - Observable: the files exist at the planned paths and the expected-findings comments match the design's finding catalogue
  - _Boundary: VocabFixtures_
  - _Requirements: 2.15, 11.5, 11.6_

- [ ] 1.3 Implement the vocabulary model types and normalisation
  - Blocked by 1.1 (module stub and dependency); may run alongside 1.2 only
  - Term, topic, kind (entity, bool, enum with values, int with optional bounds), hit and vocabulary value types with the fields and privacy of the design; the vocabulary's indexes are private and only the engine constructs it
  - Label normalisation (trim, collapse internal whitespace, Unicode lower-case) and the id rule validator; the vocabulary path and schema version constants
  - Observable: unit tests cover the normalisation cases (case, inner whitespace, tabs, Unicode), the id rule table (valid, uppercase, leading hyphen, underscore, empty) and the kind name mapping
  - _Boundary: VocabModel_
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 4.9_

- [ ] 2. Core engine: parse, validate, load, query
- [ ] 2.1 Implement the strict vocabulary file parser
  - Private raw types with span wrappers for the schema version, topic tables and the mandatory term table; unknown keys accepted; a BOM stripped before parsing
  - Any deserialisation error (including a missing term table) or an id violating the id rule becomes one structural finding carrying the line of the first problem, computed from the byte offset
  - Observable: unit tests show the structural finding's line for a syntax error, a wrong-typed field, a missing term table and an invalid id; BOM and CRLF inputs parse; the line computation clamps at the end of the text; arbitrary text never panics
  - _Boundary: VocabParser_
  - _Requirements: 1.1, 2.3, 11.1_

- [ ] 2.2 Implement semantic validation and the per-topic label index
  - Build steps 1 to 5 and 7 of the design in order: schema version check at its line; definition and kind checks keeping the term (invalid kind becomes entity); declared and implicit topics with term counts and the empty-topic finding; per-topic label index with the duplicate-label finding at the later term in file order while both terms stay indexed, plus the cross-topic label-collision lint (`VOCAB_LABEL_CROSS_TOPIC`, info) at the later term in file order naming both ids and their topics when one normalised label or alternative label occurs in two topics; broader validation removing unknown and cross-topic relations with their findings; replacement validation (unknown, cross-topic, deprecated without replacement) keeping the value as written
  - Findings carry the vocabulary file path and the term's or topic's table line and are sorted at the end of the build
  - Observable: unit tests from constructed raw values produce each of these finding codes (including `VOCAB_LABEL_CROSS_TOPIC`) exactly where the design says, and a single-topic valid raw file builds with no finding
  - _Boundary: VocabIndex_
  - _Requirements: 1.6, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.11, 2.12, 2.13, 2.14, 2.15_

- [ ] 2.3 Implement the hierarchy closure and cycle detection
  - Build step 6 of the design: one walk per term over the kept broader relations with a visited set, the start term a legal target; the walk order is the nearest-first closure; a term that reaches itself gets the cycle finding at its line and is removed from its own closure; terms that merely point into a cycle are not flagged and keep a finite closure
  - Observable: unit tests show the closure order for a diamond hierarchy and the depth-three chain, the cycle set for the three-cycle, self-loop and pointer-into-cycle cases, and that a generated vocabulary of ten thousand terms with depth ten builds without error (timing is asserted only by the release-only test of 2.8)
  - _Boundary: VocabIndex_
  - _Requirements: 2.10, 2.14, 5.1, 11.2_

- [ ] 2.4 Implement the loader and the module root
  - Status (loaded, missing, invalid) and the loaded value bundling status, vocabulary and sorted findings; text-level parse producing invalid status with an empty vocabulary or loaded status with build findings; workspace-level load reading the vocabulary path through the workspace cache once, mapping absence to the missing status with its info finding
  - Re-exports of the model types and constants from the module root (the query types are re-exported by 2.6)
  - Observable: unit tests show one read per workspace lifetime for present and absent files, the missing and invalid statuses with their single finding, and an empty vocabulary whenever the status is not loaded
  - _Boundary: VocabLoader_
  - _Requirements: 2.1, 2.2, 2.3, 8.5_

- [ ] 2.5 Implement scoped resolution and preferred-label lookup
  - The single resolution operation taking the query and the topic list: unscoped on an empty list without consulting indexes; the normalised query compared with ids, labels and alternative labels of the searched topics; found with the match kind, the term's closure and the in-scope replacement; ambiguous and not-in-topic carrying only sorted topic/id hits; unknown otherwise; answers independent of topic order
  - Preferred-label lookup as the same operation restricted to ids; no public function returning a term without a topic list
  - Observable: unit tests cover every outcome variant, whitespace-padded queries matching an id, the closure inside the found outcome, in-scope-only replacement, topic-order insensitivity, that not-in-topic hits expose no definition, and that preferred label ignores labels
  - _Boundary: VocabScope_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 5.1, 5.2_

- [ ] 2.6 Implement listings, spec topic derivation and scope findings
  - Visible terms ordered by topic then id; available topics with labels and counts; spec topic derivation from the spec model (trimmed, non-empty, deduplicated, declaration order); scope findings for undeclared topics and unknown topics, the latter only while the vocabulary is loaded, taking owned spec name, spec.json path and topics
  - Explicit integration touch: add the query-type re-export line to the module root owned by 2.4
  - Observable: unit tests cover visible ordering, implicit topics, spec topic trimming and deduplication, and the scope findings for loaded, missing and invalid vocabularies; consumers import every public type from the one module path
  - _Boundary: VocabScope, VocabLoader_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 5.3, 5.4_

- [ ] 2.7 (P) Author the seed vocabulary for the `sdd` topic
  - Header comment naming the origin (hand-authored in the importer's output shape, to be replaced by a real import), the schema version and the documentation path; a `topics.sdd` entry with label and description
  - The design's seed term set with one-sentence definitions taken from steering and the kiro-core design: at least the fourteen required terms plus the listed extras, with the broader relations, alternative labels, the enum kinds for EARS pattern, phase and severity, and the bool kind for approval
  - Explicit integration touch: add only the key `"topics": ["sdd"]` to this spec's own `spec.json` (`.kiro/specs/vocabulary-grounding/spec.json`), leaving the workflow-managed fields (phase, approvals, timestamps) untouched, so the repository's first grounded spec exercises the seed and the host check of task 5 has a declared topic
  - Observable: the file loads with status loaded and no warning or error finding, and contains every required term id (asserted by the core integration tests of 2.8)
  - _Boundary: SeedVocabulary_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [ ] 2.8 Write the engine integration tests over the fixtures and the seed
  - Fixture-driven tests through an in-memory source and workspace: status and exact finding-code list per fixture with line numbers (including the single `VOCAB_LABEL_CROSS_TOPIC` info finding on `valid.toml` at `pm-task` and zero warning/error findings); every lookup outcome on the fixture tree; closure for the depth-three chain and the pointer-into-cycle term; preferred label; visible ordering; scope findings for the no-topics, unknown-topic and missing-vocabulary cases
  - The seed file embedded from the repository, asserted to load cleanly with zero findings and the required ids; two runs produce identical findings; a release-only timing test for ten thousand terms with depth ten under one second; fixed-seed garbage documents through the parser never panic
  - Observable: the test file passes natively, and the engine library still builds for the sandbox target
  - _Boundary: CoreVocabTests_
  - _Depends: 1.2, 2.7_
  - _Requirements: 2.15, 9.2, 11.1, 11.2, 11.3, 11.5_

- [ ] 3. (P) Write the vocabulary documentation and the import mapping
  - File format and id convention (global ids, per-topic labels, the `<topic>-<name>` convention), the `topics` field of `spec.json` and the documented template change adding an empty list, the no-vocabulary and invalid-vocabulary behaviour of the plugin commands
  - Auto Ontology procedure separating verified facts (terms API `GET /api/terms`, API token header `x-api-key`, model export formats) from steps the maintainer confirms against the installed version, documenting both the direct JSON import path (`kiro-cli vocab import <export.json> --format auto-ontology-json --topic <t>`, supplied by cli-gate) and the SKOS Turtle path (`kiro-cli vocab import <file.ttl> --topic <t>`) with the target Turtle shape
  - The normative SKOS to TOML mapping table of the design (concept, id derivation, preferred label, definition, alternative labels, broader and inverse narrower, deprecation, replacement, topic and kind annotations, ignored subjects, empty definitions, id-sorted output), the `kiro:` namespace and its private-extension status, and the review-and-commit procedure using the loader's findings as the acceptance check
  - Explicit integration touch: add the `docs/` directory pattern to the structure steering document, as this spec's design requires
  - Observable: the document exists at the planned path with every section above and its mapping table equals the design's table row for row
  - _Boundary: VocabDocs_
  - _Requirements: 3.5, 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ] 4. Plugin: the `vocab` feature module
- [ ] 4.1 Implement the vocabulary feature module, per-call scoping, degradation and the term command
  - Prerequisite: `plugins/kiro` is implemented per the plugin-core spec (feature-module contract, command context, presentation, fake host, manifest test, build script) and the `../openidea` checkout providing the host WIT is present for the sandbox build
  - Command declarations (term, list, topics; all spec-addressable, pass-through arguments, list insert-capable) and the feature-module implementation dispatching on the command name, with the list and topics branches stubbed until 4.2; explicit integration touch: the crate-root module declaration
  - Per-call scoping: resolve the spec with plugin-core's error passed through, load the feature, copy spec name, spec.json path and topics into owned values, then load the vocabulary once and compute the scope findings; one debug log line per call through the host port
  - Shared degradation outputs for a missing vocabulary (info), an invalid vocabulary (error with the finding text) and a spec without topics (info with instructions and the available topics)
  - The term command's outputs for found (all fields, closure, the three deprecation branches, match kind, scope), not-in-topic, ambiguous, unknown and empty query (usage line plus the visible-terms table), with the findings-count line on every outcome
  - Observable: the plugin crate compiles natively with the module declared; unit tests cover the kind, one-line and hits text helpers, the three degradation outputs and the term outputs for each outcome
  - _Boundary: VocabCommands, PluginIntegration_
  - _Depends: 2.6_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 8.1, 8.2, 8.3, 8.4_

- [ ] 4.2 Implement the list and topics commands
  - The list command: spec line with declared topics, the optional topic filter restricted to declared topics with the undeclared-topic warning, the term table (id, label, topic, kind, one-line definition) in visible order, the findings-count line, and the whole text as the inserted content when the insert flag is set
  - The topics command: declared, available (with counts and labels), not-declared and unknown-to-the-vocabulary lines, the vocabulary status line, and all scope and vocabulary findings in the shared text form with the level derived from them; for a spec without topics it still lists the available topics
  - Observable: unit tests cover the list table and filter branches and the topics command lines for the four fixture specs
  - _Boundary: VocabCommands_
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 8.3_

- [ ] 4.3 Register the module in the plugin package
  - Insert `vocab::Vocab` at its canonical position in the plugin's module table (`ws, nav, viz, vocab, check`); the manifest gains the four static declarations in registry order with the design's titles; the README's command table documents the three commands, the programmatic argument form, the per-spec variants and the no-vocabulary behaviour
  - Explicit integration touch: verify the plugin-core design document's Allowed Dependencies line already covers the engine's vocabulary module (amended during cross-spec review: each feature module declares its own kiro-core imports in its design)
  - Observable: the existing manifest consistency test passes with the static declarations in canonical module-table order and the existing activation test registers exactly those
  - _Boundary: PluginIntegration_
  - _Requirements: 7.7, 11.6_

- [ ] 4.4 Write the plugin end-to-end tests for the vocabulary commands
  - Simulated host serving the fixture workspace tree: term found via label and via alternative label with the match-kind line, not-in-topic, ambiguous across two declared topics, unknown, empty query, no topics, deprecated with replacement, and the `Vocabulary findings: 1 (run Kiro: Vocabulary Topics)` line from `VOCAB_LABEL_CROSS_TOPIC` in `tree/.kiro/vocabulary.toml`; list rows and order, topic filter, undeclared topic warning, insert producing one insertion event; topics command lines and findings (including `VOCAB_LABEL_CROSS_TOPIC`); missing and invalid vocabulary across all three commands completing successfully with the expected levels; unresolved spec error identical to plugin-core's; the vocabulary path read at most once per command; after any vocabulary command the three per-spec vocabulary variants are registered for each known spec within the budget
  - Observable: the test file passes natively alongside the existing dispatch and manifest tests
  - _Boundary: PluginVocabTests_
  - _Depends: 1.2, 4.3_
  - _Requirements: 2.15, 8.3, 8.5, 11.6_

- [ ] 5. Validation: both targets, lints and the host check
  - Prerequisite: the dev-plugin installation of plugin-core (junction, trust, Reload Plugins) is in place in the open project
  - Engine and plugin build for the sandbox target with the TOML parser present; clippy with warnings denied and fmt check pass for the root workspace and the plugin package
  - Manual host check as a dev plugin: with this spec's requirements document open, the topics command reports `sdd` declared, the list command shows the seed table, and the term command without an argument shows the usage line plus the table (when no GUI host is available, the manual check is recorded as deferred in the completion note — matching `spec-navigation` 4.3 and `spec-visualization` 4)
  - Append a "Vocabulary commands: verification" section to the plugin README recording the component size before (baseline from 1.1) and after this spec and the host check outcome (or deferred note when no host is running)
  - Observable: all build and lint commands above succeed and the README section exists with both figures and the recorded outcome
  - _Boundary: PluginIntegration_
  - _Requirements: 11.1, 11.3, 11.6_
