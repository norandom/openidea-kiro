# Requirements Document

## Introduction

vocabulary-grounding gives kiro / cc-sdd specs an explicit, topic-scoped vocabulary. Today the words a requirement is about ("spec", "task", "boundary", "phase") carry no agreed meaning: each feature uses them its own way and agents contradict each other without noticing. Version 1 (`spec-kit-ears-tdd`) declared a vocabulary and gated on it, but matched term labels across the whole project, so a criterion in one feature could match a term of an unrelated subject domain and later checks compared requirements that were never about the same thing.

This spec delivers the vocabulary data contract (`.kiro/vocabulary.toml`, where every term belongs to one topic), the declaration of topics by each spec (`spec.json` `topics`), the engine capability that loads and validates the vocabulary and resolves labels only inside a spec's declared topics (a hit in another topic is reported as a distinct outcome, never as a match), three read-only plugin commands that expose terms and topics inside the IDE, a seed vocabulary for the `sdd` topic, and the documentation that defines how an NVIDIA Auto Ontology export (Auto Ontology JSON or SKOS Turtle) is turned into the TOML file. The constraint checks that consume this capability, the importer executable (`kiro-cli vocab import`) and the navigation/visualization labels are sibling specs.

## Boundary Context

- **In scope**: the vocabulary file format and its validation outcomes; the `topics` declaration of a spec and its outcomes; topic-scoped resolution of labels and ids, the hierarchy (`broader`) closure and listing of visible terms and topics; the `vocab.term`, `vocab.list` and `vocab.topics` plugin commands and their degradation when the vocabulary is absent or invalid; the seed `.kiro/vocabulary.toml` for the `sdd` topic; the SKOS → TOML and Auto Ontology JSON → TOML mappings and the written procedure from an Auto Ontology export to a committed vocabulary file; the documented `spec.json` template change.
- **Out of scope**: any check that produces a failure finding about requirement text (EARS shape, declared-before-use, topic mismatch, contradiction search — spec-constraints); the importer executable and its command line (cli-gate; this spec only defines the mapping and procedure it implements); navigation and visualization commands that display preferred labels (spec-navigation, spec-visualization); editing the vocabulary or any spec file from the IDE; running, deploying or calling Auto Ontology; changes to the OpenIDEA host; enforcing that spec templates carry `topics`.
- **Adjacent expectations**: kiro-core supplies the read port, the workspace, the `spec.json` model including the declared topics, the finding shape and its catalogue, and the text renderers; plugin-core supplies command dispatch, spec resolution, argument parsing and presentation, and delivers no argument from the IDE's own UI (only programmatic callers pass one); spec-constraints resolves every term through this spec's scoped capability and never bypasses it; cli-gate implements the SKOS → TOML and Auto Ontology JSON → TOML (`--format auto-ontology-json`) importers defined here and in cli-gate; the Auto Ontology export is a file the team produces outside the IDE and commits after conversion.

## Requirements

### Requirement 1: Vocabulary File Format
**Objective:** As a team running cc-sdd, I want one committed vocabulary file whose every term belongs to a topic and carries a definition, so that a word has one agreed meaning per subject domain and the file reviews well in a pull request.

#### Acceptance Criteria
1. The vocabulary file shall be `.kiro/vocabulary.toml`, a TOML document containing an optional `schema_version` string, an optional `topics` table whose entries carry an optional `label` and `description`, and a `terms` table whose entries are keyed by a term id.
2. The vocabulary file shall identify every term by an id matching `[a-z0-9][a-z0-9-]*` that is unique across the whole file, and every topic by an id matching the same rule.
3. The vocabulary file shall give every term a `topic` (the subject domain the term belongs to), a `label` (preferred label), a `definition`, and a `kind` that is one of `entity`, `bool`, `enum` (with a `values` list) and `int` (with optional `min` and `max` bounds).
4. The vocabulary file shall allow every term optional `broader` (a list of term ids of the same topic), `alt_labels` (a list of alternative labels), `deprecated` (a flag) and `replaced_by` (a term id of the same topic).
5. The vocabulary file shall treat two labels as the same label when they are equal after trimming, collapsing internal whitespace to single spaces and lower-casing, so that spelling variants in case and spacing do not create distinct terms.
6. The vocabulary file shall allow the same label or alternative label to appear in different topics, and shall require that within one topic no two terms share a label or alternative label.

### Requirement 2: Loading and Validation
**Objective:** As a spec author, I want the engine to load the vocabulary from the project and tell me exactly what is wrong with it, so that a broken vocabulary is fixed at its line rather than silently ignored.

#### Acceptance Criteria
1. When asked for the vocabulary, the kiro-core engine shall read `.kiro/vocabulary.toml` through the project read port exactly once per workspace and shall report a status of loaded, missing or invalid together with any findings.
2. If `.kiro/vocabulary.toml` cannot be read, the kiro-core engine shall report the status missing with one finding `VOCAB_MISSING` of severity `info` and an empty vocabulary.
3. If the file is not valid TOML or does not have the structure of Requirement 1, the kiro-core engine shall report the status invalid with one finding `VOCAB_INVALID` of severity `error` that names the line of the first problem, and an empty vocabulary.
4. If `schema_version` is present and is not a supported value, the kiro-core engine shall record `VOCAB_SCHEMA_UNSUPPORTED` of severity `warning` and continue loading.
5. If a term's `definition` is empty or whitespace, the kiro-core engine shall record `TERM_NO_DEFINITION` of severity `warning` at the term's line and keep the term.
6. If a term's `kind` is not one of the four kinds, or is `enum` without a non-empty `values` list, the kiro-core engine shall record `TERM_KIND_INVALID` of severity `warning` at the term's line and keep the term with kind `entity`.
7. If two terms of the same topic share a label or alternative label, the kiro-core engine shall record `TERM_DUPLICATE_LABEL` of severity `error` at the line of the later term, naming both ids and the label, and keep both terms.
8. If a term's `broader` names an id that is not a term, the kiro-core engine shall record `TERM_BROADER_UNKNOWN` of severity `warning` at the term's line and ignore that relation.
9. If a term's `broader` names a term of a different topic, the kiro-core engine shall record `TERM_BROADER_CROSS_TOPIC` of severity `error` at the term's line and ignore that relation, so that no hierarchy path crosses a topic.
10. If a term is transitively broader than itself, the kiro-core engine shall record `TERM_BROADER_CYCLE` of severity `error` at the line of every term on the cycle and shall still terminate every hierarchy query.
11. If a term is `deprecated` without `replaced_by`, the kiro-core engine shall record `TERM_DEPRECATED_NO_REPLACEMENT` of severity `warning` at the term's line.
12. If a term's `replaced_by` names an id that is not a term, or a term of a different topic, the kiro-core engine shall record `TERM_REPLACED_BY_UNKNOWN` or `TERM_REPLACED_BY_CROSS_TOPIC` respectively, of severity `warning`, at the term's line.
13. If a topic is declared in the `topics` table but no term belongs to it, the kiro-core engine shall record `TOPIC_EMPTY` of severity `info` at the topic's line.
14. The kiro-core engine shall carry every vocabulary finding with the file `.kiro/vocabulary.toml` and, where a term or topic is concerned, the 1-based line of its table header, so that both frontends present the finding in the shared `path:line: SEVERITY CODE message` form.
15. If the same normalised label or alternative label occurs in two different topics, the kiro-core engine shall record `VOCAB_LABEL_CROSS_TOPIC` of severity `info` at the line of the later term in file order, naming both ids and their topics, so that cross-topic label collisions are visible as a lint even though scoped resolution never matches across topics (1.6 allows the same label in different topics; this finding only makes the collision visible).

### Requirement 3: Spec Topic Declaration
**Objective:** As a spec author, I want to declare which topics my spec is about, so that term lookups for my spec never leave those topics.

#### Acceptance Criteria
1. The kiro-core engine shall take a spec's declared topics from the `topics` list of its `spec.json` as exposed by the spec model, trimmed, with empty entries and duplicates removed, preserving declaration order.
2. If a spec declares no topics (absent or empty list), the kiro-core engine shall record `TOPICS_UNDECLARED` of severity `info` on the spec's `spec.json` and shall perform no term resolution for that spec.
3. If a spec declares a topic that has no term and no `topics` entry in the vocabulary, the kiro-core engine shall record `TOPIC_UNKNOWN` of severity `warning` on the spec's `spec.json` naming the topic, and shall continue with the remaining declared topics.
4. While the vocabulary status is missing or invalid, the kiro-core engine shall record no `TOPIC_UNKNOWN` finding for any spec, so that a broken vocabulary is reported once and not once per spec.
5. The vocabulary documentation shall describe the `topics` field of `spec.json` and the change to the spec initialization template that adds an empty `topics` list to new specs, without this spec enforcing the template change.

### Requirement 4: Topic-Scoped Resolution
**Objective:** As a consumer of the vocabulary (checks, navigation labels, plugin commands), I want to resolve a label or id inside a given set of topics and get an explicit answer when the word exists elsewhere, so that a match always means the same meaning and never the same string in another domain.

#### Acceptance Criteria
1. The kiro-core engine shall offer exactly one resolution operation for labels and ids, which takes the query text and the list of topics to search and returns one of the outcomes: found, not in topic, ambiguous, unknown, or unscoped.
2. When the query, after the normalization of 1.5, equals the id, the normalized label or a normalized alternative label of exactly one term whose topic is in the searched topics, the kiro-core engine shall return found with that term and which of id, label or alternative label matched.
3. When the query matches no term in the searched topics but matches one or more terms in other topics, the kiro-core engine shall return not in topic, listing the topic and term id of every such term in sorted order, and shall expose no further data of those terms.
4. When the query matches two or more terms in the searched topics, the kiro-core engine shall return ambiguous with the topic and id of every candidate in sorted order and shall not choose among them.
5. When the query matches no term in any topic, the kiro-core engine shall return unknown.
6. When the searched topic list is empty, the kiro-core engine shall return unscoped without consulting any term.
7. When a found term is deprecated, the kiro-core engine shall include the deprecated flag and the replacing term (when it exists in the searched topics) in the found outcome, so that consumers can report the replacement without a second lookup.
8. The kiro-core engine shall give the same answer for a query regardless of the order of the searched topics and shall order every list in an outcome deterministically.
9. The kiro-core engine shall offer no operation that returns a term by label or id without a topic list, so that cross-topic resolution cannot be performed through any API.

### Requirement 5: Hierarchy and Preferred Labels
**Objective:** As a consumer, I want a term's broader terms and preferred label through the same scoped capability, so that hierarchy and display names never cross topics either.

#### Acceptance Criteria
1. When asked for the broader terms of a found term, the kiro-core engine shall return the transitive closure of its `broader` relations within its topic, nearest first, each term once, excluding relations ignored by 2.8 and 2.9 and terminating in the presence of cycles.
2. When asked for the preferred label of a term id inside a topic list, the kiro-core engine shall return the term's `label` when the id resolves as found, and otherwise the same outcome the resolution operation gives for that id.
3. When asked for the terms visible to a topic list, the kiro-core engine shall return every term whose topic is in the list, ordered by topic id then term id, and nothing from other topics.
4. When asked for the available topics, the kiro-core engine shall return every topic that has a `topics` entry or at least one term, with its label when declared and its term count, ordered by topic id.

### Requirement 6: Plugin Command `vocab.term`
**Objective:** As a developer in OpenIDEA, I want to look up what a term means for the spec I am working on, so that I use the agreed meaning while writing requirements.

#### Acceptance Criteria
1. The Kiro plugin shall provide the command `vocab.term` as a spec-addressable, pass-through-argument command whose argument tokens, joined by single spaces, form the query text.
2. When `vocab.term` runs with a query and the query resolves as found in the current spec's declared topics, the Kiro plugin shall show a notification containing the term id, label, topic, kind (with enum values or int bounds when present), definition, the broader terms nearest first, the alternative labels, the deprecation state with the replacement when present, which of id, label or alternative label matched, and the spec and topics that scoped the lookup.
3. When the query resolves as not in topic, the Kiro plugin shall show a warning-level notification stating that the query names a term of another topic, listing those topics and ids, listing the spec's declared topics, and stating that cross-topic matches are never resolved.
4. When the query resolves as ambiguous or unknown, the Kiro plugin shall show a warning-level notification naming the candidates (ambiguous) or stating that no term matches in the declared topics (unknown).
5. When `vocab.term` runs without a query, the Kiro plugin shall show an info-level notification that states the argument the command expects and then lists the terms visible to the current spec as `vocab.list` would, so that the command remains useful from the IDE's own UI, which passes no argument.
6. When the current spec declares no topics, the Kiro plugin shall show an info-level notification stating that the spec declares no topics, how to declare them in `spec.json`, and the available topics, and shall resolve nothing.

### Requirement 7: Plugin Commands `vocab.list` and `vocab.topics`
**Objective:** As a developer, I want to browse the terms and topics that apply to my spec, so that I can see the agreed vocabulary without opening the TOML file.

#### Acceptance Criteria
1. The Kiro plugin shall provide the command `vocab.list` as a spec-addressable, insert-capable, pass-through-argument command whose optional first argument token names one topic to show.
2. When `vocab.list` runs, the Kiro plugin shall show the spec name and its declared topics followed by a table of the visible terms with id, label, topic, kind and a definition shortened to one line, ordered as in 5.3; when the insert flag is set the complete table shall also be inserted at the cursor.
3. When `vocab.list` runs with a topic token that is among the spec's declared topics, the Kiro plugin shall restrict the table to that topic; if the token names a topic the spec does not declare, the Kiro plugin shall show a warning-level notification stating that the topic is not declared by the spec and listing the declared topics, and shall list no term of it.
4. The Kiro plugin shall provide the command `vocab.topics` as a spec-addressable command that ignores argument tokens.
5. When `vocab.topics` runs, the Kiro plugin shall show the spec's declared topics, the available topics of the vocabulary with their term counts, the available topics the spec does not declare, any declared topic that is unknown to the vocabulary, and every vocabulary and topic finding in the shared findings text form, with the notification level derived from the findings.
6. When `vocab.list` or `vocab.topics` runs for a spec that declares no topics, the Kiro plugin shall behave as in 6.6 and, for `vocab.topics`, still list the available topics.
7. The Kiro plugin shall register `vocab.term`, `vocab.list`, `vocab.list.insert` and `vocab.topics` statically with titles starting with `Kiro:` and shall gain per-spec variants of the three base commands through plugin-core's variant mechanism.

### Requirement 8: Graceful Degradation in the Plugin
**Objective:** As a developer, I want vocabulary commands to explain a missing or broken vocabulary instead of failing the plugin, so that the rest of the plugin keeps working.

#### Acceptance Criteria
1. If `.kiro/vocabulary.toml` is absent when a `vocab.*` command runs, the Kiro plugin shall show an info-level notification stating that the project has no vocabulary file, naming the expected path and the documentation for creating one, and shall complete the command successfully.
2. If the vocabulary file is invalid when a `vocab.*` command runs, the Kiro plugin shall show an error-level notification containing the `VOCAB_INVALID` finding in the shared findings text form and shall complete the command successfully.
3. While the vocabulary has validation findings but loaded, the Kiro plugin shall still answer `vocab.term` and `vocab.list` from the loaded terms and shall mention the number of vocabulary findings with a pointer to `vocab.topics`.
4. If the current spec cannot be resolved, the Kiro plugin shall return plugin-core's unresolved-spec error unchanged.
5. The Kiro plugin shall never read the vocabulary file more than once per command and shall never write any file.

### Requirement 9: Seed Vocabulary for the `sdd` Topic
**Objective:** As a team adopting kiro, I want a ready-made vocabulary for spec-driven development itself, so that this repository's own specs are grounded from day one.

#### Acceptance Criteria
1. The repository shall contain `.kiro/vocabulary.toml` with a `topics.sdd` entry and, in topic `sdd`, at least the terms spec, requirement, acceptance criterion, EARS pattern, task, boundary, component, phase, steering, roadmap, finding, topic, term and vocabulary, each with a non-empty definition and a kind.
2. The seed vocabulary shall load with status loaded and zero findings of severity `warning` or `error`.
3. The seed vocabulary shall express at least one `broader` relation and at least one alternative label, so that hierarchy and alternative-label resolution are exercised by real data.
4. The seed vocabulary shall carry a header comment naming its origin (the SDD ontology exported from Auto Ontology and converted by the documented procedure, or hand-authored in the same shape pending that export) and the schema version.

### Requirement 10: Auto Ontology Export and SKOS → TOML Mapping
**Objective:** As a vocabulary maintainer, I want a defined path from an ontology maintained in Auto Ontology to the committed TOML file, so that the vocabulary has a single source of truth outside the IDE and the importer (cli-gate) has an unambiguous contract.

#### Acceptance Criteria
1. The vocabulary documentation shall describe how to obtain an Auto Ontology JSON export (`GET /api/terms`) or SKOS Turtle file for a topic from Auto Ontology and convert it into `.kiro/vocabulary.toml` via `kiro-cli vocab import --format auto-ontology-json --topic <t>` or `kiro-cli vocab import --topic <t>` (supplied by cli-gate), stating which steps are verified against Auto Ontology's documentation and which require the maintainer to confirm against the installed version.
2. The SKOS → TOML mapping shall map each `skos:Concept` to one term whose id is the local name of the concept IRI converted to the id rule of 1.2 (CamelCase split into hyphenated lower-case words), `skos:prefLabel` to `label`, `skos:definition` to `definition`, each `skos:altLabel` to an entry of `alt_labels`, `skos:broader` (and the inverse of `skos:narrower`) to `broader`, `owl:deprecated true` to `deprecated`, and `dct:isReplacedBy` to `replaced_by`.
3. The SKOS → TOML mapping shall take a term's `topic` from a `kiro:topic` literal on the concept when present, otherwise from the topic the importer is invoked for, and shall take `kind`, enum values and int bounds from `kiro:kind`, `kiro:value`, `kiro:min` and `kiro:max` literals, defaulting `kind` to `entity`.
4. The SKOS → TOML mapping shall ignore subjects that are not `skos:Concept`, shall emit an empty `definition` for a concept without one (so that loading reports `TERM_NO_DEFINITION`), and shall emit terms ordered by id so that repeated imports produce identical files.
5. The vocabulary documentation shall state the `kiro:` namespace IRI used for the annotations of 10.3 and that it is a private extension ignored by SKOS tools.
6. The vocabulary documentation shall describe reviewing and committing the generated file, including that the loader's findings (Requirement 2) are the acceptance check for an import.

### Requirement 11: Engineering Quality
**Objective:** As a maintainer, I want the vocabulary capability to meet the engine's and plugin's standards, so that it runs identically in the sandbox and in CI.

#### Acceptance Criteria
1. The kiro-core engine shall accept any text as a vocabulary file without panicking and shall build for both the native and the sandbox target.
2. The kiro-core engine shall load and index a vocabulary of 10 000 terms with a 10-deep hierarchy in under one second in a native release build.
3. The kiro-core engine shall produce byte-identical findings and outcomes for the same vocabulary and queries across runs and targets.
4. The kiro-core engine shall register every finding code of Requirements 2 and 3 in its finding catalogue with the stated default severity and a description, so that the catalogue test and the crate documentation stay complete.
5. The kiro-core engine shall provide unit tests over vocabulary fixtures covering every finding code, every resolution outcome, the hierarchy closure with a cycle, and the seed vocabulary.
6. The Kiro plugin shall provide native end-to-end tests that run the three `vocab.*` commands against a simulated host with a fixture project, covering found, not in topic, ambiguous, unknown, no topics, missing vocabulary and invalid vocabulary, and a manifest consistency check that includes the vocabulary commands.
