# Research & Design Decisions

## Design revision: bounded deliverables (2026-11-09)

### Summary

Owns duplicate blocks, similar-routine families, clone-source adapters and clone quality/performance acceptance. Consumes measurement, coverage, ownership and report contracts; does not own metric baselines or semantic-equivalence claims.

The user instructed applying the preceding review and proceeding to designs. Requirements are approved for this design step on that basis; design approval and implementation remain pending. The original 108 criteria have explicit successors in split-map.md; added safeguards are new numeric criteria.

### Discovery and consulted guidance

Integration-focused discovery reuses the preceding repository/tool review. Consulted kiro-spec-requirements (EARS and requirements-review-gate), kiro-spec-design (design-principles, light discovery, synthesis and design-review-gate), core product/tech/structure steering and foreground workflow. Review performed inline; no subagent or model-specific implementation request was invoked.

- ../scitools-hook/src/scitools_hook/analysis/lean/similar.py:675–726,811–820 establishes canonical pair order, post-filter reachability and autojunk=False.
- [jscpd engine crates](https://jscpd.dev/reference/rust-crates) document core/tokenizer reuse. [Similarity](https://jscpd.dev/guides/similarity) uses a different AST scoring definition; do not treat it as token-family parity.
- Documentation examples refer to 5.4.0 while similarity describes 5.4.1. Prototype must pin and exercise actual compatible artifacts.

### Synthesis and decisions

- Generalization: plain typed observations consumed by pure policies and the existing report/evidence contracts.
- Build versus adopt: reuse existing parsers, joins, fixture helpers and documented libraries before adding custom machinery; the first prototype pins dependencies and records per-behavior adoption or rejection before broad implementation.
- Simplification: own only this spec's behavior; no duplicated filesystem walks, evidence frameworks or downstream schema fields. Shared registry edits remain explicit and serialized.
- Alternatives: the old monolith delays observable integration; adapters alone cannot supply all Kiro policy. The selected split retains a common contract without coupling feature acceptance.

### Requirements and design review gate

Inline reviewer checked numeric criteria, EARS shape, criterion traceability, component-to-file mapping, boundary commitments, dependencies, first deliverable and negative/error paths. Local repair pass resolved output-destination conflicts, planned-versus-executed evidence, default required metric coverage and enum ambiguity. Mechanical checks and current spec-gate results are recorded in code-constraints/design-validation.md after finalization.

### Risks and validation hooks

Prototype results are not available and no external dependency has been added. G1 must fail visibly if the required semantics or packaging cannot be met. G2 provides a real-binary slice before broad integration; G3/G5 prohibit false regression/acceptance claims; G4/G6 establish corpus/performance/platform evidence. Current design review does not certify runtime readiness.

## K0 record and first duplicate slice (2026-11-10, night)

Prototype outside the repository: `D:\BuildArtifacts\openidea-kiro\k0-clone`.

| Candidate | Finding | Decision |
|---|---|---|
| `cpd-tokenizer` 0.1.22 | 122 crates; its own tokenizers per language (regular expressions and `syn`), no tree-sitter | Not adopted. It would be a second lexing stack next to the syntax tree `kiro-code` already has |
| `cpd-core` 0.1.23 | 18 crates; token-based detection (`detect_prepared`, `PreparedSource::from_detection_tokens`) with a minimum in tokens | Not adopted for the first rule: the requirement is a run of 12 normalized lines compared by hash. Read from its source, not run on a corpus, so task 1.2 stays unchecked |
| `difflib-fast` 0.6.0 (MIT) | `ratio` and `cluster_canonicals` (exact single-linkage with the minimum pairwise ratio); 7 crates including rayon; contains `unsafe` | Candidate for the family rule (K2) |
| `difflib` 0.4.0 (MIT) | Plain port of Python's SequenceMatcher, 1 crate, `f32` ratio | Fallback candidate for K2 |

Both ratio libraries reproduced eight hand-computed values on Windows and on Linux, including the asymmetric pair (`tide`/`diet` 0.25, `diet`/`tide` 0.5) and a 303-character repetitive pair (0.9967). `difflib-fast` clustered 600 shapes of 81 characters in 62 ms on Windows and 96 ms in the Linux container.

First slice built (tasks 2.2 and 3.2):

- `crates/kiro-code/src/clones.rs` takes the code lines from the leaves of the same `ast-grep` tree the metrics use: tokens per line, comments left out, hashed. No second tokenizer.
- `crates/kiro-core/src/code/clones.rs` (pure) hashes windows of 12 lines, confirms equal windows line by line, joins consecutive windows into maximal runs, excludes a run that only overlaps itself, and words one finding per selected range with up to three other locations. This window policy is the only algorithm written here; it is about 150 lines.
- `code check` searches the whole project and reports for the selected files; findings pass through the evaluation's extension slot, which adds ownership.
- Hand-decided tests: 7 in the core, 6 in the native crate, 5 through the binary (introduce a copy with other spacing and a comment, fail, remove, pass). All passed on the first run.
- On this repository: 96 blocks. The largest are `.openidea/plugins/kiro/`, an installed copy of `plugins/kiro/`. A real one: `plugins/kiro/src/check.rs` and `crates/kiro-cli/src/report.rs` share blocks of 54, 15, 13 and 12 lines.
- Cost: the search parses every measurable file again. Release build on this repository: whole tree 4.3 s (was 2.7 s), a task-scoped check 1.3 s (was 0.2 s). Sharing the parse between metrics and lines would remove most of that.

Open in this spec: clone settings in the configuration (3.1), the family rule (4.x), jscpd report import (5.x), accuracy and budgets (6.x), platform acceptance (7.x). The source walk does not honor `.gitignore`; the `ignore` crate is already in the dependency tree through `ast-grep-language` and could replace the hand-written walk.

## Implementation record: similar-routine families (2026-11-10, late night)

- **Library.** `difflib-fast` =0.6.0 is a dependency of `kiro-code`; `similarity` calls its `ratio`. No scorer is written. The library compares characters, so each distinct token gets one character (printable ASCII first, then upward).
- **Shapes** (`crates/kiro-code/src/shapes.rs`): leaves of the shared syntax tree in document order; identifiers become `$I`, string and number literals `$L`; comments and nested routines are skipped. One rule for all languages, by node-kind name (`identifier`, `*_identifier`, `*_literal`, `string`, `integer`, `float`, `number`, `template_string`, `regex`).
- **Pairs.** Per language, sorted by length; the length ceiling ends the partner search, the token-multiset ceiling refuses single pairs, identical shapes score 1.0 without a call. Budget 2,000,000 scored pairs, then `complete = false`.
- **Families** (`crates/kiro-core/src/code/families.rs`, pure): ignored names out, connected components, members nested in another member out, components again until stable; one finding per family at the first selected member.
- **Tests, hand-decided.** Core: 7 (chain, selection, same-file shrink, ignored bridge, nested member, nested bridge, stable order); all passed on the first run. Native: 9 shape and ratio tests plus 1 unit test for scoring, refusing and the spent budget; one expectation of mine was wrong (the closing line of a 13-line routine counted as 12). Exhaustive ceiling check over 66,430 pairs of strings. Binary: 3 journeys (renamed copy across files fails and passes after one routine is kept; same-file family as `shrink:`; short routines and another language are not compared); all passed on the first run.
- **Measured on labeled pairs** (requirement 1.6): rename 1.00; guard clauses to ladder 0.8571; `if` chain to table 0.6000.
- **On this repository**: 806 of 1670 routines compared, 643 pairs scored, about 320,000 refused, 18 families. Among them `status_line` in `crates/kiro-cli/src/report.rs` and `plugins/kiro/src/check.rs` (1.00) and `Parser::parse_or` / `Parser::parse_and` (1.00). On facdrone: 6631 of 17201 routines compared, 7256 pairs scored, 21.9 million refused, 111 families.
- **Speed.** The first run of the rule took the whole-tree check on this repository from 2.8 s to 7.7 s. The cause was not the scoring (48 ms) but the statement count per routine, which asked every node for its parent; the syntax tree library answers that by walking from the root. The walk now carries the one needed fact about the parent down (`tagged_children` in `measure.rs`). Reports before and after are byte-identical on mmhmem, lesscode, mvibe and facdrone. Measurement on this repository went from about 3.1 s to 1.3 s. Larger trees are still slow: facdrone takes 47 s for the whole tree (measurement 12 s, project scan 15 s).
- **Not done**: threshold, floor and ignored names in the configuration (3.1); a labeled accuracy corpus beyond the three pairs (6.1); jscpd import (5.x). Tasks checked: 4.1, 4.2, 4.3.

## Implementation record: rule settings (2026-11-11, night)

- `[rules.<name>]` in `.kiro/code-rules.toml` sets severity (`error`, `warning`, `info`, `off`) and the numbers of `duplicate_block`, `similar_family`, `scattered_definition`, `scattered_state` and `positional_access` (`crates/kiro-core/src/code/rules.rs`). A setting that reports less than the built-in one needs a reason; an unknown rule, a key of another rule and a value out of range are configuration errors. A rule that is off does not run and the coverage line says so (task 3.1).
- The cross-file judgement moved from `code.rs` to `crates/kiro-cli/src/cross.rs`.
- Default ignored routine names now follow the design: test hooks and every Python name with two underscores on each side, in addition to `main`, `new`, `default`, `fmt`.
- Tests, hand-decided: 6 for the settings in the core, 1 more for families, 1 journey through the binary (refused without a reason, off, a higher floor, error severity, `code config`). Two expectations of mine were wrong: the refusal is printed on the error stream, and the fixture routine has nine statements, not eight.
- Not done: locality path ignores per rule, data topics, shapes from the configuration (code-locality-data 1.3 stays open).
