# Research & Design Decisions

## Design revision: bounded deliverables (2026-11-09)

### Summary

Owns shared parameter groups, command capability profiles, compatible migration, output routing and exit conventions. Does not own individual code rules, evidence semantics or external analyzers.

The user instructed applying the preceding review and proceeding to designs. Requirements are approved for this design step on that basis; design approval and implementation remain pending. The original 108 criteria have explicit successors in split-map.md; added safeguards are new numeric criteria.

### Discovery and consulted guidance

Integration-focused discovery reuses the preceding repository/tool review. Consulted kiro-spec-requirements (EARS and requirements-review-gate), kiro-spec-design (design-principles, light discovery, synthesis and design-review-gate), core product/tech/structure steering and foreground workflow. Review performed inline; no subagent or model-specific implementation request was invoked.

- Existing source: crates/kiro-cli/src/cli.rs, exit.rs, main.rs, viz.rs and import/mod.rs; tests/common/mod.rs.
- [clap derive documentation](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html): existing derive/flatten supports reusable groups. Current source uses distinct InputFormat for vocab import. The installed help was checked during the preceding review.

### Synthesis and decisions

- Generalization: shared applicable parameter groups with explicit command profiles.
- Build versus adopt: reuse existing parsers, joins, fixture helpers and documented libraries before adding custom machinery; no new analyzer is introduced.
- Simplification: own only this spec's behavior; no duplicated filesystem walks, evidence frameworks or downstream schema fields. Shared registry edits remain explicit and serialized.
- Alternatives: the old monolith delays observable integration; adapters alone cannot supply all Kiro policy. The selected split retains a common contract without coupling feature acceptance.

### Requirements and design review gate

Inline reviewer checked numeric criteria, EARS shape, criterion traceability, component-to-file mapping, boundary commitments, dependencies, first deliverable and negative/error paths. Local repair pass resolved output-destination conflicts, planned-versus-executed evidence, default required metric coverage and enum ambiguity. Mechanical checks and current spec-gate results are recorded in code-constraints/design-validation.md after finalization.

### Risks and validation hooks

Prototype results are not available and no external dependency has been added. G1 must fail visibly if the required semantics or packaging cannot be met. G2 provides a real-binary slice before broad integration; G3/G5 prohibit false regression/acceptance claims; G4/G6 establish corpus/performance/platform evidence. Current design review does not certify runtime readiness.

## Implementation record: C0 and C1 (2026-11-10)

- Compatibility records were captured from the binary before any change: 18 invocations of check, status, graph, tasks and vocab validate in `crates/kiro-cli/fixtures/compat/`, compared by `tests/cli_standard.rs`. All 18 are unchanged after the migration of `check`.
- `crates/kiro-cli-common` holds the exit model, the global, report and gate groups, environment resolution and report routing. `exit.rs` re-exports the moved types; no caller path changed.
- Environment precedence is resolved by the library from a captured value set, not by clap's `env` attribute, because an invalid value must name its variable and must not echo other values, and because tests can then pass values without changing the process environment.
- Behaviour added, none removed: `-v` (repeatable) beside `--verbose`, `-q/--quiet`, `--output <PATH>` on check, and the `KIRO_PROJECT`, `KIRO_FORMAT`, `KIRO_FAIL_ON`, `NO_COLOR`, `CLICOLOR_FORCE` fallbacks. Help text for `--project`, `--color`, `--format` and `--fail-on` now states the default and the variable in the description instead of clap's `[default: ...]` suffix, because the fields became optional to tell an explicit value from a default.
- `docs/cli.md` listed `--fail-on <error|warning|info|never>`; only `error` and `warning` ever existed. Corrected.
- Verification on Windows: 412 workspace tests pass, clippy with warnings denied, format check, `kiro-core` wasm32-wasip2 build, `cargo deny check`, and the self check all pass. Linux and macOS runs belong to task 5.2 and were not run.
- Colour is resolved but nothing prints colour yet; quiet is resolved but only verbose changes output today.
