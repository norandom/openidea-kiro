# Task-plan validation

Reviewed on 2026-11-09 after the owner's instruction: "produce tasks for the specs". This authorizes progression from the five designs to task generation. Requirements and designs are approved; tasks are generated, unapproved and entirely unchecked. Implementation has not begun.

## Plans and scope

| Spec | Major groups | Executable subtasks | Criteria mapped | Components mapped |
|---|---:|---:|---:|---:|
| [cli-contracts](../cli-contracts/tasks.md) | 5 | 11 | 11 | 3 |
| [verification-evidence](../verification-evidence/tasks.md) | 8 | 18 | 27 | 8 |
| [code-constraints](tasks.md) | 11 | 35 | 74 | 13 |
| [clone-constraints](../clone-constraints/tasks.md) | 7 | 17 | 15 | 6 |
| [code-locality-data](../code-locality-data/tasks.md) | 7 | 19 | 21 | 8 |
| Total | 38 | 100 | 148 | 38 |

The CLI counts container headers as task nodes, so status reports 138 unchecked nodes in total. The 100 executable subtasks target 1-3 hours of focused work each; prototypes are bounded experiments with stop/go results, not guaranteed successful implementations. External platform jobs and human review can add elapsed time. Zero parallel markers and zero optional acceptance tasks are intentional under the sequential foreground workflow.

## Review outcome

Task Plan Review Gate: **PASS**. Task-graph sanity review: **PASS**, mode **inline**, following the project's foreground policy. The review examined the requirements and design contracts directly, including runtime setup, shared ownership, scope boundaries, prototype feasibility, first usable journeys and acceptance evidence. One planning refinement moved exact nextest/doctest collection-format validation into the runner prerequisite task, before dependent policy integration.

Each executable task has a numeric criterion mapping, an exact design-component boundary, at least three detail bullets and an observable completion result. Cross-boundary work is explicitly integration work. The draft review confirmed all criteria and components are represented before the plans were written; the disk audit checks the generated files independently.

The review concretized the design's planned suite registry as [verification-suites.toml](../../verification-suites.toml), owned by EvidenceDocs. Its five journey entrypoints are declarations only, with planned status and empty required-test lists. The evidence design now explicitly describes promotion after implementation and independent collection. This adds one planned file without changing feature behavior or claiming that the runner exists.

## Dependency and risk gates

- CLI C2 precedes evidence E3; code starts after both foundations. The CLI's first slice uses existing test helpers and does not depend on the future evidence runner.
- Code M0 proves provider reuse; M1 proves the Rust task journey; M2 proves baseline/Git/import comparability before optional detectors. M3 provides all bundled language/report contracts. The baseline/configuration approval in code task 11.2 is a future gate over concrete candidates produced in 11.1, not approval granted by task generation.
- Clone and locality each consume code M3 and the evidence foundation, with the base platform/scale gate required for their final acceptance. Neither extension depends on the other. Locality also consumes the existing vocabulary-grounding contract.
- Shared package/module/catalogue/dispatch/CI/workflow edits are serialized integrations through the existing owners. No new task duplicates source walking, the ownership join, vocabulary resolution or the common journey harness.
- Missing metric semantics, unprovable test identities or incompatible detector semantics stop the affected prototype and return to design. Missing, empty, skipped, disabled or stale execution cannot satisfy acceptance. New heuristic rules remain advisory pending separately reviewed calibration.

## Fresh validation

- All five existing `kiro-cli check --project . --feature <spec> --fail-on warning --format json` invocations exited 0 with zero errors and warnings.
- Fresh CLI status recognized the expected groups, all 138 unchecked nodes, no optional/parallel nodes, approved designs and unapproved tasks. All 75 planned files are assigned to components.
- The structural disk audit verifies complete criterion and component coverage, valid backward local dependencies, acyclic combined milestone dependencies, unique file ownership, valid metadata, relative document links and clean task formatting.
- The existing [breadth ledger](../../breadth.toml) retains all 25 capabilities as planned and exactly covers the 148 criteria. Actual-test, end-to-end and evidence arrays remain empty. The suite registry likewise contains no fabricated test identity or execution.
- `git diff --check` verifies tracked-file whitespace; the structural audit also checks new Markdown documents and conflict markers.

Local check outputs and the one-off audit are retained under `D:/BuildArtifacts/openidea-kiro/spec-tasks-20261109/`. They validate planning artifacts only. No future feature command, provider experiment, runtime test or end-to-end acceptance gate was executed during task generation.

## Next executable step

Review the generated task plans. Once task approval and implementation are requested, begin with cli-contracts task 1.1 and follow its milestone order. The archived monolithic tasks remain historical and contribute no completion state.
