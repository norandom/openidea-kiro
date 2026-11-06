# Brief: vocabulary-grounding

## Problem

Kiro requirements drift: the same word ("spec", "task", "boundary", "phase") means different things across features, and agents quietly contradict each other. v1 (`spec-kit-ears-tdd`) declared a vocabulary and gated on it, but resolved terms by label across the whole project: an EARS criterion in one feature could match a term that belongs to a different subject domain, and downstream checks (including the BDD contradiction search) then compared requirements that were never about the same thing. v2 needs a vocabulary where every term is scoped to a topic and specs say which topics they are about — so a match means the same meaning, not the same string.

## Current State

- `kiro-core` provides the spec model with `spec.json` metadata and criteria text with line spans.
- `ears-sdd`'s `vocabulary.rs` is prior art for the TOML shape (`label`, `definition`, `broader`, `alt_labels`, `deprecated`, `replaced_by`, `normalized_label`) — its `domain` field is a *value kind* (`entity|bool|enum|int`), not a subject domain; its `skos.rs` round-trips that TOML to SKOS Turtle with an `ears:` namespace.
- NVIDIA Auto Ontology (`github.com/NVIDIA/auto-ontology`) builds/reconciles ontologies and exports SKOS/OWL/RDF — not reachable from the sandbox; consumable only as an exported file.
- No `.kiro/vocabulary.toml` exists yet in this project.

## Desired Outcome

- A committed `.kiro/vocabulary.toml` whose terms carry `topic` (subject domain, e.g. `sdd`), `kind` (value kind for the constraint layer), `label`, `definition`, `broader`, `alt_labels`, `deprecated`/`replaced_by`; seeded for the SDD topic (Spec, Requirement, Criterion, EARS pattern, Task, Boundary, Component, Phase, Steering, …) from an Auto Ontology export.
- `spec.json` gains `"topics": ["sdd", …]` declaring what a spec is about (template/skill update documented, not enforced by this spec).
- `kiro_core::vocab`: parse the file, build per-topic indexes (id, normalised label, alt labels), `broader` transitive closure with cycle detection, and `resolve(label, topics)` that only searches the given topics — returning `NotInTopic` (with the topic the label *does* exist in) rather than a cross-topic hit.
- Plugin commands: `vocab.term <id>` (definition, topic, kind, broader, altLabels), `vocab.list [topic]` (terms visible to the current spec), `vocab.topics` (declared vs available topics); all read-only.

## Approach

- TOML-first (`toml` crate, wasm-safe); SKOS Turtle import lives in cli-gate (`kiro-cli vocab import <export.ttl>`), mapping SKOS `prefLabel/altLabel/broader/definition` + a `kiro:topic`/`kiro:kind` annotation into the TOML. The plugin never parses RDF.
- Topic scoping is a hard rule in core: every resolution API takes the spec's declared topics; a spec with no `topics` gets an info finding `TOPICS_UNDECLARED` and *no* term resolution.
- Preferred labels are exposed for navigation/visualization through the same scoped API.

## Scope

- **In**: vocabulary file format + loader + validation findings (duplicate labels within a topic, unknown `broader`, cycles, deprecated without `replaced_by`, cross-topic label collisions as a `VOCAB_LABEL_CROSS_TOPIC` info lint — resolution never crosses topics, the lint only makes the collision visible); topic-scoped resolution; seed `.kiro/vocabulary.toml` for the SDD topic; `spec.json` `topics` field (parsing in core, docs for the template change); `vocab.*` plugin commands; docs: how to export the SDD ontology from Auto Ontology and import it with `kiro-cli`.
- **Out**: EARS-shape, declared-before-use and contradiction checks (spec-constraints); running Auto Ontology; editing the vocabulary or specs from the IDE; the SKOS importer implementation (cli-gate, this spec only defines the mapping).

## Boundary Candidates

- `kiro_core::vocab` (parse, index, scoped resolve) — pure, unit-testable.
- Plugin `vocab.*` command layer — thin presentation.
- Vocabulary file + topic convention — data contract shared with spec-constraints and cli-gate.

## Out of Boundary

- Any check that produces a failure finding (spec-constraints).
- Navigation / visualization commands (they consume preferred labels).

## Upstream / Downstream

- **Upstream**: kiro-core (model, `spec.json`); plugin-core (dispatch); Auto Ontology export as a file.
- **Downstream**: spec-constraints (all checks resolve terms through the scoped API); spec-navigation / spec-visualization (labels); cli-gate (importer implements this spec's SKOS → TOML mapping).

## Existing Spec Touchpoints

- **Extends**: none. **Adjacent**: spec-constraints (consumer), cli-gate (importer), kiro-core (`spec.json` field).

## Constraints

- No network; the vocabulary must already exist as a readable project file; the plugin never calls Auto Ontology.
- `read-file` ≤ 16 MiB; loading + indexing must fit the 5 s budget; deterministic ordering.
- Topic scoping may not be bypassed by any API; cross-topic lookups are a distinct result, never a silent match.
- If no vocabulary file exists, commands degrade gracefully (info notification) rather than failing the plugin.
