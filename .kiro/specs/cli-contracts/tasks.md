# Implementation Plan

Task generation authorized by the owner on 2026-11-09; the corresponding design is approved for this phase. Tasks are generated for review, not approved or executed.

## Execution contract

Requires the completed cli-gate command surface. Reuse the existing CLI fixture helper; verification-evidence is downstream and is not a prerequisite. C0 precedes C1; C1 must pass before remaining migrations. Later evidence integration is owned by verification-evidence.

Execute sequentially in the foreground. No `(P)` markers are used: shared integration files and milestone reviews are serialized. Local task order supplies the default dependency; `_Depends:_` records additional cross-group prerequisites. Cross-spec prerequisites above are mandatory and are not local numeric task IDs.

Each executable subtask targets 1-3 hours of focused implementation or validation. Time-box prototypes to their stated question; if an outcome cannot be achieved, record the failed gate and stop dependent work rather than silently widening scope. Platform jobs and owner review can add elapsed time. Use the design's File Structure Plan for file ownership. Multi-boundary tasks are explicit integration work and must coordinate upstream-owned registrations.

A task completes only with its observable result and fresh scoped verification. Do not mark a capability Passed from task completion, file existence or unit tests alone. Keep `.kiro/breadth.toml` planned until implementation supplies real selectors and current acceptance; required missing/skipped/empty/stale evidence cannot pass. Planned journey entrypoints are declared in [the suite registry](../../verification-suites.toml), with no claimed executable test identities yet.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| C0 | 1.2 | Compatibility profiles and baseline fixtures |
| C1 / G2 | 3.2 | Migrated check with real-binary fail/repair/pass |
| C2 / G6 | 5.2 | Every existing command conforms without changing legacy behavior |

A milestone does not approve later policy changes or declare the whole spec accepted. G1 failure returns to design; a failing first behavioral slice blocks expansion. Required release evidence and any concrete policy approval remain explicit final gates.

## Tasks

- [x] 1. Freeze compatibility and build prerequisites
- [x] 1.1 Establish the shared-library build boundary
  - Create the native shared library using the existing locked clap dependency and workspace lint policy; serialize package and CLI dependency registration through the existing package owner.
  - Keep core and plugin dependency direction unchanged and use configured build/cache directories.
  - Done when the library and existing CLI build with the lockfile and a dependency-tree check confirms no new native dependency reaches core or the plugin.
  - _Boundary: CliGroups_
  - _Requirements: 1.1, 1.2_

- [x] 1.2 Capture executable command profiles and legacy behavior
  - Declare applicability, accepted aliases, output roles and exceptions for every existing command, using the current command tree rather than assuming identical flags.
  - Use existing fixture support to capture representative check, status, graph, tasks and vocabulary outputs, including default/table/import behavior.
  - Done when the pre-migration binary passes the compatibility fixtures and an omitted command profile makes the conformance test fail.
  - _Boundary: CliConformance_
  - _Depends: 1.1_
  - _Requirements: 1.2, 1.4, 1.7, 2.1, 2.2, 2.3_

- [x] 2. Implement shared argument and output policy
- [x] 2.1 Resolve applicable arguments and environment precedence
  - Implement global/report/gate groups, explicit CLI over applicable environment over defaults, color precedence and quiet/verbose conflicts.
  - Reject invalid applicable environment enums with the variable name; do not inspect unrelated environment values.
  - Done when table-driven tests cover each fallback, explicit override, alias and conflict with the expected typed result or usage error.
  - _Boundary: CliGroups_
  - _Depends: 1.2_
  - _Requirements: 1.1, 1.3, 1.4_

- [x] 2.2 Implement report routing and exit decisions
  - Implement 0/1/2 outcomes over already eligible findings and route reports to stdout or a named destination without duplicate output.
  - Keep diagnostics separate, preserve schema-versioned JSON, and propagate output-write failures without changing rule semantics.
  - Done when output/exit fixtures prove JSON-only report destinations, stderr diagnostics and exit 2 for failed writes.
  - _Boundary: CliGroups_
  - _Requirements: 1.1, 1.5, 2.3_

- [x] 3. Deliver the first migrated command
- [x] 3.1 Integrate shared policy into the existing check command
  - Replace check's duplicated argument/context handling with the shared groups and compatibility exit exports.
  - Serialize existing CLI dispatch changes; preserve positionals, defaults, aliases and the established report schema.
  - Done when the original check compatibility cases have identical normalized outputs and exits through the migrated binary.
  - _Boundary: CliMigration_
  - _Depends: 2.1, 2.2_
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 2.4_

- [x] 3.2 Prove check failure, repair and pass through the binary
  - Run a temporary spec containing a known violation, assert the finding and exit, repair the fixture and repeat the same command.
  - Assert stdout, stderr and file outputs; cover environment override and failed report destination without waiting for the future evidence runner.
  - Done when the journey fails if check behavior is disabled and passes only after the fixture repair; retain the executable test for later evidence registration.
  - _Boundary: CliConformance_
  - _Depends: 3.1_
  - _Requirements: 1.7, 2.3, 2.4_

- [ ] 4. Migrate remaining command profiles
- [ ] 4.1 Preserve status and visualization invocations
  - Migrate status, graph and tasks through applicable shared groups while preserving their legacy default and table renderings.
  - Add command-specific formats and reject conflicting table/format selectors before output.
  - Done when legacy invocation fixtures remain equal and JSON, output destinations and invalid combinations behave as declared.
  - _Boundary: CliMigration_
  - _Depends: 3.2_
  - _Requirements: 1.2, 1.3, 1.5, 2.2_

- [ ] 4.2 Separate import input selection from report and artifact output
  - Retain legacy input --format and artifact --out spellings; add explicit input/report selectors without changing imported TOML.
  - Reject duplicate input selectors, colliding files and competing stdout destinations before any artifact write; keep text diagnostics on stderr.
  - Done when old/new invocations produce equal TOML, KIRO_FORMAT affects reports only and rejected destination cases preserve existing artifacts.
  - _Boundary: CliMigration_
  - _Requirements: 1.2, 1.3, 1.5, 2.1_

- [ ] 4.3 Integrate vocabulary validation and completion generation
  - Compose applicable groups for vocabulary validation and add completions generated from the command definitions with a compatible pinned completion dependency.
  - Keep completion stdout as a script and avoid irrelevant report/gate flags; preserve version behavior.
  - Done when validation exits match the shared contract and each supported completion shell and version invocation succeeds.
  - _Boundary: CliMigration_
  - _Requirements: 1.2, 1.4, 1.6, 1.7_

- [ ] 5. Validate compatibility and consumer integration
- [ ] 5.1 Complete executable conformance and standard agreement checks
  - Exercise every profile with representative real invocations, accepted aliases, allowed short flags, defaults and environment precedence.
  - Tie the CLI standard and its exceptions to executable profile checks; test output failures and import collision cases on isolated fixtures.
  - Done when all declared profiles are exercised and deleting a profile, alias or documented exception produces a targeted failure.
  - _Boundary: CliConformance_
  - _Depends: 4.1, 4.2, 4.3_
  - _Requirements: 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3_

- [ ] 5.2 Run the platform and sandbox compatibility gate
  - Run scoped native tests and existing workspace formatting, lint, dependency and core/plugin build checks on Windows, Linux and macOS using configured artifact roots.
  - Retain results for C0-C2 without claiming future breadth acceptance; register this spec's journeys when the evidence foundation is implemented.
  - Done when the required compatibility matrix passes and no fixture helper or native dependency has been duplicated into core or the plugin.
  - _Boundary: CliConformance_
  - _Requirements: 1.1, 1.2, 1.7, 2.3, 2.4_
