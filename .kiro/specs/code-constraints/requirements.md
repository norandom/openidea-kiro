# Requirements Document

## Introduction

Measured Code Constraints. Owns comparable measurements, routine and container policy, the refactoring-safe ratchet, ownership attribution and the code CLI. Clone detection belongs to clone-constraints; locality and vocabulary-backed data rules belong to code-locality-data; CLI conventions and execution evidence are consumed contracts.

## Scope and Adjacent Expectations

Consumes: cli-contracts, verification-evidence, quality-telemetry (provider contract only). The owner authorized the reviewed amendments and progression to design on 2026-11-09. Each spec delivers and accepts its own behavior; original criterion identities are preserved in [the split map](../code-constraints/split-map.md).

## Requirements

### Requirement 1: Measurement Sources
**Objective:** As an operator, I want the Code Gate to measure code natively where it can and to accept reports from analyzers I already run, so that one gate covers the project's languages without a second configuration per tool.

#### Acceptance Criteria
1. The Code Gate shall measure routine, type and file metrics for Rust, Python and TypeScript/JavaScript through a bundled offline measurement provider built on adopted open-source parsing libraries with no hand-written language parser, documenting supported syntax, extensions and unsupported facts; no licensed service or runtime network access is required.
2. The Code Gate shall accept an external report only through a source adapter registered by its owning specification, naming the originating tool and rule on every imported finding and treating the report as partial observations rather than a complete measurement census; the base gate ships no report adapter of its own and clone report import is owned by clone-constraints.
3. When comparable numeric observations exist for the same entity and metric on both sides, the Code Gate shall permit an observation-level comparison, treating absent values as unknown and excluding partial observations from completeness claims and net totals.
4. If a report file cannot be read, is not valid JSON or does not match its declared format, the Code Gate shall report `CODE_SOURCE_INVALID` as an error naming the file and the first problem, continue with every other source, and count the failure in the coverage finding of the coverage contract.
5. When several sources supply one metric, the Code Gate shall select the configured provider consistently on both sides, prefer the bundled provider by default, report differing values and prohibit comparison across incompatible metric definitions or producer versions.
6. While a selected file is in a language no source covers, the Code Gate shall list it as not measured in the coverage finding and produce no finding about its content.
7. Where a source is marked `required` in the configuration and its report is absent or invalid, the Code Gate shall stop with an error naming the source and exit with code 2, so that a pipeline cannot pass by forgetting to run the analyzer.

### Requirement 2: Configuration, Defaults and Reasons
**Objective:** As a project maintainer, I want the limits, rule switches and exceptions in one committed file with measured defaults, so that the gate's judgement is reviewable and no agent can relax it silently.

#### Acceptance Criteria
1. The Code Gate shall read its configuration from `.kiro/code-rules.toml` under the project root, running with the built-in defaults when the file is absent.
2. The Code Gate shall document the evidence behind every shipped default and enable blocking only for metric definitions and languages calibrated by the prototype corpus, leaving uncalibrated metrics and rules as warning or off.
3. The Code Gate shall accept, per rule and per metric, a severity (`error`, `warning`, `info`, `off`) and a limit, name and path ignore lists where a rule defines them, and path scopes that override routine, type and file limits for the files they match, with the most specific matching scope winning and documented tie rules.
4. The Code Gate shall require a `reason` text on every scope, on every ignore entry and on every limit or severity looser than the shipped default, rejecting a configuration lacking one with `CODE_CONFIG_INVALID` naming the key and exit code 2 before measuring anything.
5. If the configuration names an unknown rule, metric, severity or key, a limit that is not a number, or a required source the Code Gate does not know, the Code Gate shall stop with `CODE_CONFIG_INVALID` naming the entry and exit with code 2.
6. When `code config` runs, the Code Gate shall print every effective limit, severity, scope, ignore entry and source with where it came from (default, file or scope) and its reason, so that what the gate enforces is inspectable without reading the file.

### Requirement 3: Routine Complexity and Size Limits
**Objective:** As a reviewer of agent-written code, I want every routine judged against complexity and size limits, so that a routine stays something a reader reasons about rather than approximates.

#### Acceptance Criteria
1. The Code Gate shall measure strict cyclomatic complexity, deepest nesting, code lines, statements and declared parameters per routine, using candidate limits 10, 3, 60, 40 and 5 respectively, with blocking severity enabled only after the metric-and-language calibration gate passes.
2. The Code Gate shall measure the lines per statement for routines of at least 5 statements and the comment lines of every routine, judging them against the defaults 4 and 20 as warnings, so that length without complexity is reported without blocking.
3. When a measured value exceeds its limit, the Code Gate shall report `CODE_ROUTINE_LIMIT` naming the routine, the metric, the value, the limit and the scope the limit came from, with a `simplify:` hint naming the remedy for that metric (guard clauses or extraction for complexity and nesting, extraction for length, an options record for parameters, fewer lines per statement or fewer comment lines for the shrink metrics).
4. The Code Gate shall judge every limit strictly, treating a value equal to its limit as passing.
5. Where a report supplies cognitive complexity, essential complexity or path count with a declared definition, the Code Gate shall evaluate configured limits and label the provider, without claiming these metrics are supplied by the bundled base profile.
6. When `--show-highest` is given, the Code Gate shall print for every routine, type and file metric the highest measured value and where it stands, so that the first run on a grown repository reads as a distribution and not as a verdict.

### Requirement 4: Container Limits
**Objective:** As a reviewer, I want files and types judged by how much they hold, so that a module that accretes everything is refused before it becomes the place every change touches.

#### Acceptance Criteria
1. The Code Gate shall measure each file's code lines, declared functions, declared types and maximum routine cyclomatic complexity, counting classes, structs, enums, traits, interfaces and unions but excluding type aliases, with candidate limits 500, 25, 8 and 10 respectively; declared-type count remains warning and other metrics become blocking only after calibration.
2. The Code Gate shall count each type's declared methods and fields, including Rust inherent implementation blocks in the same file, using candidate limits 20 and 10 respectively and enabling blocking only after calibration.
3. When a file or type exceeds a limit, the Code Gate shall report `CODE_FILE_LIMIT` or `CODE_TYPE_LIMIT` naming the container, the metric, the value and the limit, with a `split:` hint that names a cohesive group to move and, when the file is owned by a spec, the design component so the split is made in the File Structure Plan first.
4. The Code Gate shall never ratchet container counts, retaining absolute limits and applying pre-existing classification only when exact comparable before observations justify it.
5. The Code Gate shall accept coupling, cohesion and inheritance depth of types from a report source as any other type metric, with the documentation stating that they are not measured natively because they need name resolution.

### Requirement 5: Ratchet, Baseline and Net Growth
**Objective:** As an operator adopting the gate on a grown code base, I want only regression to block, so that the gate can be installed today and the trend becomes monotonic without a cleanup project.

#### Acceptance Criteria
1. The Code Gate shall identify every measured entity by a key that is stable across two measurements — scope, project-relative path, qualified name and parameter list — so that an entity is compared with itself on the before and the after side.
2. When an eligible comparable metric worsens, the Code Gate shall report CODE_RATCHET with before, after and limit, as an error above the limit and a warning within it, after applying the decomposition, parse-validity and metric-floor safeguards.
3. When a before side exists and an entity exceeds a limit on both sides without worsening, the Code Gate shall mark the limit finding as pre-existing, report it, and never let it block unless the configuration's strict mode is on.
4. The Code Gate shall take the before side from the committed baseline file `.kiro/code-baseline.json` when it exists, or from the git revision named by `--changed <rev>` when given (the named revision wins when both exist), saying in the output which one was used and reporting no ratchet finding and no pre-existing status when neither exists.
5. The Code Gate shall never apply the ratchet to the container counts of the container-limit requirement or to imported breach-only findings, with the rules reference saying so.
6. When exactly one routine key disappears and one appears within the same scope, path and qualified name, the Code Gate shall pair the signature change only to detect regression, never to excuse an absolute breach as pre-existing, leaving ambiguous matches unpaired and visible.
7. The Code Gate shall print signed statement and line deltas including deleted routines and old rename paths only when complete comparable routine measurements exist on both sides of the accounting scope, otherwise reporting the net total as unavailable with its reason, and reporting CODE_NET_GROWTH only for a configured maximum.
8. When `code baseline` runs, the Code Gate shall write the baseline deterministically (sorted keys, no timestamps, the effective configuration digest recorded) so that two captures of the same tree are byte-identical.
9. When baseline configuration, producer, metric definition, grammar, schema or scope identity differs, the Code Gate shall report the mismatch, compare only explicitly compatible observations, and refuse unsupported pre-existing or net-growth claims.

### Requirement 6: Spec Ownership and Task Selection
**Objective:** As a developer in a spec-driven project, I want every code finding attributed to the spec, component and tasks that own the file, so that degradation is read in the language of the specs and the fix lands in the right task.

#### Acceptance Criteria
1. When a finding concerns a file that a discovered spec's design File Structure Plan maps to a component, the Code Gate shall set the finding's feature to that spec and name the component and the tasks whose boundary names it in the finding, using the engine's ownership join and never re-deriving the mapping.
2. When a file is mapped by more than one spec, the Code Gate shall set the feature to the first spec in roadmap order (discovery order when the roadmap lists none) and name the others in the finding.
3. When a selected file with at least one finding is mapped by no spec, the Code Gate shall report `CODE_FILE_UNOWNED` at info level once per file per run, so that code outside every spec is visible.
4. When `--spec <name>` is given, the Code Gate shall judge only the files that spec's design maps, and when `--task <spec>:<id>` is given only the files of the components the task's boundary names, failing with exit code 2 and the available specs, tasks or components when the spec, the task or the boundary is unknown or the task has no boundary.
5. The Code Gate shall print a per-spec summary in the text and JSON reports — findings by severity, files measured, and the net line where a before side exists — ordered by roadmap order then name, with unowned files summarised last.
6. The Code Gate shall accept `--files <path>...` and `--all` as selections in the same way as `--spec` and `--task`, so that a pre-commit hook or a pipeline can select by path or by whole tree.

### Requirement 7: Trust Signals and Degradation
**Objective:** As a reviewer, I want to know what the gate did not measure, so that a clean report is never mistaken for a checked one.

#### Acceptance Criteria
1. Whenever a run completes, the Code Gate shall report CODE_COVERAGE naming selection, accounting scope, source coverage and completeness, measured and unavailable metrics, skipped or failed rules with reasons, active extension coverage and the before-side identity.
2. If a selected source file cannot be parsed into a syntax tree or contains an error node, the Code Gate shall report `CODE_PARSE_FAILED` as an error naming the first error position and stating that the file cannot be reported as checked, judging no rule on that file.
3. If a file outside the selection fails to parse, the Code Gate shall report a nonblocking diagnostic, exclude its facts and identify any project-wide analysis made incomplete by that exclusion.
4. When a rule is off or cannot run (no source supplies its facts, the language is not covered), the Code Gate shall report it as skipped in `CODE_COVERAGE` and produce none of its findings.
5. The Code Gate shall print and count warnings, information and pre-existing findings, applying --fail-on to eligible findings while honoring strict-mode inclusion of pre-existing breaches and explicitly nonblocking outside-selection diagnostics.
6. The Code Gate shall document per rule what it deliberately does not report, stating in the rules reference and in the steering block that whether the standard library, the platform or an installed dependency would have replaced a piece of code is never checked, so that their absence from a report is not a clearance.

### Requirement 8: Agent Feedback Contract
**Objective:** As the operator of an agent harness, I want the limits in the agent's context before it writes code, every finding to carry an edit it can make, and the gate to be unrelaxable by the agent, so that agents follow the rules instead of learning them from refused commits.

#### Acceptance Criteria
1. When `code rules --write` runs, the Code Gate shall write the effective rules as a block into `.kiro/steering/code-rules.md` between a begin and an end marker, replacing a previous block and preserving the rest of the file, generated from the effective configuration so that it cannot drift, and listing the limits per scope, the rules in force with their hint tags, the lean ladder with the rungs that are the agent's alone, the ratchet statement and the per-task self-check command.
2. The Code Gate shall open every finding's hint with exactly one tag from `simplify:`, `split:`, `delete:`, `shrink:`, `collect:`, `name:` and `fix:` (the last reserved for findings about the run itself: parse failures, invalid sources or configuration, a stale baseline, coverage), followed by what to change and what replaces it in one line, carrying the hint and the tag as separate fields in the JSON report.
3. The Code Gate shall provide a JSON report in which every finding carries severity, code, feature, file, line, message, hint, tag, metric, value, before value, limit, limit source, pre-existing flag, entity key and owning component and tasks, under a schema version, with no timestamps and a deterministic order, so that a harness can inject it into an agent's context and two runs over the same input are byte-identical.
4. The code-gate workflow shall require a task-scoped check before completion and resolution of blocking findings through code changes, prohibiting an implementing agent from relaxing configuration, refreshing the baseline or expanding ignores to obtain a pass.
5. The code-gate validation workflow shall require a whole-tree check against the reviewed baseline and current execution evidence for the required code journeys.
6. The Code Gate shall document a human-owned adaptation procedure in which a limit is loosened, a scope added or a finding ignored only through a configuration change carrying a reason, reviewed by a human, followed by regenerating the steering block, with the `agent-guidance` steering naming the Code Gate as the code half of the feedback contract.

### Requirement 9: Command Line, Exit Codes, Determinism and Performance
**Objective:** As a CI maintainer, I want the code gate run by one command with the exit-code contract of the spec gate, so that a pipeline blocks on code degradation exactly as it blocks on spec findings.

#### Acceptance Criteria
1. The kiro-cli shall offer `code check` with the selections `--all`, `--changed <rev>`, `--spec <name>`, `--task <spec>:<id>` and `--files <path>...`, the options `--format text|json`, `--fail-on error|warning`, `--show-highest` and `--source <format>=<file>` (repeatable), and the companions `code baseline`, `code rules [--write]` and `code config`.
2. The kiro-cli shall use the shared exit contract: 0 for no eligible finding at --fail-on, 1 for an eligible finding and 2 for usage, configuration, filesystem or required-source failure, with diagnostics on standard error.
3. Where `--sarif <path>` is given, the Code Gate shall write the findings as a SARIF 2.1.0 document to that path beside the chosen report, with the rule identifiers equal to the finding codes and the hint carried as the result message's remediation text.
4. The Code Gate shall produce byte-identical text and JSON reports for the same tree, configuration and sources across runs and platforms: project-relative paths with `/` separators, newline-only line endings, no absolute paths, no timestamps, findings sorted by the engine's order.
5. The Code Gate shall complete task-scoped checks over this repository within 10 seconds and whole-tree checks within 60 seconds in a release build on a recorded development-machine and corpus identity, reporting incomplete analysis when a bounded-work limit prevents a full result.
6. The Code Gate shall confine source reads to the project root, read external reports only when explicitly named, and write only an explicitly requested output, baseline, steering block or SARIF artifact.

### Requirement 10: Self-Application, Fixtures, Documentation and Tests
**Objective:** As a maintainer of this repository, I want the gate to pass on its own code and to be documented with fixtures, so that the rules are credible and the measurements behind the defaults are recorded.

#### Acceptance Criteria
1. The repository shall gate itself: the CI workflow runs `kiro-cli code check --all --fail-on error` against a committed baseline through a `cargo kiro-code` alias, and the first inventory of this repository (findings by rule and the highest values) is recorded in the research log with the note that the first number is not a quality score.
2. The repository shall provide expected-result fixtures for metric and container limits, decomposition and signature-change ratchets, invalid or missing measurements, renames and deletions, ownership, parsing and source failures; extension-specific fixtures belong to their owning specs.
3. The repository shall document rules, defaults and blind spots, configuration and CI usage, and a frozen feature inventory of the archived scitools-hook project assigning every rule and technique to an implemented, adapted, deliberately different, deferred or not-applicable disposition with an owner or reason.
4. The repository shall provide unit tests for every rule over the fixtures, a property test that the native measurer never panics on arbitrary text, determinism tests comparing two renderings, exit-code tests for every code, release-only timing tests for the performance requirement, and a check that the engine's sandbox-target build remains unaffected because measurement is native-only.
5. The new code shall pass `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check` and follow the engine's lint table (unsafe code forbidden; `unwrap`, `expect`, `panic`, indexing, `todo` and `unimplemented` denied).

### Requirement 11: Comparable Facts and Safe Ratchet Decisions
**Objective:** As an adopter, I want measurement uncertainty kept distinct from code regression so that useful refactoring remains possible.

#### Acceptance Criteria
1. The Code Gate shall attach producer and version, metric-definition and grammar version, source-manifest identity, scope and completeness to observations and baselines.
2. If a before or after file failed parsing or an observation is unavailable, the Code Gate shall prohibit that observation from establishing regression, pre-existing debt or deletion.
3. When code lines or statements increase while at least one comparable control-flow metric improves and none worsens, the Code Gate shall suppress the count ratchet for that entity while retaining absolute-limit judgment.
4. If either side is below a metric's minimum population, the Code Gate shall omit the comparison and prevent that before value from excusing a new breach.
5. The Code Gate shall preserve upstream severity, blocking, pre-existing status and hint as provenance, distinguish local policy from upstream verdict, and never silently downgrade a numeric imported ratchet finding to an ordinary warning.
6. When --changed selects a revision, the Code Gate shall preserve old and new rename paths and deleted paths for accounting, distinguish worktree selection from before-side accounting, and document that untracked files require --files or --all.
7. If an active required measurement is unavailable or incompatible, the Code Gate shall refuse acceptance with a coverage failure rather than report a complete clean result.

### Requirement 12: Prototype and Incremental Delivery Gates
**Objective:** As the owner, I want reusable tooling proved on a bounded corpus before large implementation commitments.

#### Acceptance Criteria
1. The development workflow shall compare existing measurement libraries against hand-counted multilingual fixtures before authorizing custom metric extraction, recording pinned versions, discrepancies, build compatibility and the adoption decision.
2. The first code-gate deliverable shall demonstrate a real-binary task-scoped fail, code repair and pass, plus baseline-debt acceptance and regression rejection, before integration of optional rule families.
3. The code-gate acceptance suite shall cover implementer, CI, changed-revision, onboarding, hook-summary and attribution journeys with actual output, exit and file-effect assertions.
4. The rollout workflow shall retain uncalibrated rules as advisory, record reviewed findings and false positives on representative repositories, and require human review of baseline and configuration changes before blocking adoption.
5. The development workflow shall test source containment, deterministic reports, Windows, Linux and macOS behavior and the unchanged core sandbox build at each integration boundary.

### Requirement 13: Hook Summary and Metric Points
**Objective:** As a developer, I want a short plain report in my pre-commit or pre-push hook and the measured values handed to the telemetry export, so that I see what a change improved or worsened without reading the full report and can follow the trend on a dashboard.

#### Acceptance Criteria
1. When `code check --summary` is given, the Code Gate shall print a summary of at most 25 lines naming the verdict, the before side used, the count of blocking, warning and pre-existing findings, each improved and each worsened metric of the selected entities with its before and after value, and the net statement and line delta when it is available.
2. The Code Gate shall restrict the summary to printable ASCII without colour sequences and to a fixed line order, so that it reads the same in hook output on Windows, Linux and macOS.
3. The Code Gate shall provide its measured values as metric points to the provider contract of quality-telemetry, carrying per metric the highest value, the total, the breach count and the counts of improved and worsened entities with spec, component and task attributes, and writing no metric file itself.
4. While telemetry is disabled or its export fails, the Code Gate shall produce the same verdict, exit code and reports as with a successful export.
5. The Code Gate shall derive the summary and the metric points from the same report model as the text and JSON reports, so that the four outputs never disagree.
6. The repository shall document pre-commit and pre-push hook examples that call the summary on Windows, Linux and macOS, with the Code Gate installing no hook itself.
