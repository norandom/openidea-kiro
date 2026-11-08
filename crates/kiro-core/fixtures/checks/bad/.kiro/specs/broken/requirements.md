<!--
Expected findings:
Document:
- line None: INFO CHECK_COVERAGE
Requirement 1:
- line 19: ERROR TERM_TOPIC_MISMATCH shift exists only in topic(s) hr; spec declares ops, pm; cross-topic matches are never resolved
- line 19: ERROR TERM_UNDECLARED ghost is not a term in topics ops, pm
- line 19: WARNING TERM_AMBIGUOUS Level matches ops.level, pm.pm-level; use the term id
- line 19: WARNING TERM_DEPRECATED legacy-mode is deprecated; use mode
- line 15: INFO TERM_SUGGESTED Sprint (term sprint) occurs in 1.6 but is not declared in _Terms:_
Criteria:
- 1.1: ERROR EARS_NO_SHALL
- 1.2: WARNING EARS_MULTI_SHALL found 2
- 1.3: WARNING EARS_MODAL use shall, not must
- 1.4: WARNING EARS_NO_TRIGGER condition keyword without comma before shall
- 1.5: WARNING EARS_UNPARSED no subject before shall
- 1.6: (clean criterion exercising TERM_SUGGESTED)
Requirement 2:
- line 48: ERROR GUARD_UNPARSED misplaced _Guard:_ belongs indented under a criterion
Criteria:
- 2.1: ERROR GUARD_UNPARSED expected a term, a literal, or ( at position 8
- 2.2: ERROR GUARD_UNPARSED armed is a term; comparisons are against literal constants only
- 2.3: ERROR GUARD_UNPARSED second _Guard:_ on criterion
- 2.4: ERROR GUARD_TERM_UNDECLARED depth
- 2.5: ERROR GUARD_KIND_MISMATCH operator (entity) cannot be compared with a boolean
- 2.6: WARNING GUARD_UNSATISFIABLE
- 2.7: WARNING CONFLICT_REF_UNKNOWN 99.9
Requirement 3:
Criteria:
- 3.1: ERROR CONTRADICTION criteria 3.1 and 3.2 can both apply (effect armed: true vs false); witness: depth >= 1001, mode = safe
- 3.2: (clean criterion partnering with 3.1)
Requirement 4:
Criteria:
- 4.1: ERROR CONTRADICTION criteria 4.1 and 4.2 can both apply (declared conflict); witness: in every state
- 4.2: (clean unannotated criterion partnering with 4.1)
-->

# Requirements Document

## Introduction

Known-bad requirements document exercising all finding codes of spec-constraints.

## Requirements

### Requirement 1: EARS and Term checks

**Objective:** Exercise EARS flaws and term resolution findings.

_Terms: shift, ghost, Level, legacy-mode_

#### Acceptance Criteria

1. The controller records telemetry data without any modal.

2. The controller shall record telemetry and the controller shall emit a pulse.

3. The controller must verify the operating parameters.

4. When system is ready the controller shall start.

5. When ready, shall process.

6. The controller shall coordinate with the sprint.

### Requirement 2: Guard, Kind, and Conflict checks

**Objective:** Exercise guard parsing, kind mismatches, and conflict findings.

_Terms: mode, operator, armed_
_Guard: mode = safe_

#### Acceptance Criteria

1. The controller shall handle syntax errors.
   _Guard: mode ==_

2. The controller shall reject term-to-term comparisons.
   _Guard: mode = armed_

3. The controller shall reject duplicate guards.
   _Guard: mode = safe_
   _Guard: mode = normal_

4. The controller shall reject undeclared guard terms.
   _Guard: depth > 10_

5. The controller shall reject kind mismatches.
   _Guard: operator = true_

6. The controller shall warn on unsatisfiable guards.
   _Guard: mode = safe and mode = normal_

7. The controller shall warn on unknown conflict references.
   _Conflicts: 99.9_

### Requirement 3: Contradiction via conflicting effects

**Objective:** Two criteria with overlapping guards have opposing effects on term armed.

_Terms: mode, depth, armed_

#### Acceptance Criteria

1. While mode is safe and depth exceeds 1000, the controller shall arm the brake.
   _Guard: mode = safe and depth > 1000_
   _Effect: armed = true_

2. While depth is at least 500, the controller shall keep the brake released.
   _Guard: depth >= 500_
   _Effect: armed = false_

### Requirement 4: Contradiction via declared conflict

**Objective:** Two criteria declared in conflict that can both apply unconditionally.

_Terms: operator_

#### Acceptance Criteria

1. The controller shall log every state transition.
   _Conflicts: 4.2_

2. The controller shall operate silently.
