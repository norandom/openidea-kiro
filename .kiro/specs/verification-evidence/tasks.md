# Implementation Plan

Task generation authorized by the owner on 2026-11-09; the corresponding design is approved for this phase. Tasks are generated for review, not approved or executed.

## Execution contract

Requires cli-contracts C2 (task 5.2). Rust, Git, PowerShell 7 and nextest are explicit runner prerequisites; Dagger provisions its own matching tools. Accept this spec and cli-contracts independently of future planned code capabilities. No command in this plan is assumed available before its implementation task.

Execute sequentially in the foreground. No `(P)` markers are used: shared integration files and milestone reviews are serialized. Local task order supplies the default dependency; `_Depends:_` records additional cross-group prerequisites. Cross-spec prerequisites above are mandatory and are not local numeric task IDs.

Each executable subtask targets 1-3 hours of focused implementation or validation. Time-box prototypes to their stated question; if an outcome cannot be achieved, record the failed gate and stop dependent work rather than silently widening scope. Platform jobs and owner review can add elapsed time. Use the design's File Structure Plan for file ownership. Multi-boundary tasks are explicit integration work and must coordinate upstream-owned registrations.

A task completes only with its observable result and fresh scoped verification. Do not mark a capability Passed from task completion, file existence or unit tests alone. Keep `.kiro/breadth.toml` planned until implementation supplies real selectors and current acceptance; required missing/skipped/empty/stale evidence cannot pass. Planned journey entrypoints are declared in [the suite registry](../../verification-suites.toml), with no claimed executable test identities yet.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| E0 | 2.3 | Ledger structure and pure acceptance policy |
| E1 / G2 | 3.2 | Reusable harness and existing-binary journey |
| E2 / G5 | 5.3 | Independent identity and false-claim rejection |
| E3 / G6 | 8.2 | Portable runner and current foundation evidence |

A milestone does not approve later policy changes or declare the whole spec accepted. G1 failure returns to design; a failing first behavioral slice blocks expansion. Required release evidence and any concrete policy approval remain explicit final gates.

## Tasks

- [ ] 1. Establish schemas and native test prerequisites
- [ ] 1.1 Prepare evidence build and runner prerequisites
  - Provision pinned nextest and PowerShell 7, discover Git and the existing binary build, and prepare owned artifact roots without drive-root output.
  - Introduce the runner entry point with explicit tool detection and suite validation; probe pinned nextest and doctest collection/result formats before depending on exact-test evidence.
  - Done when prerequisite smoke runs and collection/result-format fixtures work on both platforms; missing tools produce incomplete/nonzero results, and inability to establish required test identities stops for design revision before policy integration.
  - _Boundary: VerificationRunner_
  - _Requirements: 2.5, 3.1, 3.3, 3.4, 4.1, 4.2_

- [ ] 1.2 Parse capability plans with strict schema validation
  - Implement versioned capabilities, statuses, explanations and scoped deferrals over the existing TOML/parser stack.
  - Reject missing or malformed ledgers, duplicate IDs and unknown statuses at the first problem without evaluating partial invalid data.
  - Done when valid seeded entries parse and each invalid fixture returns the expected positioned BREADTH_INVALID finding.
  - _Boundary: BreadthLedger_
  - _Depends: 1.1_
  - _Requirements: 1.1, 1.7, 1.9_

- [ ] 2. Implement independent structural and execution policy
- [ ] 2.1 Resolve criterion, path and status support
  - Resolve numeric, range, whole-requirement and legacy path references using existing spec/task views; inspect only the file portion of planned selectors.
  - Check structural support and task/status drift without vacuous all-complete results; report mapped coverage and unmapped criteria.
  - Done when fixtures distinguish planned warnings, implemented errors, nonexistent references and zero-task cases without claiming executed acceptance.
  - _Boundary: BreadthLedger_
  - _Depends: 1.2_
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6_

- [ ] 2.2 Validate exact execution records and expected test sets
  - Define records and pure acceptance over an independently supplied expected context, with exact suite/binary/test identities and unique result accounting.
  - Reject empty required suites, nonexistent selectors, missing/not-run/failed/skipped tests, duplicates and misleading exit-zero totals.
  - Done when a valid complete fixture passes and changing any required test identity or status makes acceptance fail.
  - _Boundary: EvidencePolicy_
  - _Depends: 2.1_
  - _Requirements: 3.1, 3.3, 3.4, 3.6_

- [ ] 2.3 Enforce release scope, deferrals and artifact identity
  - Require all in-scope criteria and required platforms, compare supplied expected identities, and validate artifact digest references.
  - Accept only owner/reason/approval-reference deferrals for explicitly excluded criteria; preserve Deferred and Partial rather than inventing Passed.
  - Done when stale inputs, unsupported schemas, unrelated criteria, incomplete multi-obligation records and malformed deferrals cannot satisfy acceptance.
  - _Boundary: EvidencePolicy_
  - _Requirements: 1.4, 2.4, 3.2, 3.5, 3.6, 3.8_

- [ ] 3. Ship the existing-binary journey before code analysis
- [ ] 3.1 Extend the existing fixture helper with replayable events
  - Reuse temporary-project and process helpers; add isolated Git fixtures and argv/cwd/exit/stdout/stderr/file-effect event records.
  - Retain output on success and failure, normalize fixture recipes instead of random paths, and exclude secrets from environment recording.
  - Done when two isolated fixture runs produce replayable artifacts with asserted output/file effects and cannot mutate one another.
  - _Boundary: JourneyHarness_
  - _Depends: 2.3_
  - _Requirements: 2.1, 2.3, 3.7, 4.1_

- [ ] 3.2 Prove a spec-check failure and repair with retained evidence
  - Drive the existing check binary over a real spec fixture and temporary Git project, then repair its violation and rerun.
  - Add a controlled disabled-check variant whose behavioral assertions fail; this is a mutation fixture, not a passing capability run.
  - Done when failure and repair artifacts are retained, disabling the checked behavior breaks the journey and no code-analysis feature is required.
  - _Boundary: EvidenceTests_
  - _Depends: 3.1_
  - _Requirements: 2.1, 2.2, 2.3, 4.1, 4.3_

- [ ] 4. Calculate independent tested-input identity
- [ ] 4.1 Build confined input and artifact manifests
  - Pin native hashing support and enumerate relevant source/spec/config/lock/test bytes, toolchain, platform and the binary actually launched.
  - Include selected dirty/untracked content, canonicalize/confine paths and allow external artifacts only through explicit artifact roots.
  - Done when changing each identity dimension or tampering with artifact bytes is detected while unrelated environment values stay unrecorded.
  - _Boundary: EvidenceInputs_
  - _Depends: 3.2_
  - _Requirements: 3.2, 3.7, 3.8_

- [ ] 4.2 Reject mid-run mutation while preserving historical evidence
  - Capture relevant manifests before and after execution and reject a run whose tested inputs changed.
  - Exclude evidence/output artifacts themselves and compare fixture contents/setup recipes rather than temporary directory names.
  - Done when a source mutation invalidates a run, an evidence-only addition does not self-invalidate it and old records remain historical after a real input change.
  - _Boundary: EvidenceInputs_
  - _Requirements: 3.2, 3.6, 3.8_

- [ ] 5. Expose planning and acceptance as distinct commands
- [ ] 5.1 Integrate structural breadth checking with shared CLI policy
  - Register additive core catalogue/module and CLI command/profile entries through their owners; compose the shared report/gate groups.
  - Return structural findings while explicitly reporting current acceptance as NotEvaluated; never execute ledger command text.
  - Done when valid plans, unknown refs, missing paths and invalid TOML yield deterministic reports and correct exits through the binary.
  - _Boundary: BreadthCommand_
  - _Depends: 2.1, 4.2_
  - _Requirements: 1.2, 1.3, 1.4, 1.6, 1.7, 1.9_

- [ ] 5.2 Integrate current acceptance and independent native inputs
  - Connect acceptance policy to independently resolved scope, actual build identity and loaded records for per-spec/all selection.
  - Make missing acceptance obligations mandatory failures independent of configurable finding thresholds; preserve usage/I/O exit 2.
  - Done when the foundation scope can pass despite downstream planned entries and all-scope acceptance refuses incomplete non-deferred obligations.
  - _Boundary: BreadthCommand_
  - _Depends: 2.3, 4.2, 5.1_
  - _Requirements: 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.8_

- [ ] 5.3 Exercise false claims through the real acceptance command
  - Mutate exact test names, result counts/status, source/config/lock/test/binary/platform identity, artifact bytes and required-platform coverage.
  - Cover existing-file/no-test selectors, empty/skipped exit-zero suites, duplicate results, mid-run changes and evidence-only additions.
  - Done when every false-claim fixture is rejected with its expected reason and the matching complete record alone is accepted.
  - _Boundary: EvidenceTests_
  - _Depends: 5.2_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.3_

- [ ] 6. Normalize collected and executed suites once
- [ ] 6.1 Adapt nextest collection and execution results
  - Consume the checked-in planned suite registry, promoting only implemented entrypoints with exact collected identities to runnable registration; planned checks and empty required-test sets are not executable acceptance.
  - Pin and fixture-test collection/JUnit normalization with distinct component and e2e partitions and bounded workers.
  - Done when registry tests prove full required-ID coverage, detect missing or duplicated suites and produce valid records from successful and failed nextest runs.
  - _Boundary: VerificationRunner_
  - _Depends: 5.3_
  - _Requirements: 2.4, 4.2, 4.4, 3.1, 3.3, 3.4_

- [ ] 6.2 Add explicit doctest identity and canonical orchestration
  - Run doctests as a separate declared suite with collected/executed identity support; refuse required doctest acceptance if the adapter cannot establish it.
  - Build once, snapshot identity, run each selected partition once, resnapshot and invoke planning then current acceptance with correct failure propagation.
  - Done when a local foundation run produces accepted records and a duplicate partition, unavailable doctest identity or required skip cannot pass.
  - _Boundary: VerificationRunner_
  - _Requirements: 2.4, 2.5, 4.2, 4.4_

- [ ] 7. Integrate workflow lifecycle and container execution
- [ ] 7.1 Integrate evidence lifecycle into task and completion workflows
  - Add evidence-owned workflow sections to task generation, implementation and validation, preserving code-owned sections and foreground execution.
  - Bind implemented selectors and planned suite declarations to the seeded ledger and suite registry; test Planned, Partial, Unverified and current Passed lifecycle behavior.
  - Done when executable workflow/ledger fixtures prevent promotion from file existence or prose claims, and the evidence reference explains replay and journey ownership.
  - _Boundary: EvidenceDocs_
  - _Depends: 6.2_
  - _Requirements: 1.8, 1.9, 2.2, 2.6, 4.4_

- [ ] 7.2 Run the canonical entry point in Dagger and CI
  - Provision PowerShell 7, nextest, Git and required Rust targets in the container and invoke the same runner used locally.
  - Serialize CI changes, publish successful and failed artifacts and keep structural validity and executed acceptance as separate gate results.
  - Done when container and CI smoke runs use the same registry without duplicate suites and missing required tooling fails acceptance.
  - _Boundary: VerificationRunner_
  - _Depends: 6.2, 7.1_
  - _Requirements: 1.9, 2.4, 2.5, 4.2_

- [ ] 8. Accept the evidence foundation on its own scope
- [ ] 8.1 Validate portable evidence and sandbox boundaries
  - Run the refusal corpus, real-binary journeys and configured lint/build checks on Windows, Linux and macOS, including core and standalone plugin builds.
  - Check path containment, artifact retention and deterministic identity normalization; keep native hashing/process dependencies outside core.
  - Done when the platform matrix passes and a disabled detector or missing required journey still makes acceptance fail.
  - _Boundary: EvidenceTests_
  - _Depends: 7.2_
  - _Requirements: 2.1, 2.3, 2.5, 3.7, 4.1, 4.2, 4.3, 4.4_

- [ ] 8.2 Integrate foundation journeys and verify current scoped acceptance
  - Register this spec's and cli-contracts' implemented journeys and exact required test identities, with ownership of downstream journey declarations left to their specs.
  - Run current acceptance for foundation scopes and preserve future capabilities as planned; attach replayable evidence references only after acceptance.
  - Done when cli-contracts and verification-evidence have complete current scoped evidence and an unimplemented downstream capability cannot be reported passed.
  - _Boundary: VerificationRunner, EvidenceDocs_
  - _Depends: 8.1_
  - _Requirements: 1.8, 1.9, 2.2, 2.4, 3.5, 3.6, 4.4_
