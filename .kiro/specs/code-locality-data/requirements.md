# Requirements Document

## Introduction

Code Locality and Named Data Access. Owns scattered definitions/state/schema and vocabulary-backed named data access. Consumes confined parsing, coverage, ownership and vocabulary resolution. Does not infer semantic equivalence, implement dependency graphs or extend IDE behavior.

## Scope and Adjacent Expectations

Consumes: code-constraints, vocabulary-grounding. The owner authorized the reviewed amendments and progression to design on 2026-11-09. Each spec delivers and accepts its own behavior; original criterion identities are preserved in [the split map](../code-constraints/split-map.md).

## Requirements

### Requirement 1: Scattered Definitions
**Objective:** As a maintainer, I want a constant that was copied instead of shared reported, so that changing a policy does not mean finding every file that hardcodes it.

#### Acceptance Criteria
1. The Code Gate shall collect every module-level binding of the project (constants, statics and module-scope assignments) with its name and the text of its initialiser, skipping a binding whose initialiser does not close within 12 lines or cannot be read, so that unreadable initialisers are never grouped.
2. When a name is bound to the same initialiser text in more files than `max_files` (default 3), the Code Gate shall report `CODE_SCATTERED_DEFINITION` against each such file in the selection naming the binding, its text and up to three other files, with a `collect:` hint (declare it once and import it; when the copies are not one decision, add the name to the ignore list with a reason).
3. The Code Gate shall key on the name and the value together, so that a name bound to a different value in each file is not reported.
4. The Code Gate shall accept an ignore list of binding names for the per-module idiom, shipped with the documented names that are written out in every module on purpose (`log`, `logger`, `pytestmark`, `__all__`) and documented as a list of names rather than values.
5. When one name is bound to two or more different values and each value stands in at least `max_files` files, the Code Gate shall report the groups separately and add one info finding naming the name and its values, so that one name with two meanings stays visible instead of being unified by accident.
6. The Code Gate shall document that comparison is textual, that a value referencing another module-level name can over-report, and that semantic equality is not inferred.

### Requirement 2: Scattered State
**Objective:** As a reviewer, I want a set of states that is tested in many files reported as one finding, so that a state machine spread across the code base gets one owner and one transition table.

#### Acceptance Criteria
1. The Code Gate shall collect, project-wide, every comparison of a subject against a string or integer literal (equality and inequality tests, match arms over literals, membership tests over literal collections), grouped by the subject's name as written (a variable, field or accessor such as `status`, `self.state`, `job.phase`) and the set of literal values it is compared with.
2. When a subject is compared with at least `min_states` (default 2) distinct literal values and those comparisons stand in more files than `max_files` (default 3), the Code Gate shall report `CODE_SCATTERED_STATE` against each such file in the selection naming the subject, the values, the number of files and up to three other files, with a `collect:` hint (one enumeration type and one owner — a transition function or table — so comparisons become calls or exhaustive matches).
3. Where an enumeration declaration and all compared variant paths can be resolved without ambiguity within the supported syntactic profile, the Locality Gate shall report exhaustive matching of that enumeration in more than max_files files at information level, otherwise recording the enum analysis as unavailable.
4. The Code Gate shall report each subject at most once per file, never for a subject whose comparisons all stand in one file and never for a subject on the configured ignore list of subject names with reasons.
5. The Code Gate shall document what the rule cannot see: a state carried in booleans, a state tested through a helper predicate, two unrelated subjects that share a name, and literal values compared through a variable.

### Requirement 3: Role-Named Data Access
**Objective:** As a reviewer of data code, I want columns, fields and axes of arrays and dataframes addressed by a role name grounded in the project vocabulary rather than by position, so that a reader knows what a column means without the notebook that produced it.

#### Acceptance Criteria
1. The Code Gate shall report `CODE_POSITIONAL_ACCESS` as a warning when a tabular receiver is indexed by an integer literal in a column, field or axis position according to the shipped per-language patterns (for Python: dataframe positional column selection, the column axis of a two-dimensional array, element access on a row record; for Rust: positional column selection on dataframes, the column axis of two-dimensional arrays), with a `name:` hint (select the column by its role name, or give the row a typed record).
2. Where the configuration names one or more vocabulary topics as data topics, the Code Gate shall resolve every column or field name used in data code (string literals and attribute names in column-selection positions, field names of row records) against those topics through the vocabulary's scoped resolution, reporting `CODE_COLUMN_UNGROUNDED` as a warning for a name that resolves to no term with the nearest terms as suggestions.
3. While no data topic is configured, the Code Gate shall run only the positional check, stating in the coverage finding that column names were not grounded so that a data file is never reported clean without saying so.
4. When the same column-name literal is used in more files than `max_files`, the Code Gate shall report `CODE_SCATTERED_SCHEMA` at info level against each such file naming the literal and the files, with a `collect:` hint (one schema or constants module, or a typed row record, owned by one design component).
5. The Code Gate shall ship the receiver and position patterns for Python (pandas, polars, NumPy) and Rust (polars, ndarray), documented, and accept additional patterns per language from the configuration with a reason.
6. The Code Gate shall document what the rule cannot see: a position computed from a variable, a name built at runtime, access through a helper, and receivers the patterns do not name.

### Requirement 4: Noise Control and Independent Acceptance
**Objective:** As a maintainer, I want heuristic findings calibrated without treating names as proof of shared meaning.

#### Acceptance Criteria
1. The Locality Gate shall ship new locality and data-access rules as warning or information until a labeled positive-and-negative corpus and representative-repository review justify stronger policy.
2. The Locality Gate shall distinguish supported, unsupported, ambiguous and failed extraction, report skipped language or receiver coverage and avoid claiming type-resolved knowledge from a textual match.
3. The locality acceptance suite shall exercise same-name unrelated subjects, ignored per-module idioms, single-owner state, legitimate positional indexing, computed-access blind spots and missing or ambiguous vocabulary.
4. The first locality deliverable shall show a real-binary scattered-definition finding removed by consolidation, followed by independent state and named-data journeys using the shared evidence workflow.
