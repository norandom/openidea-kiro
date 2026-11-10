# Code-quality program: criterion migration

The user accepted the review amendments and asked for bounded designs on 2026-11-09. The active code-constraints spec is now the measured-code deliverable; this document preserves the program view. The previous requirements/design/tasks/spec metadata are archived under history/2026-11-09-monolith. **Do not execute the archived task list.**

## Ownership and deliverables

| Spec | Owns | First independently useful result |
|---|---|---|
| cli-contracts | Shared groups and compatible migration | Existing check through shared library |
| verification-evidence | Ledger, run evidence and reusable harness | Existing-binary journey and false-claim rejection |
| code-constraints | Comparable numeric measurements and safe ratchet | Task-owned fail/fix/pass and baseline regression |
| clone-constraints | Clone detection, families and jscpd import | Introduced-copy finding and repair |
| code-locality-data | Scattered policy/state/schema and named data | Scattered-definition consolidation |

## Original criteria and successors

Original IDs refer only to the archived monolith, not current code-constraints numbering. Each of the 108 original criteria has exactly one primary owner; additional references preserve composite obligations. Behavioral corrections are intentional amendments authorized by the user: sparse findings are partial, default severities require calibration, skipped tests do not pass, CLI compatibility has explicit exceptions and evidence needs current execution identity.

| Original criterion | Primary successor | Additional obligations |
|---|---|---|
| A3.1 | cli-contracts:1.1 | — |
| A3.2 | cli-contracts:1.2 | — |
| A3.3 | cli-contracts:1.3 | — |
| A3.4 | cli-contracts:1.4 | — |
| A3.5 | cli-contracts:1.5 | — |
| A3.6 | cli-contracts:1.6 | — |
| A3.7 | cli-contracts:1.7 | — |
| A1.1 | verification-evidence:1.1 | — |
| A1.2 | verification-evidence:1.2 | — |
| A1.3 | verification-evidence:1.3 | — |
| A1.4 | verification-evidence:1.4 | — |
| A1.5 | verification-evidence:1.5 | — |
| A1.6 | verification-evidence:1.6 | — |
| A1.7 | verification-evidence:1.7 | — |
| A1.8 | verification-evidence:1.8 | — |
| A1.9 | verification-evidence:1.9 | — |
| A2.1 | verification-evidence:2.1 | — |
| A2.2 | verification-evidence:2.2 | code-constraints:12.3, clone-constraints:3.3, verification-evidence:4.1 |
| A2.3 | verification-evidence:2.3 | — |
| A2.4 | verification-evidence:2.4 | — |
| A2.5 | verification-evidence:2.5 | — |
| A2.6 | verification-evidence:2.6 | — |
| 1.1 | code-constraints:1.1 | clone-constraints:1.1, code-locality-data:1.1, code-locality-data:2.1, code-locality-data:3.1 |
| 1.2 | code-constraints:1.2 | clone-constraints:2.1 |
| 1.3 | code-constraints:1.3 | — |
| 1.4 | code-constraints:1.4 | — |
| 1.5 | code-constraints:1.5 | — |
| 1.6 | code-constraints:1.6 | — |
| 1.7 | code-constraints:1.7 | — |
| 2.1 | code-constraints:2.1 | — |
| 2.2 | code-constraints:2.2 | — |
| 2.3 | code-constraints:2.3 | — |
| 2.4 | code-constraints:2.4 | — |
| 2.5 | code-constraints:2.5 | — |
| 2.6 | code-constraints:2.6 | — |
| 3.1 | code-constraints:3.1 | — |
| 3.2 | code-constraints:3.2 | — |
| 3.3 | code-constraints:3.3 | — |
| 3.4 | code-constraints:3.4 | — |
| 3.5 | code-constraints:3.5 | — |
| 3.6 | code-constraints:3.6 | — |
| 4.1 | code-constraints:4.1 | — |
| 4.2 | code-constraints:4.2 | — |
| 4.3 | code-constraints:4.3 | — |
| 4.4 | code-constraints:4.4 | — |
| 4.5 | code-constraints:4.5 | — |
| 9.1 | code-constraints:5.1 | — |
| 9.2 | code-constraints:5.2 | — |
| 9.3 | code-constraints:5.3 | — |
| 9.4 | code-constraints:5.4 | — |
| 9.5 | code-constraints:5.5 | — |
| 9.6 | code-constraints:5.6 | — |
| 9.7 | code-constraints:5.7 | — |
| 9.8 | code-constraints:5.8 | — |
| 9.9 | code-constraints:5.9 | — |
| 10.1 | code-constraints:6.1 | — |
| 10.2 | code-constraints:6.2 | — |
| 10.3 | code-constraints:6.3 | — |
| 10.4 | code-constraints:6.4 | — |
| 10.5 | code-constraints:6.5 | — |
| 10.6 | code-constraints:6.6 | — |
| 11.1 | code-constraints:7.1 | code-locality-data:3.3, clone-constraints:2.4 |
| 11.2 | code-constraints:7.2 | — |
| 11.3 | code-constraints:7.3 | — |
| 11.4 | code-constraints:7.4 | — |
| 11.5 | code-constraints:7.5 | — |
| 11.6 | code-constraints:7.6 | — |
| 12.1 | code-constraints:8.1 | — |
| 12.2 | code-constraints:8.2 | — |
| 12.3 | code-constraints:8.3 | — |
| 12.4 | code-constraints:8.4 | — |
| 12.5 | code-constraints:8.5 | — |
| 12.6 | code-constraints:8.6 | — |
| 13.1 | code-constraints:9.1 | — |
| 13.2 | code-constraints:9.2 | — |
| 13.3 | code-constraints:9.3 | — |
| 13.4 | code-constraints:9.4 | — |
| 13.5 | code-constraints:9.5 | — |
| 13.6 | code-constraints:9.6 | — |
| 14.1 | code-constraints:10.1 | — |
| 14.2 | code-constraints:10.2 | clone-constraints:3.2, code-locality-data:4.3 |
| 14.3 | code-constraints:10.3 | — |
| 14.4 | code-constraints:10.4 | clone-constraints:3.4, code-locality-data:4.3 |
| 14.5 | code-constraints:10.5 | — |
| 5.1 | clone-constraints:1.1 | — |
| 5.2 | clone-constraints:1.2 | — |
| 5.3 | clone-constraints:1.3 | — |
| 5.4 | clone-constraints:1.4 | — |
| 5.5 | clone-constraints:1.5 | — |
| 5.6 | clone-constraints:1.6 | — |
| 5.7 | clone-constraints:1.7 | — |
| 6.1 | code-locality-data:1.1 | — |
| 6.2 | code-locality-data:1.2 | — |
| 6.3 | code-locality-data:1.3 | — |
| 6.4 | code-locality-data:1.4 | — |
| 6.5 | code-locality-data:1.5 | — |
| 6.6 | code-locality-data:1.6 | — |
| 7.1 | code-locality-data:2.1 | — |
| 7.2 | code-locality-data:2.2 | — |
| 7.3 | code-locality-data:2.3 | — |
| 7.4 | code-locality-data:2.4 | — |
| 7.5 | code-locality-data:2.5 | — |
| 8.1 | code-locality-data:3.1 | — |
| 8.2 | code-locality-data:3.2 | — |
| 8.3 | code-locality-data:3.3 | — |
| 8.4 | code-locality-data:3.4 | — |
| 8.5 | code-locality-data:3.5 | — |
| 8.6 | code-locality-data:3.6 | — |

## Added review safeguards

- code-constraints:11.1–11.7 and 12.1–12.5: provenance, comparison eligibility, decomposition/floors, source severity, deletion/rename accounting, required coverage and prototype/rollout gates.
- cli-contracts:2.1–2.4: input/report separation, view compatibility and first migration slice.
- verification-evidence:3.1–3.8 and 4.1–4.4: independently verified identity, exact test collection, nonvacuous acceptance and early shared harness.
- clone-constraints:2.1–2.4 and 3.1–3.4: source dialects, separate detector semantics, correct pair dedupe and reuse/family corpus.
- code-locality-data:4.1–4.4: advisory calibration, ambiguous extraction and independent journeys.

## Deferred vision retained

Structure-constraints owns future file/architecture/call cycles, layers, new dependencies, fan-in/out, coupling and reachable complexity. Lean-code owns unused routines/parameters/classes/variables, pass-through, single implementation and over-export. Kiro's original intentions/precedence, supersession/envelope comparison, production-code separation, environment doctor, vocabulary labeler and plugin report readout remain separate roadmap work. No deferred capability is marked implemented or passed by this split.
