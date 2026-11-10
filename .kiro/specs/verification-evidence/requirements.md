# Requirements Document

## Introduction

Breadth and Execution Evidence. Owns capability planning, reference validation, execution evidence acceptance and a reusable real-binary harness/runner. Consumes feature journeys without owning their business assertions or requiring code analysis to exist.

## Scope and Adjacent Expectations

Consumes: cli-contracts. The owner authorized the reviewed amendments and progression to design on 2026-11-09. Each spec delivers and accepts its own behavior; original criterion identities are preserved in [the split map](../code-constraints/split-map.md).

## Requirements

### Requirement 1: Capability Ledger
**Objective:** As an owner reviewing what has actually been verified, I want a committed ledger linking each capability to its requirement criteria, its tests and its evidence, in the shape of QOpenBB's `.kiro/breadth.toml`, so that "planned" is never mistaken for "passed" and every spec's criteria are visibly mapped or visibly unmapped.

#### Acceptance Criteria
1. The repository shall keep a committed ledger `.kiro/breadth.toml` of `[[capabilities]]` entries, each with an `id`, `requirements` (references of the form `<spec>:<criterion>` with single criteria, whole requirements or ranges within one requirement, the QOpenBB path form `<spec>/requirements.md:<ref>` also accepted), `planned_checks`, `test_paths`, `e2e_paths`, a `status` from `planned`, `implemented`, `partial`, `unverified`, `passed`, `deferred`, and `evidence` records.
2. When `breadth check` runs, the kiro-cli shall resolve every requirement reference against the discovered specs' criteria and report `BREADTH_REQ_UNKNOWN` as an error for a reference naming no spec, requirement or criterion.
3. When `breadth check` runs, the kiro-cli shall verify that every listed test and end-to-end path exists under the project root (a `path::name` form names the file before `::`), reporting `BREADTH_TEST_MISSING` as an error, or as a warning while the capability's status is `planned`.
4. When a status lacks its required structural support, the ledger checker shall report BREADTH_STATUS_UNSUPPORTED: implemented requires actual test paths, passed requires test and end-to-end paths plus execution-record references, and partial, unverified or deferred requires an explanation, with current-execution validity decided separately by the acceptance verifier.
5. When every task that references a capability's criteria is complete in its spec's tasks document while the capability's status is still `planned`, the kiro-cli shall report `BREADTH_STALE` as a warning naming the capability and the spec, so that planned checks are never read as completed tests.
6. When `breadth check` runs, the kiro-cli shall report `BREADTH_CRITERION_UNMAPPED` at info level once per spec whose tasks have been generated, naming the number and the first five criteria that no capability maps, and `BREADTH_COVERAGE` at info level summarising capabilities by status and mapped criteria per spec.
7. If the ledger is absent, is not valid TOML, lacks the capabilities list, repeats an id or uses an unknown status, the kiro-cli shall report `BREADTH_INVALID` as an error with the line of the first problem and evaluate nothing else from the file.
8. The specification workflow shall seed planned capabilities during task generation, record actual tests during implementation, keep incomplete obligations partial and permit passed only after the acceptance verifier confirms current required journeys.
9. The repository shall seed the five split specifications in the breadth ledger before their implementation tasks are generated and run ledger checks and execution acceptance as distinct CI gates once those commands exist.

### Requirement 2: Real-Binary Acceptance Workflow
**Objective:** As the owner, I want every user-visible capability of the Code Gate accepted by an end-to-end journey over the real binary and a real repository, so that unit coverage supports but never substitutes for behaviour a user would observe.

#### Acceptance Criteria
1. The repository shall provide an end-to-end suite that drives the built `kiro-cli` binary over temporary projects assembled from real spec trees, source trees and, where a journey needs history, real git repositories created by the test, asserting on standard output, standard error, exit codes and written files rather than on internal types.
2. The acceptance workflow shall track the implementer, CI, change, onboarding, sources, breadth and attribution journeys under their owning specs, with code journeys owned by code-constraints, jscpd import by clone-constraints and the breadth journey by verification-evidence.
3. The end-to-end suite shall record commands, exits, stdout digests, fixture identities and replay instructions in retained execution artifacts, including successful runs rather than relying on captured console output alone.
4. The CI gate shall run required journeys after component checks and validate current execution evidence before accepting any capability as passed.
5. The repository shall run the same acceptance entry point locally and in Dagger, reporting missing required tools as incomplete acceptance and never converting their skipped journeys into passed status.
6. The rules reference shall state the end-to-end-first policy: a capability's acceptance is its journey, unit tests pin algorithms, and a clean unit run is not acceptance.

### Requirement 3: Current Execution Evidence
**Objective:** As a reviewer, I want evidence tied to actual executions and tested inputs rather than claims in prose.

#### Acceptance Criteria
1. The acceptance verifier shall require structured records naming capability, criteria, exact test identities, command and working directory, collected and executed counts, result and required skips, tested input identity, platform and artifact digests.
2. The acceptance verifier shall compute expected source, specification, configuration, lockfile, test and binary identity independently of a submitted record and reject mismatches.
3. If a required test is missing, not collected, not executed, failed or skipped, the acceptance verifier shall reject passed status even when the runner exits zero.
4. The acceptance verifier shall reject a test selector that names an existing file but no collected executable test and reject an empty required suite.
5. The acceptance verifier shall require every in-scope release criterion to be mapped and supported by current required journeys, accepting a deferral only with explicit owner approval, scope and reason.
6. The evidence workflow shall preserve historical runs without treating them as current acceptance and keep multi-obligation capabilities partial until all required obligations pass.
7. The evidence workflow shall preserve replayable fixture and environment identity plus stdout, stderr and written-file results, excluding secrets and storing disposable artifacts in configured build-artifact directories.
8. The acceptance verifier shall exclude evidence records themselves from tested-input identity and bind acceptance to the tested tree or content manifest rather than pretending a later evidence-only commit was executed.

### Requirement 4: Early Harness and Portable Gates
**Objective:** As an implementer, I want acceptance checks available before feature integration.

#### Acceptance Criteria
1. The verification workflow shall provide the shared harness and one existing-binary journey before code-analysis implementation, reusing the established CLI test setup.
2. The verification workflow shall run component tests and journey tests through one canonical entry point locally and in CI without executing the same suite twice.
3. The verification workflow shall test rejected evidence by changing test identifiers, result status and input identities and by disabling a detector in a controlled fixture.
4. The verification workflow shall collect requirements and design coverage independently of implementation status and leave planned capabilities unverified until executable acceptance is available.
