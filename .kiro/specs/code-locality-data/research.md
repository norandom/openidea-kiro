# Research & Design Decisions

## Design revision: bounded deliverables (2026-11-09)

### Summary

Owns scattered definitions/state/schema and vocabulary-backed named data access. Consumes confined parsing, coverage, ownership and vocabulary resolution. Does not infer semantic equivalence, implement dependency graphs or extend IDE behavior.

The user instructed applying the preceding review and proceeding to designs. Requirements are approved for this design step on that basis; design approval and implementation remain pending. The original 108 criteria have explicit successors in split-map.md; added safeguards are new numeric criteria.

### Discovery and consulted guidance

Integration-focused discovery reuses the preceding repository/tool review. Consulted kiro-spec-requirements (EARS and requirements-review-gate), kiro-spec-design (design-principles, light discovery, synthesis and design-review-gate), core product/tech/structure steering and foreground workflow. Review performed inline; no subagent or model-specific implementation request was invoked.

- Existing vocabulary-grounding Lookup and code ownership contracts remain authoritative; sibling scattered definitions are textual grouping.
- [ast-grep introduction](https://ast-grep.github.io/guide/introduction.html) documents structural extraction. Compare it with direct tree-sitter on the bounded corpus; neither supplies semantic state ownership or vocabulary policy.

### Synthesis and decisions

- Generalization: plain typed observations consumed by pure policies and the existing report/evidence contracts.
- Build versus adopt: reuse existing parsers, joins, fixture helpers and documented libraries before adding custom machinery; the first prototype pins dependencies and records per-behavior adoption or rejection before broad implementation.
- Simplification: own only this spec's behavior; no duplicated filesystem walks, evidence frameworks or downstream schema fields. Shared registry edits remain explicit and serialized.
- Alternatives: the old monolith delays observable integration; adapters alone cannot supply all Kiro policy. The selected split retains a common contract without coupling feature acceptance.

### Requirements and design review gate

Inline reviewer checked numeric criteria, EARS shape, criterion traceability, component-to-file mapping, boundary commitments, dependencies, first deliverable and negative/error paths. Local repair pass resolved output-destination conflicts, planned-versus-executed evidence, default required metric coverage and enum ambiguity. Mechanical checks and current spec-gate results are recorded in code-constraints/design-validation.md after finalization.

### Risks and validation hooks

Prototype results are not available and no external dependency has been added. G1 must fail visibly if the required semantics or packaging cannot be met. G2 provides a real-binary slice before broad integration; G3/G5 prohibit false regression/acceptance claims; G4/G6 establish corpus/performance/platform evidence. Current design review does not certify runtime readiness.

## Implementation record: scattered definitions and scattered state (2026-11-10, night)

- **L0 engine choice.** Extraction walks the `ast-grep` tree that the metrics and the duplicate search already use, with one table of node kinds per language (`crates/kiro-code/src/locality.rs`). No pattern language is exposed and no second engine was added. Task 1.2 asked for a recorded comparison of direct queries and `ast-grep` patterns on a fixture corpus; that comparison was not run, so 1.2 stays unchecked.
- **One parse for the project-wide rules.** `crates/kiro-code/src/scan.rs` parses every measurable file once and feeds both the code lines of the duplicate search and the locality facts.
- **Rules** (`crates/kiro-core/src/code/locality.rs`, pure): scattered definition (name and initializer in more than 3 files, warning, `collect:`), ambiguity information when one name has several scattered values, and scattered state (a subject with at least 2 literal values across more than 3 files, information, `collect:`).
- **Severity decision.** The design shipped scattered state as a warning. It is information here, following review item C4: the subject is its spelling, so unrelated variables of one name are merged. On this repository the first run showed the problem at once: `self` was "compared with 52 literal values in 14 files", because every `match self` was one subject. The receiver is now excluded.
- **Tests.** Hand-decided: 8 rule tests in the core, 5 extraction tests for Rust, Python and TypeScript in the native crate, 5 journeys through the binary (a constant written a fourth time fails and passes after it is defined once and imported; a state spelled in four files is reported and gone after the decisions move into one enumeration). All passed on the first run.
- **On this repository**: 310 module-level definitions and 589 literal comparisons in 161 files; 4 scattered-definition findings (`ROADMAP_PATH` in four files) and 60 scattered-state findings, among them `cmd.name` compared with 18 literal strings in 10 files, which is the command dispatch of the plugin.
- **Not done**: settings in the configuration (1.3), extraction confidence states (2.1), enumeration exhaustiveness (4.1, 4.2 in part), scattered schema (4.3), data access patterns and vocabulary grounding (5.x, 6.x), accuracy and cost records (7.x). Tasks checked: 2.2, 3.1, 3.3.
