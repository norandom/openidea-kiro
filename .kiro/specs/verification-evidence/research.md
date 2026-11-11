# Research & Design Decisions

## Design revision: bounded deliverables (2026-11-09)

### Summary

Owns capability planning, reference validation, execution evidence acceptance and a reusable real-binary harness/runner. Consumes feature journeys without owning their business assertions or requiring code analysis to exist.

The user instructed applying the preceding review and proceeding to designs. Requirements are approved for this design step on that basis; design approval and implementation remain pending. The original 108 criteria have explicit successors in split-map.md; added safeguards are new numeric criteria.

### Discovery and consulted guidance

Integration-focused discovery reuses the preceding repository/tool review. Consulted kiro-spec-requirements (EARS and requirements-review-gate), kiro-spec-design (design-principles, light discovery, synthesis and design-review-gate), core product/tech/structure steering and foreground workflow. Review performed inline; no subagent or model-specific implementation request was invoked.

- ../QOpenBB/.kiro/breadth.toml (200 entries), .kiro/steering/testing.md:37–45 and tests/contracts/test_evidence.py:32–72,111–115.
- Existing CLI tests/common/mod.rs supplies setup_tree/run_cli/run_cli_in. Existing Dagger gate runs cargo test; implementation will reconcile the nextest steering requirement through one runner.
- Sibling evidence policy is prior art, not evidence that its application suites or this future verifier passed.

### Synthesis and decisions

- Generalization: one pure claim validator and one native identity boundary for all feature journeys.
- Build versus adopt: reuse existing parsers, joins, fixture helpers and documented libraries before adding custom machinery; no new analyzer is introduced.
- Simplification: own only this spec's behavior; no duplicated filesystem walks, evidence frameworks or downstream schema fields. Shared registry edits remain explicit and serialized.
- Alternatives: the old monolith delays observable integration; adapters alone cannot supply all Kiro policy. The selected split retains a common contract without coupling feature acceptance.

### Requirements and design review gate

Inline reviewer checked numeric criteria, EARS shape, criterion traceability, component-to-file mapping, boundary commitments, dependencies, first deliverable and negative/error paths. Local repair pass resolved output-destination conflicts, planned-versus-executed evidence, default required metric coverage and enum ambiguity. Mechanical checks and current spec-gate results are recorded in code-constraints/design-validation.md after finalization.

### Risks and validation hooks

Prototype results are not available and no external dependency has been added. G1 must fail visibly if the required semantics or packaging cannot be met. G2 provides a real-binary slice before broad integration; G3/G5 prohibit false regression/acceptance claims; G4/G6 establish corpus/performance/platform evidence. Current design review does not certify runtime readiness.
