# Project Vocabulary Guide

This document describes the project vocabulary specification format, ID and labeling conventions, spec-level topic scoping, plugin integration behavior, and the import procedure from external ontologies (including Auto Ontology and SKOS Turtle).

---

## 1. File Format and Structure

Project vocabularies are stored in `.kiro/vocabulary.toml` at the root of the workspace.

### Schema Header
The file begins with the schema version string:
```toml
schema_version = "2"
```
The engine requires `schema_version = "2"`. Files without a schema version or with unsupported versions trigger a `VOCAB_SCHEMA_UNSUPPORTED` finding.

### Topic Definitions (`[topics.<id>]`)
Topics define domains or subject-matter boundaries. Each topic is declared as a table:
```toml
[topics.sdd]
label = "Spec-driven development"
description = "Artifacts and roles of the kiro / cc-sdd workflow."
```
- `label`: Human-readable display label (optional, defaults to `<id>`).
- `description`: Optional documentation summary.
- If terms reference a topic that has no `[topics.<id>]` entry, the engine creates an implicit topic. If a declared topic has no terms, the engine reports a `TOPIC_EMPTY` finding.

### Term Definitions (`[terms.<id>]`)
Terms define domain entities, types, enumerations, or measurements:
```toml
[terms.acceptance-criterion]
topic = "sdd"
label = "Acceptance criterion"
kind = "entity"
definition = "One EARS-shaped, testable statement under a requirement, identified as N.M."
broader = ["requirement"]
alt_labels = ["criterion", "AC"]

[terms.phase]
topic = "sdd"
label = "Phase"
kind = "enum"
values = ["initialized", "requirements-generated", "design-generated", "tasks-generated"]
definition = "The workflow state recorded in spec.json."

[terms.old-task]
topic = "sdd"
label = "Work item"
kind = "entity"
definition = "Former name of a task."
deprecated = true
replaced_by = "task"
```

### Mandatory Fields and Tables
- The `[terms]` table is mandatory. An empty vocabulary writes an empty `[terms]` section. An absent `[terms]` table reports `VOCAB_INVALID`.
- For each term:
  - `topic`: Required topic identifier.
  - `label`: Required human-readable label.
  - `definition`: Required non-empty string. An absent or empty definition triggers `TERM_NO_DEFINITION`.

---

## 2. ID and Label Conventions

### Global ID Grammar
Term and topic identifiers share the identical machine-readable grammar:
- Must match `^[a-z0-9][a-z0-9-]*$`.
- Length: 1 to 64 ASCII lowercase alphanumeric characters and hyphens.
- Must start with an ASCII lowercase letter (`[a-z]`) and end with an alphanumeric character (`[a-z0-9]`).
- Violations produce a `VOCAB_INVALID` error naming the invalid ID.

### Uniqueness and Cross-Topic Namespacing
- **IDs are globally unique across the entire file**. Two terms cannot share the same ID even if they belong to different topics.
- **Convention**: When the same domain concept exists across multiple topics (e.g., "task" in `sdd` vs. "task" in `pm`), prefix the ID with the topic name: `<topic>-<name>` (for example, `task` in `sdd` and `pm-task` in `pm`).
- **Labels are per-topic**: Two terms in different topics MAY share the same label (e.g., both labeled `"Task"`). In that case, the engine reports an informational lint finding `VOCAB_LABEL_CROSS_TOPIC` at the later term in file order. Within the same topic, identical normalized labels report `TERM_DUPLICATE_LABEL`.

### Label Normalization
Queries, labels, and alternative labels are normalized prior to matching:
1. Trim leading and trailing whitespace.
2. Collapse internal whitespace sequences (spaces, tabs, newlines) into a single space.
3. Convert characters to Unicode lowercase.

---

## 3. Term Kinds and Facets

The `kind` field specifies the structural category of a term:
- `entity`: Concept or object (default if omitted or invalid).
- `bool`: Boolean flag or predicate.
- `enum`: Enumerated value set. Requires a non-empty `values` array of strings (e.g., `values = ["alpha", "beta"]`).
- `int`: Integer measurement or count. Optional `min` and `max` integers define bounds. If `min > max`, `TERM_KIND_INVALID` is reported and kind defaults to `entity`.

---

## 4. Hierarchy and Deprecation

### Broader Relations (`broader`)
- A term may declare broader parent terms via `broader = ["<parent-id>", ...]`.
- Parents must belong to the **same topic**. Cross-topic parent references report `TERM_BROADER_CROSS_TOPIC` and are pruned from hierarchy queries. Unknown parent IDs report `TERM_BROADER_UNKNOWN`.
- Cycles (e.g. $A \to B \to A$ or self-loops $A \to A$) are detected and report `TERM_BROADER_CYCLE`. The start term is removed from its own ancestor closure, ensuring searches terminate deterministically.

### Deprecation and Replacement (`deprecated`, `replaced_by`)
- Setting `deprecated = true` marks the term as deprecated.
- If `deprecated = true` and `replaced_by` is absent, the engine reports `TERM_DEPRECATED_NO_REPLACEMENT`.
- `replaced_by` must reference an existing term in the **same topic**. Cross-topic replacements report `TERM_REPLACED_BY_CROSS_TOPIC`; unknown replacement IDs report `TERM_REPLACED_BY_UNKNOWN`.

---

## 5. Spec Topic Scoping (`spec.json`)

To prevent terms from different domains colliding or bleeding across specifications, every specification scopes its vocabulary lookups via `topics` in its `spec.json`:

```json
{
  "feature": "vocabulary-grounding",
  "phase": "tasks-generated",
  "topics": ["sdd"]
}
```

### Scoping Rules
1. `topics` is an array of strings. Empty strings and duplicate entries are removed while preserving declaration order.
2. **Undeclared Topics**: If `topics` is absent or empty, the engine reports `TOPICS_UNDECLARED` (severity `info`) on `spec.json`. Vocabulary resolution for that spec is disabled (`Lookup::Unscoped`).
3. **Unknown Topics**: If a declared topic does not exist in `.kiro/vocabulary.toml`, the engine reports `TOPIC_UNKNOWN` (severity `warning`) on `spec.json`.
4. **Error Suppression**: While `.kiro/vocabulary.toml` is missing or invalid, `TOPIC_UNKNOWN` findings are suppressed so a broken vocabulary file is reported once rather than per specification.
5. **Template Initialisation**: New specifications created by `/kiro-spec-init` or `/kiro-spec-quick` initialize with `"topics": []` to prompt authors to declare domain topics.

---

## 6. Plugin Commands & Degradation Behavior

The Kiro IDE plugin contributes three vocabulary commands:

| Command | Title | Addressable | Output |
|---------|-------|-------------|--------|
| `vocab.term` | `Kiro: Vocabulary Term` | Yes | Definition, kind, broader closure, replacement |
| `vocab.list` | `Kiro: List Vocabulary Terms` | Yes | Table of visible terms (supports `--insert`) |
| `vocab.topics` | `Kiro: Vocabulary Topics` | Yes | Declared and available topics, counts, findings |

### Degradation Order
When commands are executed, the plugin evaluates workspace and vocabulary status in this exact sequence:
1. **Unresolved Spec**: If `--spec` names a nonexistent feature, returns error without reading the vocabulary.
2. **Missing Vocabulary**: If `.kiro/vocabulary.toml` is absent, returns an informational notification:
   `No vocabulary found at .kiro/vocabulary.toml. See docs/vocabulary.md for format and setup.`
3. **Invalid Vocabulary**: If `.kiro/vocabulary.toml` has syntax or structural errors, returns an error notification:
   `.kiro/vocabulary.toml:<line>: ERROR VOCAB_INVALID <message>`
4. **Undeclared Topics**: If the spec declares no topics, returns an info notification instructing the author to declare topics in `spec.json` and lists available topics.
5. **Successful Resolution**: Queries are resolved against terms visible in the declared topics.

---

## 7. Auto Ontology Export & Import Procedure

Organizations maintaining taxonomies in Auto Ontology can export and convert them into `.kiro/vocabulary.toml`.

### Procedure Separation: Verified Facts vs. Local Configuration

#### Verified Facts (Auto Ontology REST API)
- Auto Ontology provides a REST API endpoint: `GET /api/terms`.
- Authentication uses the HTTP header `x-api-key: <api-token>`.
- The endpoint returns a JSON payload (either an array or `{"terms": [...]}`) containing term objects:
  ```json
  [
    {
      "id": "insider-threat",
      "name": "Insider Threat",
      "description": "Risk posed by individuals within an organization.",
      "synonyms": ["insider risk", "internal threat"],
      "related_terms": ["data-exfiltration"],
      "zones": ["security"]
    }
  ]
  ```
- Alternatively, `POST /api/model/export` exports YAML in the `auto_ontology` format containing the same fields.

#### Local Steps to Confirm Against Installed Version
- Host URL and port of the Auto Ontology deployment (e.g., `https://ontology.internal.corp:8443`).
- API token generation and role-based access permissions.
- Mapping between Auto Ontology `zones` and Kiro `topics`.

### Importer Invocations (Supplied by `cli-gate`)

#### Direct JSON Import Path
```bash
kiro-cli vocab import export.json --format auto-ontology-json --topic sdd [--output .kiro/vocabulary.toml]
```
- Converts `name` into `label`.
- Derives ID as `<topic>-<slug(name)>` (or `<slug(name)>` if `--no-topic-prefix` is passed or the name already begins with `<topic>-`).
- Maps `description` to `definition`.
- Maps `synonyms` to sorted, deduplicated `alt_labels`.
- Sets `kind = "entity"` and `broader = []` (Auto Ontology term models do not maintain strict taxonomical hierarchies).

#### SKOS Turtle Import Path
```bash
kiro-cli vocab import vocabulary.ttl --topic sdd [--output .kiro/vocabulary.toml]
```
- Reads standard SKOS RDF Turtle files, applying the normative mapping below.

---

## 8. Normative SKOS → TOML Mapping

The following mapping rules are normative for `cli-gate` and vocabulary importers:

| SKOS / RDF Pattern | TOML Mapping | Transformation Rule |
|--------------------|--------------|---------------------|
| `?c a skos:Concept` | `[terms.<id>]` | `<id>` is the local name after `#` or the last `/`. Split CamelCase into hyphenated words (`InsiderThreat` → `insider-threat`, `HTTPSProxy` → `https-proxy`, `Level2Cache` → `level2-cache`). Replace invalid characters with `-`. Must satisfy `is_id`. |
| `skos:prefLabel` | `label` | Prefer literal without language tag, else `@en`, else first in document order. If absent, defaults to `<id>`. |
| `skos:definition` | `definition` | Same literal selection. If absent, emits `""` (the Kiro loader flags `TERM_NO_DEFINITION`). |
| `skos:altLabel` | `alt_labels` | Array of alternative labels, sorted and deduplicated. |
| `skos:broader ?p`<br>`?p skos:narrower ?c` | `broader` | Array of parent IDs, merged from both directions, sorted and deduplicated. |
| `owl:deprecated true` | `deprecated = true` | Emits boolean deprecation flag. |
| `dct:isReplacedBy ?r` | `replaced_by = "<id>"` | Target term ID of replacement. |
| `kiro:topic "<topic>"` | `topic = "<topic>"` | Overrides CLI `--topic` argument when present. |
| `kiro:kind "<kind>"` | `kind = "<kind>"` | `entity`, `bool`, `enum`, or `int` (default: `entity`). |
| `kiro:value "<v>"` | `values = [...]` | Populates `values` array for `enum` kind in document order. |
| `kiro:min <n>` / `kiro:max <m>` | `min = <n>`, `max = <m>` | Populates integer bounds for `int` kind. |
| Non-concepts | Ignored | `skos:ConceptScheme`, collections, and blank nodes are ignored. |
| Output ordering | Deterministic | `schema_version = "2"`, `[topics.<topic>]` with `label` from `skos:prefLabel` or `dct:title`, followed by `[terms.*]` sorted strictly by ID. |

### Private Extension Namespace
The `kiro:` RDF namespace IRI is:
```text
urn:openidea-kiro:vocab:
```
Predicates:
- `urn:openidea-kiro:vocab:topic`
- `urn:openidea-kiro:vocab:kind`
- `urn:openidea-kiro:vocab:value`
- `urn:openidea-kiro:vocab:min`
- `urn:openidea-kiro:vocab:max`

This namespace is a private extension for Kiro attributes; standard SKOS tools ignore it safely.

---

## 9. Review and Commit Procedure

When updating or importing `.kiro/vocabulary.toml`:
1. Execute validation checks:
   ```bash
   cargo test -p kiro-core --test vocab
   ```
2. Verify with the Kiro plugin command `vocab.topics` or check diagnostics in the IDE:
   - Ensure `status` is `Loaded`.
   - Ensure zero findings of severity `warning` or `error`.
   - Note any informational `VOCAB_LABEL_CROSS_TOPIC` collisions.
3. Review git diff for intended additions, removals, or deprecations.
4. Commit `.kiro/vocabulary.toml` and any corresponding `spec.json` topic updates together.
