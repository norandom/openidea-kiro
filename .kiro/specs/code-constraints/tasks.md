# Implementation Plan

Task generation authorized by the owner on 2026-11-09; the corresponding design is approved for this phase. Tasks are generated for review, not approved or executed.

## Execution contract

Prerequisites as relaxed by the owner on 2026-11-10: cli-contracts C1 (done) and the existing real-binary test helper. The remaining cli-contracts migrations and verification-evidence follow the port; journeys are registered with the evidence runner when it exists. G1 is a bounded experiment, not permission for open-ended custom extraction. M1 and the M2 baseline/regression journey precede optional extension integration. Each integration task serializes upstream package, catalogue and dispatch registrations. Existing Source/Finding/ownership joins stay authoritative.

Execute sequentially in the foreground. No `(P)` markers are used: shared integration files and milestone reviews are serialized. Local task order supplies the default dependency; `_Depends:_` records additional cross-group prerequisites. Cross-spec prerequisites above are mandatory and are not local numeric task IDs.

Each executable subtask targets 1-3 hours of focused implementation or validation. Time-box prototypes to their stated question; if an outcome cannot be achieved, record the failed gate and stop dependent work rather than silently widening scope. Platform jobs and owner review can add elapsed time. Use the design's File Structure Plan for file ownership. Multi-boundary tasks are explicit integration work and must coordinate upstream-owned registrations.

A task completes only with its observable result and fresh scoped verification. Do not mark a capability Passed from task completion, file existence or unit tests alone. Keep `.kiro/breadth.toml` planned until implementation supplies real selectors and current acceptance; required missing/skipped/empty/stale evidence cannot pass. Planned journey entrypoints are declared in [the suite registry](../../verification-suites.toml), with no claimed executable test identities yet.

The [archived monolithic tasks](history/2026-11-09-monolith/tasks.md) remain historical. This plan replaces them; none of their checkboxes imply progress here.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| M0 / G1 | 1.4 | Pinned provider decision and semantic corpus |
| M1 / G2 | 4.3 | Rust task-scoped failure, repair and pass |
| M2 / G3 | 7.3 | Comparable baseline/Git/imports and debt/regression acceptance |
| M3 | 9.3 | All required languages, reports and feedback |
| M4 / G4-G7 | 11.3 | Platform/scale evidence and reviewed self-gate |

A milestone does not approve later policy changes or declare the whole spec accepted. G1 failure returns to design; a failing first behavioral slice blocks expansion. Required release evidence and any concrete policy approval remain explicit final gates.

## Tasks

- [ ] 1. Prove measurement reuse before broad implementation
- [x] 1.1 Prepare the native provider experiment
  - Create the native measurement package with workspace lint settings and an isolated provider-prototype build using configured artifact/cache roots.
  - Pin candidate metric-library and grammar versions for the experiment; provision development-only comparison tools without adding them to runtime requirements.
  - Done when the experiment builds offline after provisioning and core/plugin dependency inspection shows no native parser or process dependency.
  - _Boundary: NativeMeasure_
  - _Requirements: 1.1, 9.6, 12.1_

- [x] 1.2 Create hand-counted multilingual semantic fixtures
  - Encode exact expected routine/file/type measurements for Rust, Python and TS/JS constructs, including boolean decisions, nesting, docstrings, parameters and same-file inherent impls.
  - Capture comparison provenance for the optional Lizard cross-check, distinguishing hand-counted truth from external tool disagreement.
  - Done when the prototype runner reports per-metric expected/actual values and detects deliberately changed metric definitions.
  - _Boundary: CodeTests_
  - _Depends: 1.1_
  - _Requirements: 1.1, 3.1, 3.2, 4.1, 4.2, 10.2, 12.1_

- [x] 1.3 Compare reuse candidates against the semantic contract
  - Exercise the preferred measurement library over the corpus and bounded queries only for demonstrated missing facts; record definition, grammar and producer differences.
  - Check supported extensions, malformed-source behavior, package/license compatibility and deterministic output; do not relabel incompatible formulas.
  - Done when each required metric/language has a demonstrated implementation route or a concrete failed gate that stops broad implementation.
  - _Boundary: NativeMeasure_
  - _Depends: 1.2_
  - _Requirements: 1.1, 1.5, 2.2, 11.1, 12.1_

- [ ] 1.4 Integrate the G1 decision and calibrated profile fixtures
  - Persist the tested pin/adoption decision and per-definition/language calibration as executable fixtures and supporting metric documentation.
  - Allow custom extraction only for recorded rejected reuse cases; keep uncalibrated severities advisory without weakening required completeness.
  - Done when compatibility tests validate every admitted profile and G1 is explicitly passed; otherwise return to design before task 2.
  - _Boundary: CodeTests, CodeDocs_
  - _Depends: 1.3_
  - _Requirements: 2.2, 3.1, 4.1, 4.2, 10.3, 12.1, 12.4_

- [ ] 2. Establish pure policy contracts and safe source input
- [x] 2.1 Validate observations, identity and completeness
  - Implement versioned entities, metric/producer identity, coverage cells and numeric facts independently of native parsers.
  - Reject nonfinite/conflicting observations and invalid paths; represent missing, partial, failed and not-applicable facts without numeric zero substitution.
  - Done when contract fixtures prohibit incompatible comparison and preserve provenance through deterministic serialization.
  - _Boundary: CodeFacts_
  - _Depends: 1.4_
  - _Requirements: 1.3, 1.5, 5.1, 5.9, 11.1, 11.2, 11.7_

- [x] 2.2 Resolve effective configuration and reason-bearing overrides
  - Implement defaults, scoped precedence/ties, ignores, source preferences and required metrics with strict catalogue validation.
  - Require reasons for scopes, ignores and relaxations; expose effective provenance and a stable digest before any measurement.
  - Done when missing configuration uses calibrated defaults and malformed/unknown/unreasoned settings produce positioned configuration errors.
  - _Boundary: CodeConfig_
  - _Depends: 2.1_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [ ] 2.3 Read a confined immutable source bundle
  - Implement sorted source selection, supported extensions, default exclusions, the source-size bound and project-root confinement including symlink escapes.
  - Keep immutable source texts reusable by extensions and distinguish selected parse/read failures from outside-selection diagnostics.
  - Done when containment/exclusion fixtures prove only eligible confined inputs are read and unsupported files remain explicitly unmeasured.
  - _Boundary: NativeMeasure_
  - _Depends: 2.1, 2.2_
  - _Requirements: 1.1, 1.6, 7.2, 7.3, 9.6_

- [x] 2.4 Adapt existing ownership and task selection
  - Use the existing ownership join for file/component/task relationships and roadmap precedence; do not implement a second mapping algorithm.
  - Resolve spec/task/files/all scopes and keep unowned and multiply-owned files visible; reject unknown or unmapped task boundaries.
  - Done when fixture selection contains exactly the mapped files and diagnostics name available choices with primary/secondary owners in the required order.
  - _Boundary: CodeAttribution_
  - _Depends: 2.1_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 3. Implement the Rust measurement and absolute-limit slice
- [x] 3.1 Produce Rust routine and container observations
  - Integrate the G1-admitted Rust profile, stable qualified identities, same-file inherent impl counts and first parse-error location.
  - Emit all required routine/file/type metrics with independent completeness, excluding unsupported constructs as specified.
  - Done when Rust semantic fixtures match exactly and parse errors suppress all rule facts for the affected selected file.
  - _Boundary: NativeMeasure_
  - _Depends: 2.3_
  - _Requirements: 1.1, 3.1, 3.2, 4.1, 4.2, 7.2, 11.1_

- [ ] 3.2 Judge routine and container limits with calibrated severity
  - Evaluate strict-greater-than thresholds, population floors and optional imported metrics; produce metric-specific simplify/split hints and highest values.
  - Use per-language/definition calibration and absolute-only container policy with no container ratchet.
  - Done when equality, floor, scoped-limit and optional-metric fixtures produce the expected values, severities and remedy tags.
  - _Boundary: CodeLimits_
  - _Depends: 2.2, 3.1_
  - _Requirements: 2.2, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 3.3 Define deterministic report and coverage envelopes
  - Implement the shared finding/rule outcome envelope with metric, owner, hint, provenance, match trust and pre-existing fields.
  - Render deterministic text/JSON with stable ordering and coverage based on measurement state rather than finding counts.
  - Done when repeat renderings are byte-identical and required unavailable metrics prevent a clean complete verdict.
  - _Boundary: CodeReport_
  - _Depends: 2.1, 2.4, 3.2_
  - _Requirements: 6.5, 7.1, 7.4, 7.5, 8.2, 8.3, 9.4, 11.7_

- [ ] 4. Deliver a task-scoped Rust journey
- [ ] 4.1 Integrate pure numeric evaluation and extension composition
  - Compose validated facts, configured limits, ownership and coverage in the designed evaluation order; reserve typed extension results without importing private extension state.
  - Serialize additive core exports/catalogue registration with the existing owner and retain nonblocking outside-selection diagnostics.
  - Done when pure integration fixtures show selected parse failure blocks acceptance and empty extension input leaves numeric results unchanged.
  - _Boundary: CodeEvaluate_
  - _Depends: 3.3_
  - _Requirements: 1.5, 6.1, 7.1, 7.2, 7.3, 7.4, 7.5, 11.7_

- [x] 4.2 Integrate the first code check command and shared profiles
  - Register code check additively through the shared CLI owner and expose task/spec/files/all selection, report output and eligible finding exits.
  - Connect validated config, confined source input, Rust measurement and pure evaluation; reject incompatible selections before reading/writing.
  - Done when real invocations produce correct 0/1/2 exits, task ownership and only explicitly requested outputs.
  - _Boundary: CodeCommands_
  - _Depends: 4.1_
  - _Requirements: 6.4, 6.6, 9.1, 9.2, 9.6_

- [ ] 4.3 Prove Rust fail, repair and pass before expanding scope
  - Use the shared evidence harness for a task-owned Rust violation, assert report/exit/file effects, repair source and rerun the same command.
  - Register exact implemented test identities and include a disabled-measurement variant that cannot satisfy the journey.
  - Done when current evidence proves the M1 slice on Windows/Linux/macOS with unchanged core/plugin builds; Python/TS and optional detectors remain unaccepted.
  - _Boundary: CodeTests_
  - _Depends: 4.2_
  - _Requirements: 6.1, 6.4, 8.2, 9.2, 12.2, 12.5_

- [ ] 5. Implement safe comparison and deterministic baselines
- [x] 5.1 Pair exact and changed signatures without excusing new debt
  - Pair exact comparable identities and unique signature changes; keep ambiguous candidates unpaired and missing-before unknown unless coverage proves newness.
  - Permit heuristic pairing only to add regression, never to classify pre-existing absolute breaches; bypass container and breach-only ratchets.
  - Done when exact-debt, changed-signature, ambiguous-pair and incomplete-before fixtures return the expected trust and eligibility.
  - _Boundary: CodeRatchet_
  - _Depends: 4.3_
  - _Requirements: 5.1, 5.2, 5.3, 5.5, 5.6, 5.9, 11.1, 11.2_

- [x] 5.2 Apply population, parse and decomposition safeguards
  - Check both-side parse validity and metric floors before attaching before values or excusing debt.
  - Suppress line/statement regression only on proven control-flow improvement with none worsening, excluding Essential from the exemption; retain absolute limits and strict mode.
  - Done when guard-clause, floor-crossing, parse-repair and unchanged-debt fixtures distinguish allowed refactoring from genuine regression.
  - _Boundary: CodeRatchet_
  - _Requirements: 5.2, 5.3, 5.5, 11.2, 11.3, 11.4_

- [x] 5.3 Capture and load complete deterministic baselines
  - Implement baseline read/write through the existing fact contract with sorted keys, no timestamps, config/source/scope identity and atomic requested replacement.
  - Reject incomplete required measurements and incompatible schemas; diagnose recapture/migration without automatically resetting policy.
  - Done when two captures are byte-identical, interrupted/failed writes preserve the old file and incompatible baselines cannot excuse debt.
  - _Boundary: CodeCommands_
  - _Depends: 5.2_
  - _Requirements: 5.4, 5.8, 5.9, 9.1, 9.6, 11.1, 11.7_

- [ ] 6. Preserve Git history and complete net accounting
- [x] 6.1 Read revision blobs and rename/deletion manifests safely
  - Use separate Git argv and read-only revision/diff/blob operations; preserve old/new rename paths and deletions in before/after accounting.
  - Cover tracked staged/unstaged changes while keeping untracked files explicit through all/files selection; diagnose missing/shallow history without fetching.
  - Done when temporary Git fixtures exercise rename, deletion, modification and invalid revisions without changing checkout state.
  - _Boundary: GitBefore_
  - _Depends: 5.3_
  - _Requirements: 5.4, 5.7, 9.6, 11.6_

- [x] 6.2 Compute net growth only from complete comparable scopes
  - Account for complete routine additions, deletions and explicit renames across both scope manifests, with configured net-growth limits only.
  - Return unavailable with a reason for missing/incompatible accounting facts rather than invented zero values or savings.
  - Done when renamed/deleted routines contribute signed deltas correctly and sparse or failed before measurements never produce a complete net total.
  - _Boundary: CodeRatchet_
  - _Depends: 6.1_
  - _Requirements: 5.7, 5.9, 11.2, 11.6, 11.7_

- [x] 6.3 Integrate before-side precedence and changed selection
  - Connect named revision precedence over committed baseline, comparison eligibility and distinct selected/accounting scopes.
  - Show before-side identity, debt/strict-mode behavior and net availability in text/JSON summaries; serialize the orchestration join.
  - Done when real Git and baseline fixtures demonstrate correct precedence and selection without losing old rename or deleted paths.
  - _Boundary: CodeCommands, CodeEvaluate_
  - _Depends: 6.2_
  - _Requirements: 5.3, 5.4, 5.7, 6.5, 7.1, 9.1, 11.6_

- [ ] 7. Report improvements, keep source meaning and prove the ratchet
- [x] 7.1 Render the hook summary and provide metric points
  - Build the ASCII summary (at most 25 lines, fixed order, improved and worsened metrics with before and after values, net delta or unavailable) and the metric points as views of the report model.
  - Add --summary to code check; implement the quality-telemetry provider contract for the code report and register its names in that catalogue; write no metric file.
  - Done when summary, points, text and JSON agree on the same fixtures, summary bytes are printable ASCII with LF only, and the verdict is the same with telemetry off, on and failing.
  - _Boundary: CodeReport, CodeCommands_
  - _Depends: 6.3_
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [ ] 7.2 Integrate provider selection and required-source failures
  - Define the report-source registration contract with no base adapter; select the configured provider consistently on both sides, report value conflicts and reject incompatible producer/definition/grammar comparisons.
  - Continue after optional source errors with coverage findings; make missing/invalid required sources exit 2 and preserve local versus upstream policy fields.
  - Done when multi-source fixtures cannot hide absence, silently downgrade imported findings or substitute partial observations for net completeness.
  - _Boundary: CodeCommands, CodeEvaluate_
  - _Depends: 7.1_
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 3.5, 4.5, 7.1, 9.1, 9.2, 11.5, 11.7_

- [x] 7.3 Prove baseline debt, regression and changed-source journeys
  - Drive existing debt acceptance, new regression rejection and repair through the binary with real baseline/Git fixtures.
  - Cover signature trust, decomposition, floors, parse repair, same-change moves that keep debt status, renames/deletions and required-source failure using the already implemented component corpus.
  - Done when current M2 evidence passes and a disabled ratchet breaks its journey; extension joins may proceed only after their remaining upstream prerequisites.
  - _Boundary: CodeTests_
  - _Depends: 7.2_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 10.2, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 12.2, 12.3_

- [ ] 8. Complete bundled language coverage
- [ ] 8.1 Integrate the calibrated Python measurement profile
  - Implement the G1-approved Python/pyi adapter with qualified entities, parameters, docstring/comment treatment and supported syntax.
  - Keep unavailable/failed metrics explicit and do not manufacture coverage for lambdas or excluded constructs.
  - Done when Python semantic, malformed-input and identity fixtures match the pinned profile across supported platforms.
  - _Boundary: NativeMeasure_
  - _Depends: 7.3_
  - _Requirements: 1.1, 3.1, 3.2, 4.1, 4.2, 7.2, 11.1, 12.1_

- [ ] 8.2 Integrate calibrated TypeScript and JavaScript profiles
  - Implement the admitted TS/JS grammar variants and extensions, declaration counts excluding aliases, parameters and stable identities.
  - Preserve syntax-specific unsupported coverage and do not merge incompatible grammar semantics.
  - Done when TS/JS/JSX/TSX and module-extension fixtures match the semantic oracle and parsing failures cannot yield checked files.
  - _Boundary: NativeMeasure_
  - _Requirements: 1.1, 3.1, 3.2, 4.1, 4.2, 7.2, 11.1, 12.1_

- [ ] 8.3 Verify multilingual source and ownership journeys
  - Extend real-binary implementer/source/attribution journeys to each required profile, including unowned/multiply-owned files and unknown selectors.
  - Register exact test identities under their owning capabilities and retain baseline/provider compatibility evidence.
  - Done when all supported language profiles have current behavioral evidence and unsupported language files remain visibly unmeasured.
  - _Boundary: CodeTests_
  - _Depends: 8.1, 8.2_
  - _Requirements: 1.1, 1.2, 1.6, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 10.2, 12.3_

- [ ] 9. Expose deterministic reports and agent feedback
- [ ] 9.1 Add SARIF and full report reproducibility checks
  - Render SARIF 2.1.0 with finding-code rule IDs and remediation hints while retaining deterministic text/JSON fields and highest values.
  - Test newline/path normalization, stable ties and absence of timestamps/absolute paths across platforms.
  - Done when SARIF parses against its required shape and repeated report bytes agree for identical inputs.
  - _Boundary: CodeReport_
  - _Depends: 8.3_
  - _Requirements: 3.6, 6.5, 8.2, 8.3, 9.3, 9.4_

- [ ] 9.2 Generate effective rules without overwriting surrounding steering
  - Generate scopes, reasons, limits, hint tags, lean-ladder responsibilities, ratchet policy and task self-check guidance from effective configuration.
  - Replace exactly one valid marker block, preserve surrounding text and refuse malformed/duplicate boundaries; include explicit blind spots.
  - Done when idempotence and preservation fixtures pass and generated guidance changes whenever effective policy changes.
  - _Boundary: CodeFeedback_
  - _Depends: 9.1_
  - _Requirements: 2.6, 7.6, 8.1, 8.2, 8.6_

- [ ] 9.3 Integrate companion commands and code-owned workflow sections
  - Connect config, rules/write, baseline and SARIF output through confined native I/O; serialize code-owned sections in shared implementation/validation/review workflows.
  - Add executable workflow checks for task/whole-tree verification and prohibition of agent policy relaxation; complete metric/blind-spot references, the frozen scitools-hook feature inventory and the pre-commit/pre-push hook examples for Windows, Linux and macOS.
  - Done when companion-command journeys preserve files correctly and workflow fixtures require code repair rather than baseline/ignore changes to obtain a pass.
  - _Boundary: CodeCommands, CodeFeedback, CodeDocs_
  - _Depends: 9.2_
  - _Requirements: 2.6, 7.6, 8.1, 8.4, 8.5, 8.6, 9.1, 9.3, 9.6, 10.3, 13.6_

- [ ] 10. Measure robustness, platform compatibility and total cost
- [ ] 10.1 Exercise parser robustness and complete rule regression fixtures
  - Run arbitrary-text panic-resistance/property tests over admitted native profiles and bound oversized/malformed source behavior.
  - Cover every rule, all exit classes, containment escapes and repeat-render determinism using shared harness support.
  - Done when the required corpus and robustness runs pass and a bounded-out measurement is reported incomplete rather than clean.
  - _Boundary: CodeTests_
  - _Depends: 9.3_
  - _Requirements: 7.2, 7.3, 10.2, 10.4, 12.5_

- [ ] 10.2 Validate release performance over the recorded corpus
  - Measure release task-scoped and whole-tree runs on this repository with recorded machine, build and corpus identity, including global accounting costs.
  - Assert task time below 10 seconds and whole-tree below 60 seconds; exercise work-limit exhaustion and collect representative false positives.
  - Done when the timing budget passes without reducing required coverage and uncalibrated findings remain advisory pending human review.
  - _Boundary: CodeTests_
  - _Requirements: 9.5, 10.4, 12.4, 12.5_

- [ ] 10.3 Run cross-platform and sandbox integration checks
  - Use the canonical runner for native component/journey partitions, separate doctests, fmt, clippy and dependency checks on Windows, Linux and macOS.
  - Build pure core and the standalone plugin for their established sandbox targets; rerun CLI and evidence contract fixtures at the join.
  - Done when the required matrix passes with identical reports and no native measurement dependency enters the sandbox graph.
  - _Boundary: CodeTests_
  - _Requirements: 9.4, 10.4, 10.5, 12.5_

- [ ] 11. Integrate self-application and current acceptance
- [ ] 11.1 Capture the advisory repository inventory and reviewed policy inputs
  - Execute the implemented inventory/highest-value command and retain findings by rule plus corpus identity, explicitly separating counts from quality scores.
  - Generate candidate configuration/baseline artifacts only through supported commands and attach reasons/calibration evidence for human review.
  - Done when candidates are reproducible and reviewable; blocking adoption stays pending until the owner approves these concrete policy artifacts.
  - _Boundary: CodeDocs_
  - _Depends: 10.3_
  - _Requirements: 2.2, 8.6, 10.1, 10.3, 12.4_

- [ ] 11.2 Integrate the approved self-gate without bypassing evidence
  - Prerequisite: owner approval of the exact configuration and baseline candidates from 11.1; changes invalidate that approval.
  - Commit the approved policy inputs to the working tree and serialize the cargo alias/CI additions through existing owners, using the canonical runner.
  - Done when self-gating uses the reviewed baseline, a fixture regression blocks CI and advisory/rollback settings are demonstrably effective.
  - _Boundary: CodeCommands, CodeDocs_
  - _Depends: 11.1_
  - _Requirements: 8.5, 8.6, 10.1, 12.4_

- [ ] 11.3 Accept all code journeys against current scoped evidence
  - Run implementer, CI, changed-revision, onboarding, hook-summary and attribution journeys with current binary/input identity and output/file-effect assertions.
  - Require all language/profile milestones, disabled-detector refusal and exact required tests; update ledger references only after scoped acceptance.
  - Done when M0-M4 evidence is current and complete; Rust-only, skipped or stale runs remain Partial/Unverified and cannot complete this spec.
  - _Boundary: CodeTests_
  - _Depends: 11.2_
  - _Requirements: 8.4, 8.5, 10.1, 10.4, 10.5, 12.2, 12.3, 12.4, 12.5_
