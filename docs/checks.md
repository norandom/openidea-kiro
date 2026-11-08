# Constraints and Spec Checks Guide

This document describes the specification checking engine (`kiro_core::checks`), the acceptance criterion shape rules (EARS), the topic-scoped term declaration conventions, the guard and effect language, the bounded contradiction search, and the corresponding OpenIDEA plugin commands.

---

## 1. Overview and Motivation

In the Kiro spec-driven development methodology, specifications are executable contracts: they are formally checked, not just written and forgotten. Version 1 of the constraints checker (`ears-sdd`) checked EARS shape and searched for contradictions textually across the entire workspace, leading to false positives when unrelated domain concepts used identical words.

Version 2 introduces:
- **Strict Topic Scoping**: Terms in acceptance criteria and guards resolve exclusively within the topics declared in `spec.json`.
- **Structured Annotation Conventions**: Lightweight annotations (`_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_`) directly in Markdown requirements documents.
- **Typed Guard Language**: A small, refusable expression language over booleans, enums, and integers.
- **Bounded Contradiction Search**: An exact decision-diagram (BDD) engine that provides concrete counterexample witness states when criteria conflict, and explicitly refuses when problem size exceeds caps rather than guessing.
- **Non-Breaking Adoption**: Documents without annotations never fail term or contradiction checks; `CHECK_COVERAGE` distinguishes between "checked and clean" versus "not annotated / skipped".

---

## 2. Annotation Conventions and Placement

Criteria in `requirements.md` can be enriched with four italicized Markdown annotations.

### Annotation Placement Rules
1. **`_Terms: ..._`**
   - **Requirement Level**: Placed in the requirement description block before `#### Acceptance Criteria`. Scope applies to all criteria within that requirement.
   - **Criterion Level**: Placed indented directly under the criterion text before any blank line. Adds terms specific to that single criterion.
2. **`_Guard: ..._`**
   - Must be indented directly under the criterion text before any blank line.
   - Defines the precondition or applicability state under which the criterion must hold.
   - Only one `_Guard:_` is allowed per criterion. Placing a guard at requirement level or specifying duplicate guards produces a `GUARD_UNPARSED` error.
3. **`_Effect: ..._`**
   - Must be indented directly under the criterion text before any blank line.
   - Comma-separated list of assignments `term = literal` specifying the required state or action.
4. **`_Conflicts: ..._`**
   - Must be indented directly under the criterion text before any blank line.
   - Comma-separated list of criterion IDs (e.g. `1.2, 2.1`) in the same document whose responses are mutually incompatible.

```markdown
### Requirement 1: Telemetry and mode control

**Objective:** The controller tracks telemetry and switches modes.

_Terms: mode, depth, armed_

#### Acceptance Criteria

1. The controller shall record all telemetry readings.
   _Guard: mode = normal_
   _Effect: armed = false_

2. When the on-call engineer signals, the controller shall switch to safe mode.
   _Terms: operator_
   _Guard: mode = normal_
   _Effect: armed = false_
```

---

## 3. Guard and Effect Language

The guard language is deliberately small, deterministic, and bounded to prevent ambiguities and ensure fast evaluation in both native CI gates and WebAssembly sandboxes.

### Grammar
```text
Expr       ::= Disj
Disj       ::= Conj ("or" Conj)*
Conj       ::= Neg ("and" Neg)*
Neg        ::= "not" Neg | Atom
Atom       ::= "(" Expr ")" | TermRef | Comparison
Comparison ::= TermRef Op Literal
Op         ::= "=" | "==" | "!=" | "<" | "<=" | ">" | ">="
Literal    ::= Integer | Boolean | Word
```
- **Operator precedence**: `not` binds tightest, followed by `and`, followed by `or`. Parentheses override precedence.
- **Maximum recursion depth**: 64 nested levels.
- **Term-to-term comparisons**: Strictly disallowed (`GUARD_UNPARSED`). Comparisons must always be between a declared term and a literal constant.
- **Effect syntax**: `_Effect: term = literal, term2 = literal2_`. Only `=` is permitted; comparison operators are disallowed.

---

## 4. Term Scoping and Type Rules

### Term Reference Resolution
A term referenced in a guard or effect is matched against declared terms in `_Terms:_` (on the requirement, followed by the criterion):
1. Case-insensitive equality with the resolved term ID (e.g. `mode`).
2. Case-insensitive equality with the normalized label with internal spaces replaced by hyphens (e.g. `legacy-mode` matching `Legacy mode`).

Term resolution resolves exclusively within `spec.json.topics`. References to terms outside declared topics trigger `TERM_TOPIC_MISMATCH` or `GUARD_TERM_UNDECLARED`.

### Kind and Literal Compatibility
| Term Kind | Allowed Literals / Forms | Allowed Operators | Notes |
|-----------|-------------------------|-------------------|-------|
| `bool` | `true`, `false`, or bare `term` / `not term` | `=`, `==`, `!=` | Bare `term` evaluates to `term = true` |
| `enum` | Quoted or bare words matching declared `values` | `=`, `==`, `!=` | Value must be in term definition |
| `int` | Integer literals (`-9223372036854775808..9223372036854775807`) | `=`, `==`, `!=`, `<`, `<=`, `>`, `>=` | Subject to integer region finitisation |
| `entity` | None | None | Cannot appear in guards or effects (`GUARD_KIND_MISMATCH`) |

---

## 5. Integer Regions and Domain Finitisation

To evaluate integer guards with finite decision diagrams, the engine partitions the integer domain into contiguous intervals (regions) defined by all constants appearing in guards or effects for that term across the document, bounded by any `min` and `max` declared in `.kiro/vocabulary.toml`.

For example, for an integer term `depth` with `min = 0` and constants `500` and `1000` appearing in criteria:
- Below first constant: `0..=499`
- Point 1: `500`
- Gap 1: `501..=999`
- Point 2: `1000`
- Above last constant: `1001..`

Every point and interval is mapped to a single one-hot boolean variable in the decision diagram.

---

## 6. Contradiction Search and Witness Generation

### Incompatibility Rule
Two criteria $C_1$ and $C_2$ are considered an incompatible candidate pair if and only if:
1. **Declared Conflict**: Either criterion lists the other in `_Conflicts:_` (e.g. `_Conflicts: 1.2_`), OR
2. **Effect Clash**: Both criteria assign conflicting literals to the same vocabulary term in their `_Effect:_` annotations.

### Satisfiability and Witness States
For every candidate pair $(C_1, C_2)$:
- If $\text{Guard}(C_1) \land \text{Guard}(C_2)$ is satisfiable under the domain constraints, a `CONTRADICTION` error is emitted on the earlier criterion.
- The finding message includes the exact witness state assignment, listing values in `(topic, id)` order.
- Unconditional criteria apply in every state. If neither criterion specifies a guard, the witness is reported as `"in every state"`.

### Search Caps and Refusal
The search is bounded by two configurable caps to prevent exponential memory consumption:
- **`max_variables`** (default: `512`): Maximum total one-hot domain variables for a pair.
- **`max_nodes`** (default: `100,000`): Maximum intermediate BDD nodes allocated during boolean operations.

When a cap is exceeded, the search refuses for the affected pair and emits `SEARCH_REFUSED` (`warning`). A refused search is never reported as free of contradictions.

#### Levers to Narrow Search
1. Reduce the number of terms referenced in a single guard.
2. Reduce the number of distinct integer cut-points across the feature.
3. Split broad requirements documents into separate, focused specifications.
4. If necessary, raise `max_variables` or `max_nodes` in project configuration.

### Documented Blind Spot
The contradiction search considers only criteria within the same requirements document whose terms resolved inside the specification's declared topics. Inter-spec contradictions and cross-topic contradictions are an intentional blind spot to maintain modularity and prevent combinatorial explosion.

---

## 7. Worked Example: Effect Clash with Witness

Consider the following excerpt from a submersible controller specification:

```markdown
### Requirement 3: Safety braking

_Terms: mode, depth, armed_

#### Acceptance Criteria

1. While mode is safe and depth exceeds 1000, the controller shall arm the brake.
   _Guard: mode = safe and depth > 1000_
   _Effect: armed = true_

2. While depth is at least 500, the controller shall keep the brake released.
   _Guard: depth >= 500_
   _Effect: armed = false_
```

### Analysis:
1. **Candidate Pair**: Criterion 3.1 and 3.2 both define effects on term `armed` (`true` vs `false`), making them mutually incompatible.
2. **Preconditions**:
   - $\text{Guard}(3.1) = (\text{mode} = \text{safe}) \land (\text{depth} \ge 1001)$
   - $\text{Guard}(3.2) = (\text{depth} \ge 500)$
3. **Joint Conjunction**: Both guards simultaneously hold when $\text{mode} = \text{safe}$ and $\text{depth} \ge 1001$.
4. **Reported Finding**:
```text
.kiro/specs/sub/requirements.md:7: ERROR CONTRADICTION criteria 3.1 and 3.2 can both apply (effect armed: true vs false); witness: depth >= 1001, mode = safe
```

---

## 8. Catalog of Check Finding Codes

| Code | Severity | Description |
|------|----------|-------------|
| `EARS_NO_SHALL` | `error` | Criterion has no `shall` |
| `EARS_MULTI_SHALL` | `warning` | Criterion has more than one `shall` ("found N") |
| `EARS_MODAL` | `warning` | Competing modal outside literals and hyphenated compounds ("use `shall`, not …") |
| `EARS_NO_TRIGGER` | `warning` | Condition keyword without a comma before `shall` |
| `EARS_UNPARSED` | `warning` | Missing subject or response around the single `shall` |
| `EARS_PATTERN` | `info` | Recognised pattern ("pattern: <name>"), only when enabled |
| `TERM_UNDECLARED` | `error` | Declared term unknown in the declared topics |
| `TERM_TOPIC_MISMATCH` | `error` | Declared term exists only in undeclared topics; never a match |
| `TERM_AMBIGUOUS` | `warning` | Declared term matches several terms; use the id |
| `TERM_DEPRECATED` | `warning` | Declared term is deprecated |
| `TERM_SUGGESTED` | `info` | A visible label occurs in criteria but is not declared |
| `CHECK_UNSCOPED` | `warning` | Annotations present but the spec has no topics or the vocabulary is unavailable |
| `GUARD_UNPARSED` | `error` | Guard/effect text outside the grammar, term-vs-term comparison, duplicate or misplaced annotation |
| `GUARD_TERM_UNDECLARED` | `error` | Guard/effect references a term not declared in `_Terms:_` |
| `GUARD_KIND_MISMATCH` | `error` | Term kind incompatible with the literal or operator |
| `GUARD_UNSATISFIABLE` | `warning` | A guard holds in no state |
| `CONFLICT_REF_UNKNOWN` | `warning` | `_Conflicts:_` names no criterion |
| `CONTRADICTION` | `error` | Two incompatible criteria can both apply; witness given |
| `SEARCH_REFUSED` | `warning` | A cap was exceeded; the listed pairs were not checked |
| `CHECK_COVERAGE` | `info` | "{criteria} criteria, {annotated} annotated, {pairs} pairs considered, {contradictions} contradictions, {refused} refused" (line: None) |

---

## 9. Non-Breaking Adoption ("No Annotations, No Failures")

To allow smooth adoption across existing repositories:
- Requirements documents that contain no annotations produce **zero warnings and zero errors** from the term and contradiction checkers.
- EARS checking continues to validate natural language structure independently.
- When the term or contradiction check runs, the report includes a single `CHECK_COVERAGE` info finding indicating the coverage statistics. A document with `0 annotated, 0 pairs considered` makes it immediately evident that formal constraint checking was bypassed.

---

## 10. Proposed Template Update (For Opt-In Adoption)

The following commented hints illustrate how requirements templates may present constraint annotations to spec authors. This proposal is non-binding and does not modify baseline templates or skills:

```markdown
### Requirement N: <Title>

**Objective:** <Business or system goal>

<!-- Optional: Declare vocabulary terms scoped to this requirement -->
<!-- _Terms: term-id-1, term-id-2_ -->

#### Acceptance Criteria

1. While <condition>, the system shall <action>.
   <!-- Optional formal guard and effect annotations: -->
   <!-- _Guard: term-id-1 = value and term-id-2 > 0_ -->
   <!-- _Effect: term-id-1 = new-value_ -->
   <!-- _Conflicts: N.M_ -->
```
