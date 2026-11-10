# Implementation Plan

Task generation authorized by the owner on 2026-11-09; the corresponding design is approved for this phase. Tasks are generated for review, not approved or executed.

## Execution contract

Requires code-constraints M3 (task 9.3), verification-evidence E3 and the completed vocabulary-grounding scoped Lookup contract. Final acceptance requires code-constraints platform/scale gate 10.3. No dependency on clone-constraints. Reuse source bundles, vocabulary resolution and shared reporting/harness; new rules stay advisory pending human-reviewed calibration.

Execute sequentially in the foreground. No `(P)` markers are used: shared integration files and milestone reviews are serialized. Local task order supplies the default dependency; `_Depends:_` records additional cross-group prerequisites. Cross-spec prerequisites above are mandatory and are not local numeric task IDs.

Each executable subtask targets 1-3 hours of focused implementation or validation. Time-box prototypes to their stated question; if an outcome cannot be achieved, record the failed gate and stop dependent work rather than silently widening scope. Platform jobs and owner review can add elapsed time. Use the design's File Structure Plan for file ownership. Multi-boundary tasks are explicit integration work and must coordinate upstream-owned registrations.

A task completes only with its observable result and fresh scoped verification. Do not mark a capability Passed from task completion, file existence or unit tests alone. Keep `.kiro/breadth.toml` planned until implementation supplies real selectors and current acceptance; required missing/skipped/empty/stale evidence cannot pass. Planned journey entrypoints are declared in [the suite registry](../../verification-suites.toml), with no claimed executable test identities yet.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| L0 / G1 | 1.3 | One pinned extraction engine and labeled contract |
| L1 / G2 | 3.3 | Scattered-definition consolidation journey |
| L2 | 4.3 | State/schema and ambiguity behavior |
| L3 | 6.2 | Supported data profiles and vocabulary journeys |
| L4 / G4-G6 | 7.3 | Noise/scale/platform and current evidence |

A milestone does not approve later policy changes or declare the whole spec accepted. G1 failure returns to design; a failing first behavioral slice blocks expansion. Required release evidence and any concrete policy approval remain explicit final gates.

## Tasks

- [ ] 1. Prove bounded extraction and advisory settings
- [ ] 1.1 Prepare labeled patterns and extraction prerequisites
  - Provision pinned query-engine/grammar candidates for a native-only experiment under configured artifact/cache roots.
  - Create positive/negative Rust, Python and TS/JS definition/state fixtures plus supported receivers, ambiguities and computed-access cases.
  - Done when expected facts/confidence/findings are explicit and the experiment builds without changing source walking or vocabulary storage.
  - _Boundary: LocalityTests_
  - _Requirements: 1.1, 1.6, 2.1, 2.3, 3.1, 3.5, 4.1, 4.2, 4.3_

- [ ] 1.2 Compare direct queries and ast-grep on the fixed patterns
  - Compare correctness, pattern complexity, grammar/build compatibility and query limits against the corpus.
  - Choose and pin one engine/configuration syntax while keeping grouping and vocabulary decisions in pure policy.
  - Done when a bounded comparison admits an engine with concrete evidence; unmet extraction contracts stop for design revision.
  - _Boundary: LocalityExtract_
  - _Depends: 1.1_
  - _Requirements: 1.1, 2.1, 2.3, 3.5, 4.1, 4.2_

- [ ] 1.3 Integrate reason-bearing settings and advisory defaults
  - Implement thresholds, binding/subject/path ignores, topics and pattern settings with reasons and strict unknown-key validation.
  - Tie fixtures to shipped idioms, textual-grouping limits and unsupported receiver/computed-access examples.
  - Done when G1 passes and warning/info defaults remain unchanged absent separate approved calibration.
  - _Boundary: LocalityConfig, LocalityDocs_
  - _Depends: 1.2_
  - _Requirements: 1.4, 1.6, 2.4, 2.5, 3.3, 3.5, 3.6, 4.1_

- [ ] 2. Establish private facts and definition extraction
- [ ] 2.1 Represent extraction confidence without semantic inference
  - Model bindings, typed comparisons, qualified enum evidence and receiver-aware data accesses as transient extension facts.
  - Represent supported, ambiguous, unsupported and failed extraction in coverage; distinguish declaration identity from textual subject spelling.
  - Done when fixtures separate unavailable analysis from measured absence without adding numeric-baseline state.
  - _Boundary: LocalityFacts_
  - _Depends: 1.3_
  - _Requirements: 1.1, 2.1, 2.3, 3.1, 3.2, 4.2_

- [x] 2.2 Extract supported module-level definitions
  - Extract Rust const/static, Python module assignments and TS/JS top-level bindings from shared texts.
  - Apply whitespace-only normalization and the 12-line initializer cap; record unreadable cases without evaluating referenced values.
  - Done when language fixtures capture expected name/value/ranges and long or malformed definitions cannot enter groups.
  - _Boundary: LocalityExtract_
  - _Depends: 2.1_
  - _Requirements: 1.1, 1.4, 1.6, 4.2_

- [ ] 3. Deliver the scattered-definition consolidation slice
- [x] 3.1 Group definitions by name and initializer
  - Count distinct files per name/value, apply idiom ignores and report selected files with up to three other locations.
  - Keep different values separate and add ambiguity information when each value group reaches the required population.
  - Done when threshold/equality, unrelated values and ignore fixtures yield exact hints without merging unrelated definitions.
  - _Boundary: LocalityRules_
  - _Depends: 2.2_
  - _Requirements: 1.2, 1.3, 1.4, 1.5, 1.6_

- [ ] 3.2 Integrate the advisory extension into the base gate
  - Serialize additive module/catalogue/config and orchestration registration through base owners and consume their ownership presentation.
  - Return findings and extraction coverage while retaining numeric/baseline and shared CLI behavior.
  - Done when enabled/disabled runs expose findings/skips correctly and core has no query-engine dependency.
  - _Boundary: LocalityRules, LocalityConfig_
  - _Depends: 3.1_
  - _Requirements: 1.2, 1.4, 4.1, 4.2_

- [x] 3.3 Prove consolidation removes the real-binary finding
  - Run a four-file repeated-definition fixture, consolidate the declaration and repeat the same check.
  - Assert output/exit/file effects through shared evidence and include a disabled-rule variant that breaks the journey.
  - Done when current L1 evidence proves consolidation without state or data-rule prerequisites.
  - _Boundary: LocalityTests_
  - _Depends: 3.2_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 4.3, 4.4_

- [ ] 4. Add textual state and schema aggregation
- [ ] 4.1 Extract literal comparisons and conservative enum evidence
  - Extract equality/inequality, literal match arms and literal-set membership by subject spelling and typed values.
  - Require unique declaration and qualified variant evidence for completeness; wildcard, alias and ambiguous cases are unavailable.
  - Done when supported patterns match expected facts without claiming same-spelled unrelated subjects share semantics.
  - _Boundary: LocalityExtract_
  - _Depends: 3.3_
  - _Requirements: 2.1, 2.3, 2.5, 4.2_

- [ ] 4.2 Evaluate scattered state and exhaustive-enum information
  - Apply state/file thresholds and reason-bearing ignores with at most one finding per subject per selected file.
  - Keep single-file owners clear and produce enum information only from complete unambiguous evidence.
  - Done when single-owner, ignored, unrelated-name, alias and wildcard fixtures preserve advisory/unknown behavior.
  - _Boundary: LocalityRules_
  - _Depends: 4.1_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 4.1, 4.3_

- [ ] 4.3 Integrate schema aggregation and the state repair journey
  - Implement pure schema grouping from typed named-access fixtures with selected-file filtering and owner hints.
  - Integrate state scattering/repair through the binary and shared evidence; native data extraction follows in the next milestone.
  - Done when schema policy fixtures pass and retained state evidence proves removal after consolidation.
  - _Boundary: LocalityRules, LocalityTests_
  - _Depends: 4.2_
  - _Requirements: 2.2, 2.4, 3.4, 4.3, 4.4_

- [ ] 5. Implement receiver-aware data patterns
- [ ] 5.1 Validate custom queries before source analysis
  - Compile the single selected query language with language, receiver/position role, capture contract and reason.
  - Reject malformed syntax/roles/captures before analysis; bound evaluation without code execution or I/O.
  - Done when valid additions yield typed facts and invalid/exhausted patterns produce configuration errors or incomplete coverage respectively.
  - _Boundary: DataPatterns_
  - _Depends: 4.3_
  - _Requirements: 3.5, 3.6, 4.2_

- [x] 5.2 Extract supported Python tabular accesses
  - Add pandas, polars and NumPy positional/named/row-record patterns backed by recognized import, annotation or library-method evidence.
  - Keep ordinary list/tuple indexing clear and unknown rows ambiguous; expose computed names/positions as blind spots.
  - Done when every declared Python profile has positive/negative coverage and ambiguous receivers never produce definitive violations.
  - _Boundary: DataPatterns_
  - _Requirements: 3.1, 3.2, 3.5, 3.6, 4.2, 4.3_

- [x] 5.3 Extract supported Rust tabular accesses
  - Add polars/ndarray column/axis and named-access patterns with the declared receiver-confidence checks.
  - Retain TS/JS definitions/state support without a shipped tabular claim; keep computed/helper accesses unavailable.
  - Done when Rust profile fixtures pass with unsupported receiver/language coverage explicit.
  - _Boundary: DataPatterns_
  - _Requirements: 3.1, 3.2, 3.5, 3.6, 4.2, 4.3_

- [ ] 6. Ground names through the existing topic-scoped resolver
- [ ] 6.1 Evaluate positional access and scoped name resolution
  - Use existing Lookup for configured data topics, preserving resolved, unknown/suggestions, ambiguous, deprecated and topic-mismatch outcomes.
  - With no topics run positional checks only; missing configured vocabulary/topics cannot become a clean grounded result.
  - Done when ordinary-indexing negatives, topic isolation and every lookup outcome match the required findings/coverage without a second resolver.
  - _Boundary: DataGrounding_
  - _Depends: 5.2, 5.3_
  - _Requirements: 3.1, 3.2, 3.3, 3.6, 4.2, 4.3_

- [ ] 6.2 Integrate named-data and schema repair journeys
  - Join native data facts to name/positional policy and schema grouping through base orchestration, preserving advisory severities.
  - Run positional-to-named, vocabulary-repair and schema-consolidation journeys including no-topic and missing/ambiguous vocabulary cases.
  - Done when current evidence shows each repair and missing/disabled required grounding cannot satisfy its capability.
  - _Boundary: DataGrounding, LocalityRules, LocalityTests_
  - _Depends: 6.1_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.3, 4.4_

- [ ] 7. Validate noise, runtime and portable acceptance
- [ ] 7.1 Integrate accuracy tests and blind-spot reference examples
  - Run the labeled corpus with known negatives clear, recording precision/recall, versions and representative false positives.
  - Tie textual-name/initializer, computed-access and unsupported-library limits to executable reference examples.
  - Done when declared corpus expectations pass and representative findings remain advisory pending separate human calibration.
  - _Boundary: LocalityTests, LocalityDocs_
  - _Depends: 6.2_
  - _Requirements: 1.6, 2.5, 3.6, 4.1, 4.2, 4.3_

- [ ] 7.2 Measure global extraction cost and bounded failures
  - Exercise large bundles/adversarial queries with global aggregation included in recorded base task/whole-tree budgets.
  - Force work limits/malformed input and permute source order to check incomplete coverage and deterministic output.
  - Done when release budgets pass without dropping required profiles and unavailable extraction cannot produce accepted evidence.
  - _Boundary: LocalityTests_
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [ ] 7.3 Run platform integration and current locality acceptance
  - Require the base platform/scale gate; run consolidation/state/schema/data journeys on Windows/Linux/macOS with core/plugin and shared-contract checks.
  - Register actual selectors and replayable evidence through the canonical runner independently of clone completion.
  - Done when L0-L4 and all required profiles have current evidence; unsupported, skipped or stale profiles remain visibly incomplete.
  - _Boundary: LocalityTests_
  - _Requirements: 3.5, 4.1, 4.2, 4.3, 4.4_
