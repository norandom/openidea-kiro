# Implementation Plan

- [x] 1. Foundation: engine integration, fixtures, text utilities, guard language, annotation reader
- [x] 1.1 Integrate the checks seam into the engine crate
  - Prerequisite: `crates/kiro-core` (kiro-core spec) with the vocabulary module and the committed seed vocabulary (vocabulary-grounding spec) is implemented and builds for native and the sandbox target; `plugins/kiro` (plugin-core spec) builds its component so the size baseline below can be taken
  - Record the current size of the built plugin component as the baseline before the engine gains the decision-diagram dependency; the figure is reported by task 7
  - Add the decision-diagram library (`biodivine-lib-bdd = "0.6"`, default features) and the parser-combinator library (`winnow = "0.7"`) as runtime dependencies, and declare the checks module as its documented module root plus one documented empty stub per file of the File Structure Plan (text, ears, annotations, guard, terms, finitize, bdd, contradiction) so later tasks own their files
  - Append the twenty check finding codes (including `CHECK_COVERAGE`) to the finding catalogue and the constants module with the default severities and descriptions of the design's Supporting References; extend the crate documentation's code table and add the "requirements.md annotations" grammar subsection so the existing doc test keeps documentation and catalogue in agreement
  - Explicit integration touches: verify the kiro-core design document's Allowed Dependencies line already names the decision-diagram library and `winnow` (checks only) and that the technology steering document already records the decisions in place of the conditional sentences (amended during cross-spec review)
  - Observable: native build, sandbox-target build, clippy with warnings denied, fmt check and the existing catalogue tests pass; the dependency tree for the sandbox target shows no filesystem-dependent crate; the baseline size is noted
  - _Boundary: CoreIntegration_
  - _Requirements: 1.8, 8.1, 8.3_

- [x] 1.2 (P) Create the check fixtures
  - The known-good tree: a vocabulary for topic `ops` (enum, int with a lower bound only, bool, entity with an alternative label, a deprecated term with replacement), a roadmap, and spec `clean` declaring `ops` whose requirements document uses every EARS pattern, declares terms on requirements and on one criterion, carries disjoint guards, effects and one declared conflict, and declares every term whose label it mentions (producing zero warnings/errors and only the single `CHECK_COVERAGE` info finding under the default configuration)
  - The known-bad tree: a vocabulary with topics `ops` (plus a second "Level" label), `pm` (another "Level" label), `hr`, and `wide` (sixty-five enum terms of eight values, produced once by a small script and committed); spec `broken` (topics `ops`, `pm`) producing every code of this spec except the refusal and the unscoped finding (including `CHECK_COVERAGE`), with the two contradiction pairs and witnesses the design fixes; spec `wide` exceeding the default variable cap with one declared conflict (producing `SEARCH_REFUSED` and `CHECK_COVERAGE`); spec `none` without topics but with annotations (producing `TOPICS_UNDECLARED` and `CHECK_UNSCOPED`, with no `CHECK_COVERAGE` because neither the term nor the contradiction check runs)
  - The EARS regression document: the version-1 false positives (quoted modal, hyphenated compound, possessive apostrophe, parenthetical comma in the response, both `If` forms) that must produce no finding, and one criterion per EARS code
  - Each requirements fixture carries a header comment listing the expected codes per criterion id (plus document-level findings), matching the design's finding catalogue and witness strings
  - Observable: the files exist at the planned paths and the expected-findings comments match the design row for row
  - _Boundary: CheckFixtures_
  - _Requirements: 1.8, 2.10, 8.5_

- [x] 1.3 (P) Implement criterion text masking and word scanning
  - Masking of balanced quoted literals (straight and typographic double quotes, backticks, single quotes that are not inside a word) preserving byte length, leaving unbalanced quotes unmasked and treating word-internal apostrophes as part of the word; word enumeration; whole-word case-insensitive search; hyphen adjacency; whole-word phrase containment for normalised multi-word labels
  - Observable: unit tests cover balanced and unbalanced quotes, typographic pairs, backticks, possessives, length preservation, boundary matches, hyphen adjacency and multi-word phrases
  - _Boundary: CriterionText_
  - _Depends: 1.1_
  - _Requirements: 2.2, 2.10, 3.7_

- [x] 1.4 (P) Implement the guard and effect language
  - Combinator lexer and parser over `winnow` for the design's grammar: or/and/not with precedence, parentheses, bare term references, comparisons with the six operators where `=` and `==` both mean equality, integer, boolean, quoted and bare-word literals, kebab-case words, negative numbers; empty guard text means unconditional; effect lists of equality assignments; explicit nesting-depth cap (`MAX_GUARD_DEPTH = 64`) for the wasm stack; every error carries the design's leading phrase and a byte position
  - Observable: unit tests cover precedence, parentheses, both equality spellings, negative integers, quoted and bare words, effect lists, nesting depth above 64, and every error message with its position
  - _Boundary: GuardLanguage_
  - _Depends: 1.1_
  - _Requirements: 4.1, 4.4, 4.8_

- [x] 1.5 (P) Implement the annotation reader
  - A typed view over a parsed requirements document for the four keys (matched case-insensitively): declared-term lists on requirements and criteria (split on commas, trimmed, deduplicated after label normalisation), the first guard, effect and conflict list per criterion with later duplicates and requirement-level guard/effect/conflict annotations kept aside for reporting, the "any annotation present" flag and the first annotation line
  - Observable: unit tests cover requirement versus criterion placement, duplicates, misplaced annotations, the flag and first line, and list splitting with deduplication
  - _Boundary: AnnotationReader_
  - _Depends: 1.1_
  - _Requirements: 1.6, 3.1, 4.9_

- [x] 2. Core checks: EARS shape and declared terms
- [x] 2.1 Implement the EARS shape check
  - Analysis of one criterion over its masked text: shall count, offending modals outside hyphenated compounds, leading condition keyword, last comma before the first shall, the `then` form, subject and response from the unmasked text, and the pattern classification (ubiquitous, event, state, unwanted, optional, complex) that is always computed for a single-shall criterion
  - The document-level check producing the six EARS codes at the criterion line, pattern reports only when enabled, and the summary of criteria per pattern
  - Observable: unit tests produce each EARS code from constructed criteria, reproduce the version-1 false-positive cases without a finding, accept both `If` forms, detect complex conditions, and show pattern reports off and on
  - _Boundary: EarsCheck_
  - _Depends: 1.3_
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10_

- [x] 2.2 (P) Implement term scope resolution and the term check
  - Resolution of every declared name through the vocabulary's scoped resolution with exactly the spec's declared topics, kept per requirement and per criterion with the outcome; the term-reference rule for guards and effects (resolved id, or the declared name with whitespace replaced by hyphens, case-insensitively); the found-id set per requirement for the summary
  - The findings for deprecated, mismatched, unknown and ambiguous declared terms at their annotation line with the design's messages; term suggestions once per requirement and term, excluding terms declared on the requirement or on the criterion where the label occurs; the unscoped finding helper; the terms summary counts
  - Observable: unit tests cover each resolution outcome and its code, the reference rule for ids, hyphenated labels and case, suggestions bounded to one per requirement and term with per-criterion exclusion, and the unscoped finding text; no code path resolves a name without the topic list
  - _Boundary: TermCheck_
  - _Depends: 1.3, 1.5_
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 4.2_

- [x] 3. Contradiction search
- [x] 3.1 Implement finitisation: typed guards, domains and conflicts
  - Per criterion with any of guard, effect or conflicts: parsing errors, undeclared references, unresolved declared terms (excluded silently), kind rules for bool, enum, int and entity terms, bare-word literals that name a declared term (literal constants only), duplicate and misplaced annotations, unknown conflict ids; criteria without annotations that are named by an accepted conflict list are prepared as unconditional
  - Domains for every term used by an accepted criterion in topic-then-id order: bool, enum values in declaration order, integer regions cut at the constants mentioned by accepted guards and effects within the declared bounds; the brute-force evaluator of a typed guard over a state, for tests
  - Observable: unit tests cover the kind table, term-versus-term rejection, regions with and without bounds, the domain order, the referenced-unconditional rule and each finding code of this task
  - _Boundary: Finitization_
  - _Depends: 1.4, 1.5, 2.2_
  - _Requirements: 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 5.2, 5.6_

- [x] 3.2 Implement the decision-diagram engine
  - One variable set per encoding with one-hot blocks declared contiguously in domain order; the variable cap clamped at the design's ceiling and checked before any library call; the exactly-one domain constraint, guard encoding, conjunction, satisfiability and witness decoding all through the library's limited operation with the node cap; refusals naming the cap and stage; the largest-blocks helper
  - Observable: unit tests show contiguous blocks, the domain forbidding two values of one term, variable and node refusals on tiny caps, witness decoding to term values, and a domain node count that stays linear in the number of blocks
  - _Boundary: BddEngine_
  - _Depends: 3.1_
  - _Requirements: 5.2, 5.6, 5.7, 5.8_

- [x] 3.3 Implement pair selection and the contradiction search
  - Incompatible pairs from declared conflicts (symmetric) and clashing effects on one term, one per unordered pair in document order; the search building the encoding, encoding each accepted guard, reporting unsatisfiable guards, deciding each pair and reporting contradictions at the first criterion with the reason and the witness in topic-then-id order (or "in every state"); one refusal finding without a line listing the unchecked pairs and the largest terms; the search summary
  - Observable: unit tests cover declared and effect pairs, disjoint guards, an unconditional pair, an unsatisfiable guard, refusal listing, and the exact message forms of the design
  - _Boundary: ContradictionSearch_
  - _Depends: 3.1, 3.2_
  - _Requirements: 4.10, 5.1, 5.3, 5.4, 5.5, 5.7_

- [ ] 4. Runner and engine tests
- [ ] 4.1 Implement the check runner, configuration and report
  - Configuration with the design's defaults, the single-check constructor, severity overrides limited to this spec's codes; the report with per-check status, summaries, topics and vocabulary status; orchestration in the design's order: missing requirements document, EARS, the scoped decision with skip reasons in the stated precedence, pass-through of exactly the four upstream findings once, the unscoped finding when annotations are present, term scope and findings, finitisation and search, one `CHECK_COVERAGE` info finding (`line: None`) on the requirements document whenever the term or contradiction check ran carrying the counts of requirement 1.8, overrides, sorting
  - The workspace form loading the vocabulary before the feature so one cached read serves the call
  - Observable: unit tests cover skip reasons and their precedence, single pass-through, `CHECK_COVERAGE` emission when terms or contradictions ran and its absence when only EARS ran or when both were skipped, override application only to this spec's codes, the single-check constructor, and equality of two reports from equal inputs
  - _Boundary: CheckRunner_
  - _Depends: 2.1, 2.2, 3.3_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 3.8, 3.9_

- [ ] 4.2 Write the fixture-driven and self-corpus engine tests
  - Fixture-driven tests through an in-memory source and workspace: the known-good tree with zero warning/error findings and only `CHECK_COVERAGE` under the default configuration (plus pattern reports with patterns on); the known-bad specs (`broken` and `wide`) with their exact code multisets (including `CHECK_COVERAGE`) and the two witness strings; the refused spec; the unscoped spec (`none`, with `TOPICS_UNDECLARED` + `CHECK_UNSCOPED` and no `CHECK_COVERAGE`); the trees without and with a corrupted vocabulary; the regression document; single-check statuses; a severity override
  - The self corpus: this repository's own requirements documents, spec metadata and the committed seed vocabulary embedded through a manual list, asserting no warning or error about a requirements document from the term or contradiction checks; determinism across two runs of every fixture
  - Observable: the test file passes natively with every fixture's expected-findings header reproduced exactly
  - _Boundary: CoreCheckTests_
  - _Depends: 1.2, 4.1_
  - _Requirements: 1.6, 1.8, 8.2, 8.6_

- [ ] 4.3 Write the generated-model cross-check, no-panic and timing tests
  - A fixed-seed generator of small models (at most twelve one-hot values) whose decision-diagram verdicts are compared with brute-force enumeration through the typed-guard evaluator on sixty-four models; fixed-seed garbage requirements documents and guard strings through the runner and the guard parser never panic
  - Release-only timing: the EARS and term checks over one thousand generated criteria, and the search over two hundred generated guarded criteria on thirty-two terms (enum terms with up to eight values, int terms split at up to eight constants) within the default caps, each under one second
  - Observable: the tests pass natively, the timing assertions are compiled only in release builds, and the engine library still builds for the sandbox target
  - _Boundary: CoreCheckTests_
  - _Depends: 4.1_
  - _Requirements: 8.1, 8.2, 8.4_

- [ ] 5. (P) Write the checks documentation
  - The four annotations with placement (requirement block, or indented directly under the criterion before any blank line), the guard grammar, the term-reference rule, kinds and literal forms, the integer region rule, the incompatibility rule, caps with the meaning of a refusal and the levers that narrow a search, the finding code table with default severities (all twenty codes including `CHECK_COVERAGE`), the worked example from the known-bad fixture with its witness, the statement that a document without annotations produces no term or contradiction failure, the same-topic-only blind spot of the contradiction search, `CHECK_COVERAGE` as the signal that distinguishes "checked, nothing found" from "not checked", and the proposed requirements-template change as commented hints, not applied
  - Observable: the document exists at the planned path with every section above and its code table equals the design's table row for row
  - _Boundary: ChecksDocs_
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 6. Plugin: the `check` feature module
- [ ] 6.1 Implement the check feature module and the four commands
  - Prerequisite: `plugins/kiro` is implemented per the plugin-core spec (feature-module contract, command context, presentation, fake host, manifest test, build script) and the `../openidea` checkout providing the host WIT is present for the sandbox build; explicit integration touch: the crate-root module declaration
  - Command declarations (spec, ears, terms, contradictions; all spec-addressable with the spec as first argument token; spec insert-capable) and the feature-module implementation dispatching on the command name
  - Report execution: resolve the spec with plugin-core's error passed through, run the workspace form of the checks with the per-command configuration (all checks with patterns off; EARS with patterns on; terms only; contradictions only); one debug log line per call
  - Rendering: the title with counts or "no findings", the scope line, one status line per selected check (ran with counts, or skipped with reason), the patterns line for the EARS command, the caps in the contradictions line, the documentation pointer when a check was skipped for a vocabulary or topics reason, the findings in the shared text form or an explicit "No findings." body; insertion of the complete text when the insert flag is set
  - Observable: the plugin crate compiles natively with the module declared; unit tests cover the title, scope line, status lines, pointer, and rendering with and without findings
  - _Boundary: CheckCommands, PluginIntegration_
  - _Depends: 4.1_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.9_

- [ ] 6.2 Register the module in the plugin package
  - Insert `check::Check` at its canonical position in the plugin's module table (`ws, nav, viz, vocab, check`); the manifest gains the five static declarations in registry order with the design's titles; the README documents the four commands, the programmatic argument form, the per-spec variants and the degradation
  - Explicit integration touches: verify the plugin-core design document's Allowed Dependencies line already covers the engine's checks module (each feature module declares its own kiro-core imports in its design) and that its command budget worked example already carries the full figures (25 + 17·n), both amended during cross-spec review
  - Observable: the existing manifest consistency test passes with the five new declarations and the existing activation test registers exactly those
  - _Boundary: PluginIntegration_
  - _Depends: 6.1_
  - _Requirements: 6.8, 8.7_

- [ ] 6.3 Write the plugin end-to-end tests for the check commands
  - Simulated host serving the check fixture trees: the spec command on the clean spec (`0 errors, 0 warnings, 1 info` with `CHECK_COVERAGE`, info level, three status lines) and on the broken spec (error level, counts in the title, contradiction lines); the EARS command's patterns line and pattern reports; the terms command's counts and mismatch; the contradictions command's pairs line and witness; the unscoped spec's skipped lines, unscoped finding and pointer; missing and invalid vocabulary across the commands completing successfully with the expected levels; insert producing one insertion event with the full text; the unresolved-spec error identical to plugin-core's; the vocabulary path read at most once per command
  - Observable: the test file passes natively alongside the existing dispatch, vocabulary and manifest tests
  - _Boundary: PluginCheckTests_
  - _Depends: 1.2, 6.2_
  - _Requirements: 6.9, 8.7_

- [ ] 7. Validation: both targets, lints, size and the host check
  - Prerequisite: the dev-plugin installation of plugin-core (junction, trust, Reload Plugins) is in place in the open project
  - Engine and plugin build for the sandbox target with the decision-diagram library present; clippy with warnings denied and fmt check pass for the root workspace and the plugin package; the dependency tree for the sandbox target is re-checked
  - Manual host check as a dev plugin: with this spec's requirements document open, the spec command shows three status lines and the EARS command shows the patterns line; with the vocabulary-grounding requirements document open (that spec declares a topic), the contradictions command shows the caps line; with this spec's document open it shows the skipped line with its reason (when no GUI host is available, the manual check is recorded as deferred in the completion note — matching `spec-navigation` 4.3 and `spec-visualization` 4)
  - Append a "Check commands: verification" section to the plugin README recording the component size before (baseline from 1.1) and after this spec and the host check outcome (or deferred note when no host is running)
  - Observable: all build and lint commands above succeed and the README section exists with both figures and the recorded outcome
  - _Boundary: PluginIntegration_
  - _Depends: 4.3, 6.3_
  - _Requirements: 8.1, 8.2, 8.7_
