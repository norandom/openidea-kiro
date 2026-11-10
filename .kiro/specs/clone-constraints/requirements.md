# Requirements Document

## Introduction

Clone Constraints. Owns duplicate blocks, similar-routine families, clone-source adapters and clone quality/performance acceptance. Consumes measurement, coverage, ownership and report contracts; does not own metric baselines or semantic-equivalence claims.

## Scope and Adjacent Expectations

Consumes: code-constraints. The owner authorized the reviewed amendments and progression to design on 2026-11-09. Each spec delivers and accepts its own behavior; original criterion identities are preserved in [the split map](../code-constraints/split-map.md).

## Requirements

### Requirement 1: Duplicate Blocks and Similar-Routine Families
**Objective:** As a reviewer, I want a copy or a renamed copy of code the project already holds reported as one finding naming the original, so that an agent that writes the same routine again is told where it already exists.

#### Acceptance Criteria
1. The Code Gate shall report `CODE_DUPLICATE_BLOCK` for a run of at least 12 consecutive code lines in a selected file that the project holds at another location, where a code line is the file's lexemes with whitespace and comments removed and each line is compared by hash, naming up to three other locations; two copies in one file count, both ends are reported when both are selected, and a run that repeats only itself is not reported.
2. The Clone Gate shall compare routines of at least 6 statements using identifier/literal-normalized token shapes and a deterministic similarity ratio from an adopted open-source library, defined as twice the matched length divided by the total length, reporting connected families whose retained edges score at least 0.9.
3. When a family contains a selected routine, the Code Gate shall report one `CODE_SIMILAR_FAMILY` finding for the family naming the other members with their locations, the family size and the weakest similarity holding it together, with a `shrink:` hint when every member is in one file and a `delete:` hint when the family spans files.
4. The Clone Gate shall exclude ignored names, cross-language pairs and routines lacking statement counts, remove members nested in another retained member and recompute family reachability after removal so an excluded routine cannot bridge a family.
5. The Code Gate shall refuse a pair before scoring it when the length band or the token multiset makes the threshold unreachable, so that the pass is exact and no family is lost, with both bounds documented as exact ceilings rather than approximations.
6. The Code Gate shall document that the family rule finds a routine that was copied and renamed and does not find a routine that was re-implemented in another shape (stating the scores measured on the labeled corpus for a rename, for guard clauses rewritten as a ladder and for a table-driven rewrite), and that an embedding-based clone finder is a periodic report a human or an agent reads and is never a reason a change is refused.
7. When a jscpd report is supplied, the Clone Gate shall import exact or normalized block clones as CODE_DUPLICATE_BLOCK with source identity, preserve other clone kinds separately and deduplicate only equivalent two-ended ranges.

### Requirement 2: Clone Sources and Comparable Semantics
**Objective:** As an operator, I want external clone reports integrated without confusing different detection methods.

#### Acceptance Criteria
1. The Clone Gate shall accept pinned jscpd JSON report variants, preserve producer identity, clone kind and ranges, report invalid optional sources and fail on absent or invalid required sources.
2. The Clone Gate shall keep AST-similarity and token-shape similarity results explicitly distinct and never apply one detector's threshold to another detector's score.
3. The Clone Gate shall merge only equivalent clone pairs with both locations and compatible detector semantics, retaining overlapping but different clones and source provenance.
4. The Clone Gate shall report unavailable or bounded-out clone analysis as incomplete coverage and never use absent clone records as proof of no duplication.

### Requirement 3: Reuse Prototype and Incremental Acceptance
**Objective:** As the owner, I want clone implementation cost and noise controlled by executable comparisons.

#### Acceptance Criteria
1. The clone development workflow shall evaluate existing detection libraries and reports against a pinned corpus before authorizing a custom detector, recording adoption or rejection with concrete differences.
2. The family acceptance corpus shall cover asymmetric scoring order, repeated tokens, exact pruning bounds, excluded-name bridges and nested members connected through a third routine.
3. The first clone deliverable shall detect an introduced copy through the real binary and pass after its removal, with a separate imported-report journey and a required-source failure case.
4. The clone workflow shall measure accuracy and runtime against a labeled corpus, keep unsupported semantic variants advisory and require measured performance within the code-gate budget before acceptance.
