# Implementation Plan

Task generation authorized by the owner on 2026-11-09; the corresponding design is approved for this phase. Tasks are generated for review, not approved or executed.

## Execution contract

Requires code-constraints M3 (task 9.3), its validated M0-M2 contracts, and verification-evidence E3. Final acceptance requires the base platform/scale gate (code-constraints 10.3). No dependency on code-locality-data. Reuse immutable source bundles, shared report composition and evidence support; do not add a filesystem walk or numeric-baseline state.

Execute sequentially in the foreground. No `(P)` markers are used: shared integration files and milestone reviews are serialized. Local task order supplies the default dependency; `_Depends:_` records additional cross-group prerequisites. Cross-spec prerequisites above are mandatory and are not local numeric task IDs.

Each executable subtask targets 1-3 hours of focused implementation or validation. Time-box prototypes to their stated question; if an outcome cannot be achieved, record the failed gate and stop dependent work rather than silently widening scope. Platform jobs and owner review can add elapsed time. Use the design's File Structure Plan for file ownership. Multi-boundary tasks are explicit integration work and must coordinate upstream-owned registrations.

A task completes only with its observable result and fresh scoped verification. Do not mark a capability Passed from task completion, file existence or unit tests alone. Keep `.kiro/breadth.toml` planned until implementation supplies real selectors and current acceptance; required missing/skipped/empty/stale evidence cannot pass. Planned journey entrypoints are declared in [the suite registry](../../verification-suites.toml), with no claimed executable test identities yet.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| K0 / G1 | 1.3 | Pinned detector/scorer decisions and report dialects |
| K1 / G2 | 3.2 | Introduced-copy and repair journey |
| K2 | 4.3 | Token-family compatibility and bridge exclusions |
| K3 / G4-G6 | 7.2 | Imports, scale/platform and current evidence |

A milestone does not approve later policy changes or declare the whole spec accepted. G1 failure returns to design; a failing first behavioral slice blocks expansion. Required release evidence and any concrete policy approval remain explicit final gates.

## Tasks

- [ ] 1. Prove reusable detection and scoring semantics
- [ ] 1.1 Prepare clone corpus and development prerequisites
  - Provision pinned development-only jscpd tooling under configured artifact/cache roots; runtime remains bundled/offline.
  - Create labeled copies, renamed routines, threshold ties, repeated tokens, asymmetric pairs and captured report dialects with upstream provenance.
  - Done when expected ranges/scores and tool versions are recorded and the corpus runs independently of locality features.
  - _Boundary: CloneTests_
  - _Requirements: 1.1, 1.2, 1.6, 2.1, 3.1, 3.2_

- [ ] 1.2 Compare native detector and family scorer candidates
  - Prototype compatible cpd-core/tokenizer releases and adopted ratio libraries against the labeled normalized-line and family expectations.
  - Record normalization/window/range, grammar, build/license and deterministic-score differences; do not depend on the internal CLI target.
  - Done when reuse decisions have executable evidence and custom work is limited to demonstrated mismatches; semantic changes stop for design revision.
  - _Boundary: CloneSources_
  - _Depends: 1.1_
  - _Requirements: 1.1, 1.2, 1.5, 2.2, 3.1_

- [ ] 1.3 Integrate admitted semantics and extension settings
  - Implement typed floors/thresholds, ignored hook names and reason-bearing overrides with explicit supported report dialects.
  - Tie config fixtures to documented token/AST distinctions and renamed-copy versus semantic-rewrite limits.
  - Done when G1 passes and invalid settings fail through the base configuration contract without a second policy file.
  - _Boundary: CloneConfig, CloneDocs_
  - _Depends: 1.2_
  - _Requirements: 1.1, 1.2, 1.4, 1.6, 2.1, 2.2, 3.1_

- [ ] 2. Establish private facts and normalized duplicate detection
- [ ] 2.1 Validate range, shape and detector identities
  - Implement private pairs/shapes with canonical endpoints, producer/method identity and separate coverage.
  - Reject reversed, negative, out-of-root/self-overlapping ranges and nonfinite scores; require one deterministic token dictionary per bundle.
  - Done when fixtures preserve method distinctions and incompatible token dictionaries cannot be compared.
  - _Boundary: CloneModel_
  - _Depends: 1.3_
  - _Requirements: 1.1, 1.2, 1.7, 2.2, 2.3, 2.4_

- [x] 2.2 Extract confirmed normalized-line duplicates
  - Integrate the admitted detector/tokenizer over shared texts, counting code-bearing lines and confirming hash candidates against normalized text.
  - Merge contained windows deterministically and keep parse failures/work limits visible.
  - Done when below/at-12-line, comments/formatting, same-file and self-overlap fixtures produce exact expected ranges.
  - _Boundary: CloneSources_
  - _Depends: 2.1_
  - _Requirements: 1.1, 2.4, 3.1_

- [ ] 2.3 Render selected findings and merge equivalent clone pairs
  - Report both selected ends, up to three other locations and stable hints through the base envelope.
  - Deduplicate only compatible two-ended pairs; retain one-end overlaps, different methods and provenance.
  - Done when selection/merge fixtures retain distinct clones without self-only repetitions.
  - _Boundary: CloneRules_
  - _Depends: 2.2_
  - _Requirements: 1.1, 1.7, 2.3, 2.4_

- [ ] 3. Deliver the first duplicate journey
- [ ] 3.1 Integrate clone configuration and results into the base gate
  - Request additive module/catalogue/config and orchestration registrations through base owners, serializing shared files.
  - Pass private extension results to common reports/coverage without changing numeric baselines or source walking.
  - Done when the real code command reports enabled clones and disabled/unavailable coverage explicitly.
  - _Boundary: CloneSources, CloneRules, CloneConfig_
  - _Depends: 2.3_
  - _Requirements: 1.1, 1.7, 2.4_

- [x] 3.2 Prove introduced-copy detection and removal
  - Introduce a qualifying copy in an isolated project, assert locations/output/exit under warning-level gating, remove it and rerun.
  - Record shared evidence and a disabled-detector variant that breaks the behavioral assertion.
  - Done when current K1 evidence proves copy/repair without token-family or locality prerequisites.
  - _Boundary: CloneTests_
  - _Depends: 3.1_
  - _Requirements: 1.1, 3.3_

- [ ] 4. Implement compatible token families
- [x] 4.1 Extract normalized eligible routine shapes
  - Normalize identifiers/literals while preserving other tokens and compatible entity/statement identities.
  - Exclude cross-language pairs, missing statement counts and parse failures; preserve nesting for filtering.
  - Done when floor/normalization fixtures match the oracle and missing counts never become zero or eligible shapes.
  - _Boundary: CloneSources_
  - _Depends: 3.2_
  - _Requirements: 1.2, 1.4, 2.4_

- [x] 4.2 Score canonical pairs with exact pruning
  - Wire the admitted library ratio with canonical argument order; write no scorer.
  - Apply length-band/multiset ceilings only when threshold is unreachable; cache and bound pair work.
  - Done when asymmetric/repeated-token/tie cases match the recorded corpus scores and exhaustive small cases prove pruning loses no accepted pair.
  - _Boundary: CloneRules_
  - _Depends: 4.1_
  - _Requirements: 1.2, 1.5, 3.2_

- [x] 4.3 Build families after exclusions and reachability repair
  - Build connected components, remove ignored/nested members and recompute reachability so excluded third-member bridges cannot connect survivors.
  - Emit one selected-anchor family with sorted members, weakest retained edge and same-file shrink/cross-file delete hints.
  - Done when bridge, nesting, ignores, language and selection fixtures yield exactly the expected stable families.
  - _Boundary: CloneRules_
  - _Requirements: 1.3, 1.4, 1.6, 2.2, 3.2_

- [ ] 5. Import external clone reports without semantic substitution
- [ ] 5.1 Parse pinned jscpd dialects with provenance
  - Implement captured firstFile/secondFile and duplicationA/duplicationB dialects with supported producer/kind interpretation.
  - Preserve AST results as separate advisory findings, reject unknown dialects and consume explicitly supplied reports without invoking jscpd.
  - Done when fixtures retain ranges/methods and malformed or unknown reports cause source errors instead of guessed clones.
  - _Boundary: CloneSources_
  - _Depends: 4.3_
  - _Requirements: 1.7, 2.1, 2.2, 2.3, 2.4_

- [ ] 5.2 Prove imported-report and required-source journeys
  - Exercise native/import equivalence, differing endpoint overlaps, AST separation and invalid optional sources through the binary.
  - Assert absent/invalid required reports exit 2 and partial imports do not prove complete native coverage.
  - Done when retained source provenance and every import/failure assertion match the declared semantics.
  - _Boundary: CloneTests_
  - _Depends: 5.1_
  - _Requirements: 1.7, 2.1, 2.2, 2.3, 2.4, 3.3_

- [ ] 6. Measure accuracy and total work
- [ ] 6.1 Validate labeled duplicate and family accuracy
  - Run the pinned positive/negative corpus and differential scorer checks, recording precision/recall and representative findings.
  - Require declared positives found and known negatives clear; retain advisory unsupported variants and semantic-rewrite blind spots.
  - Done when corpus expectations pass without claiming semantic-equivalence detection.
  - _Boundary: CloneTests_
  - _Depends: 5.2_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 3.2, 3.4_

- [ ] 6.2 Exercise adversarial budgets and deterministic output
  - Measure large unrelated bundles and repetitive shapes including extraction/scoring within the recorded base task/whole-tree budget.
  - Force token/comparison limits and permute source order to verify incomplete coverage and deterministic reports.
  - Done when budgets pass and exhausted work never satisfies required clone acceptance.
  - _Boundary: CloneTests_
  - _Requirements: 1.5, 2.4, 3.4_

- [ ] 7. Accept the extension at the serialized platform join
- [ ] 7.1 Integrate rule references and base contract regressions
  - Tie detector definitions, exact pruning claims, family limits and dialect examples to executable fixtures.
  - Rerun base config/report/ownership tests at the join, ensuring clone facts never enter numeric baselines.
  - Done when base behavior is preserved and documented examples match actual findings.
  - _Boundary: CloneRules, CloneDocs_
  - _Depends: 6.2_
  - _Requirements: 1.6, 2.2, 2.3, 2.4, 3.4_

- [ ] 7.2 Run platform journeys and current scoped evidence
  - Require the base platform/scale gate; run copy/repair, family, import and source-failure journeys on Windows/Linux/macOS with core/plugin checks.
  - Register actual selectors/evidence only after scoped acceptance and retain incomplete/disabled variants as rejection tests.
  - Done when K0-K3 have current required evidence and missing, skipped or stale runs cannot mark capabilities Passed.
  - _Boundary: CloneTests_
  - _Depends: 7.1_
  - _Requirements: 2.1, 2.4, 3.3, 3.4_
