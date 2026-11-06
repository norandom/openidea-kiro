# Implementation Plan

- [ ] 1. Foundation: workspace scaffold, finding model, Markdown scanner, fixtures
- [x] 1.1 Create the root Cargo workspace and the engine crate skeleton
  - Prerequisite: `rustup target add wasm32-wasip2` (required for the wasm build observable here and in 5.4); `wasmtime` on PATH is optional and only enables the wasm *test* run in 5.4
  - Root workspace manifest listing the engine crate with resolver 3; engine package on edition 2024 with serde (derive) and serde_json as the only runtime dependencies and insta as a dev-dependency
  - Lint table forbidding unsafe code and denying unwrap, expect, panic, indexing/slicing, todo and unimplemented in library code
  - Per-target config for the wasm test runner only
  - Crate root with crate-level docs placeholder, every `pub mod` declaration for the modules in the File Structure Plan (and the render module root declaring its two submodules) written up front, and one documented empty stub per module so the crate builds before any module has content (ownership of each stub file passes to its module task); later module tasks never edit the crate root (public re-exports are added in 5.1; crate-level docs and doc tests in 5.4)
  - Observable: native build, `wasm32-wasip2` build, clippy with warnings denied and fmt check all pass on the empty crate
  - _Boundary: CrateScaffold_
  - _Requirements: 1.1, 12.4, 12.6_

- [x] 1.2 Implement the finding model and code catalogue
  - Severity levels, the finding value with feature, project-relative file, optional 1-based line and message, and builder helpers that take the default severity from the catalogue
  - Catalogue of every code this crate emits (the 21 entries in the design's Supporting References) with default severity and description, plus string constants per code
  - Stable sort by file, then line with absent lines first, then code, then message
  - JSON serialization of a finding with fixed field order
  - Observable: unit tests prove the sort order, that every constant is in the catalogue, and the JSON shape from the design's data contract
  - _Boundary: Finding_
  - _Requirements: 10.1, 10.2, 10.3_

- [x] 1.3 Implement the shared Markdown line scanner
  - Single forward pass producing typed lines (blank, heading, list item with optional bullet/checkbox/numbered label, annotation, table row, fence open/close/fenced, text) with 1-based line numbers identical for LF and CRLF and a tolerated leading byte-order mark
  - Heading outline, fenced blocks with info string, span and body; fence rules per design (three or more backticks or tildes, closed by the same character at least as long)
  - The owned annotation value (key, value, line) that the requirements and tasks parsers both reuse
  - Unclosed fence runs to the last line and yields the info finding
  - Observable: unit tests cover BOM, CRLF numbering, `##tag` not a heading, mismatched fence lengths, tilde fences, checkbox with optional marker, dotted numbered labels, annotation and table-row recognition
  - _Boundary: MarkdownScanner_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 12.3_

- [x] 1.4 (P) Create the fixture trees, grammar matrix and self fixtures
  - Minimal tree: roadmap with two entries and a dependency; `demo` feature in tasks-generated phase with topics, canonical requirements with annotations, table-shape design plan with two components (at least one mapped to a source-file path such as `src/demo.rs`) and a Mermaid block, tasks using `(P)`, `[x]`, `[ ]*` and all three annotations where every `_Boundary:_` value names one of the two demo components and every `_Requirements:_` entry names an existing demo criterion; `old` feature superseded with notes naming `demo`; no `initialized`-phase feature is included (tests synthesise one in memory)
  - Grammar matrix adapted from the prior-art conformance inputs: canonical, dotted, R-style, amendment, empty, fenced example, malformed (duplicate ids, non-numeric heading, unclosed fence)
  - Task variants fixture (`[X]`, optional marker, `(P)` on a major, depth-3 id, duplicate id, invalid requirement reference) and design plan fixtures (table with multi-path cells, bullet shape, no plan, and `tree-plan.md`: the template tree with comments plus a table row naming some of the tree files, so that some tree files are assigned to a component and the rest stay unassigned)
  - Self fixtures: byte copies of this spec's spec.json, requirements.md, design.md, tasks.md and the steering roadmap
  - Observable: every file under the minimal, grammars, tasks, design and self fixture directories listed in the File Structure Plan exists with the content described above (golden fixtures are created in 5.1)
  - _Boundary: Fixtures_
  - _Requirements: 4.1, 4.3, 4.4, 4.5, 4.12, 5.1, 5.2, 5.3, 6.2, 6.3, 6.4, 6.8, 7.2, 8.7_

- [ ] 2. Core: document parsers
- [x] 2.1 (P) Implement the spec metadata parser
  - Parse `spec.json` field by field through a generic JSON value so unknown or wrongly typed fields degrade individually; expose name, language, phase with raw string, approvals, ready flag, topics, notes and timestamps
  - Phase classification for the six recognised phases (case-insensitive, trimmed) with unknown and missing phase findings; invalid JSON yields defaults plus the error finding
  - The document-kind enumeration, the phase-label helper (`phase_raw` trimmed when non-empty, else `"missing"`), the required-document table per phase, and successor-name extraction from notes (whole-word, longest-first, deduplicated); the workspace and frontends consume these
  - Observable: unit tests cover all phase spellings, each finding, defaults for absent fields, ignored unknown fields, the phase-label helper (absent, empty, padded, present), the required-document table and successor extraction where a longer name shadows a shorter prefix
  - _Boundary: SpecMetaParser_
  - _Depends: 1.2_
  - _Requirements: 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 8.7_

- [x] 2.2 (P) Implement the requirements parser
  - State machine over scanned lines recognising canonical, dotted, R-style and amendment headings; criterion opening by self-identifying dotted markers anywhere and by bare numbered markers under an Acceptance Criteria heading or inside an amendment
  - Continuation joining, blank-line closing, objective capture, annotations (the scanner's owned annotation value) attached to the requirement or the open criterion, extends list on amendments
  - Findings for requirements without criteria, duplicate ids (first wins) and non-numeric headings (block skipped); empty document yields no findings
  - Lookup helpers by requirement id and criterion id
  - Observable: the grammar fixtures parse to the expected criterion id lists and finding codes; fenced example ignored
  - _Boundary: RequirementsParser_
  - _Depends: 1.3, 1.4_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 4.11, 4.12_

- [x] 2.3 (P) Implement the tasks parser
  - Task items from checkbox list lines with one- or two-group ids, checked/optional/parallel flags, description with the parallel marker removed, detail lines until the next task or heading
  - Two-level nesting in document order; hierarchy finding for depth over two or a sub-task without its major (kept at top level); duplicate id finding (first wins)
  - Interpretation of the requirements, depends and boundary annotations (comma split, trimmed, empties dropped), recording the line of each so the join can place its findings, with the invalid-reference finding; other annotations stored as the scanner's owned annotation value
  - Counts for the document (all items) and per major task (sub-tasks only, or itself when it has none); lookup by id and flat iteration
  - Observable: the minimal and variants fixtures parse to the expected tree, flags, annotation lines, counts and findings
  - _Boundary: TasksParser_
  - _Depends: 1.3, 1.4_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11, 5.12, 5.13_

- [x] 2.4 (P) Implement the design parser: regions, table and bullet shapes, components, Mermaid blocks
  - File Structure Plan and Components regions by heading text up to the next heading of equal or higher level
  - Table shape with component and file/files/path columns (multi-path cells split, responsibility column optional) and bold-component bullets with backticked paths
  - Component discovery from level-4 headings in the Components region and from any table's component column; findings for components without files, unassigned files and a missing plan; Mermaid blocks with spans; design path normalisation
  - Lookup helpers: component by trimmed, case-insensitive name; files of a component; unassigned files
  - Fenced directory trees inside the plan are ignored by this task; the self design fixture's expected file set here is the table-derived set only (tree support lands in 2.5)
  - Observable: the table, bullet and no-plan design fixtures and the self design fixture parse to the expected components, files, Mermaid blocks and findings
  - _Boundary: DesignParser_
  - _Depends: 1.3, 1.4_
  - _Requirements: 6.1, 6.2, 6.3, 6.5, 6.6, 6.7, 6.8, 6.9, 6.10_

- [x] 2.5 Implement the design parser: fenced directory-tree shape and merge
  - Tree entries from fenced blocks inside the plan region with depth from the name column (unit 4 when tree glyphs are present, else the smallest indent), directory entries pushed on a stack and skipped as files, trailing comments kept as responsibility, full paths joined from ancestors
  - Tree files also named by a table or bullet mapping are merged regardless of which appears first in the document (component filled in, responsibility kept when the mapping has none); remaining tree files stay unassigned with their finding
  - Observable: the tree design fixture (`tree-plan.md` from 1.4, tree plus a table row naming some tree files) parses to the expected full paths and responsibilities, the table-named tree files are merged into their component without duplicate entries, and the remaining tree files carry the unassigned-file finding
  - _Boundary: DesignParser_
  - _Requirements: 6.4, 6.7, 6.10_

- [x] 2.6 (P) Implement the roadmap parser
  - Specs region by heading text; entries from checkbox list lines split into name, description and dependencies with `--` and em dash separators, backticks stripped, `none` as empty
  - Unknown-dependency finding after all entries are read; missing-section finding with an empty list; names in document order deduplicated
  - Observable: the minimal roadmap fixture and the self roadmap fixture parse to the expected entries, and unit tests cover each separator, `none`, a missing section and an unknown dependency
  - _Boundary: RoadmapParser_
  - _Depends: 1.3, 1.4_
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [ ] 3. Core: renderers
- [x] 3.1 (P) Implement the graph model, caps, safe identifiers and line clipping
  - Graph with direction, nodes (id, label, optional class) and edges (from, to, optional label); caps with defaults; rendered result carrying truncation flag and omitted counts
  - Identifier map producing letters, digits and underscores only, prefixing when needed and suffixing on collisions, stable for a given insertion order
  - Cap application keeping the first nodes and the edges whose ends survive, in input order; line clipping at a newline below the byte limit with an omitted-lines trailer (`"… N more lines"` when no hint is given, or `"… N more lines — <hint>"` when an optional trailer hint is supplied; crate-internal helpers may carry a temporary `#[allow(dead_code)]` until 3.2/3.3 call them, so clippy stays clean per task)
  - Observable: unit tests cover sanitising, collision suffixes, cap counts and clipping at exact boundaries with and without a trailer hint
  - _Boundary: RenderGraph_
  - _Depends: 1.1_
  - _Requirements: 11.1, 11.3, 11.7, 11.8_

- [x] 3.2 Implement the Mermaid backend
  - Graph header by direction, quoted node labels with escaping of double quotes and angle brackets, class lines, edges with optional quoted labels, ellipsis node stating omitted counts when truncated, single trailing newline
  - Observable: unit tests prove no raw `<`, `>` or unescaped quote appears inside labels and that the truncated output contains exactly one ellipsis node
  - _Boundary: MermaidRenderer_
  - _Requirements: 11.1, 11.2, 11.3, 11.8_

- [x] 3.3 (P) Implement the text backend
  - Graph as one line per node followed by indented outgoing edges under the same caps, with the omitted-counts trailer
  - Findings as `path:line: SEVERITY CODE message` lines in the finding sort order (file, line, code, message), line omitted when absent
  - Markdown table with escaped pipes and padded short rows
  - Observable: unit tests cover a graph with labelled and unlabelled edges, a finding without a line, and a cell containing a pipe
  - _Boundary: TextRenderer_
  - _Depends: 3.1, 1.2_
  - _Requirements: 11.4, 11.5, 11.6, 11.8_

- [ ] 4. Integration: join, workspace and resolution
- [x] 4.1 Implement the file, component and task join
  - Path matching by normalised equality or suffix at a segment boundary in either direction, normalising with the design parser's path normalisation (the join must not import the resolver — dependency direction)
  - Owners of a queried path: matching components in design order, tasks whose boundary names them in document order, criteria from those tasks' requirement references sorted and deduplicated, and a covered flag
  - Coverage listing of components with files and bound tasks, unscoped tasks, unassigned files, and unknown-boundary tasks (every task whose `_Boundary:_` contains one or more names matching no design component paired with those unmatched names as written, or all boundary names when the design document is absent)
  - Link findings: unknown boundary component at the task line; unknown criterion reference (valid shape only; a whole-requirement reference is known when that requirement exists) and unknown task dependency at their annotation lines; tolerating absent requirements or design
  - Observable: unit tests cover the segment-boundary rule (`b/c.rs` must not match `ab/c.rs`), multi-match, not covered, case-insensitive names, unknown-boundary tasks with design present (including mixed known/unknown boundaries) and absent, and each of the three findings at its specified line
  - _Boundary: Join_
  - _Depends: 2.2, 2.3, 2.5_
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10_

- [ ] 4.2 Implement the read port, in-memory source and workspace
  - Read trait keyed by project-relative path; in-memory source with exact key lookup (no path normalisation in this module) for tests and frontends
  - Workspace owning its source, known feature names (sorted, extendable), a read cache that also remembers absent paths, loaded features and the roadmap; document paths built by joining the fixed directory and file names only
  - Loading a feature reads its four documents once each, parses them, runs the join's link findings when tasks exist, records the missing-document finding with severity from the phase table (an absent `spec.json` is always a warning), and records the dangling-successor finding for superseded features using the metadata parser's successor extraction; loading adds the feature to the known names
  - Every finding recorded on a loaded feature (parser, link, missing-document, dangling-successor) carries that feature's name
  - Roadmap loading and adoption of its names as known features; merged, sorted findings across features and roadmap
  - Observable: a counting source wrapper proves each path is read exactly once across repeated loads; missing-document severities follow the phase table including the always-warning absent `spec.json`; every finding on a loaded feature carries its name; the superseded fixture yields the dangling finding when its successor is not known and none when it is
  - _Boundary: Workspace_
  - _Depends: 2.1, 2.6, 4.1_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 8.8, 10.4_

- [ ] 4.3 Implement path normalisation, current-spec resolution and the successor chain
  - Relative normalisation (separators, leading `./`, repeated and trailing slashes); absolute-to-relative conversion against the workspace root with case-insensitive prefix comparison for Windows-style paths and an explicit outside result; feature extraction from `.kiro/specs/<feature>/…`
  - Resolution order: explicit name, active-file feature, single known feature, otherwise unresolved with sorted candidates and the normalised active path
  - Successor chain over the workspace following the first successor, loading as needed, terminating at a non-superseded feature or reporting a cycle with its finding; re-export of the metadata parser's successor extraction
  - Observable: unit tests cover `C:\root\x`, `c:/ROOT/x`, `/root/x`, mixed separators, outside paths, each resolution branch, and a two-feature cycle producing exactly one cycle finding
  - _Boundary: Resolver_
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9_

- [ ] 5. Validation
- [ ] 5.1 Create the crate-root re-exports, the shared test harness, golden fixtures and golden renderer tests
  - Public re-exports of the model, workspace, resolution, join, finding and render types from the crate root as listed in the design's module map (every module is already `pub mod` since 1.1; this is the only edit to the crate root until 5.4 adds the crate-level docs and doc tests)
  - Test helper building an in-memory source from embedded fixture text so no test touches the file system; harness and tests import through the crate-root re-exports
  - Golden graph (labels with quotes and angle brackets, classes, labelled edges), golden text rendering, truncated Mermaid under small caps, and golden findings text, each stored as a fixture file
  - Observable: golden tests compare renderer output byte-for-byte against the fixture files and pass, compiled against the crate-root re-exports
  - _Boundary: CrateScaffold, Fixtures, IntegrationTests_
  - _Depends: 3.2, 3.3, 4.3_
  - _Requirements: 1.5, 11.2, 11.3, 11.4, 11.5, 11.8_

- [ ] 5.2 (P) Write the grammar matrix and workspace integration tests
  - Owns only the grammar and workspace test files; reads the shared helper from 5.1 without changing it
  - Grammar test iterating every grammar fixture against expected criterion ids and finding codes
  - Workspace tests over the minimal tree: clean load of `demo`, missing-document severities for tasks-generated (from the fixture) versus initialized (an `initialized` feature synthesised in the in-memory source, since the fixture has none), single read per path, superseded `old` resolving to `demo` with a terminating chain, owners of a demo source file, resolution from an absolute Windows-style active path
  - Self-fixture test: this spec's own design yields every component of its Files by Component table with at least one file, its tasks produce no unknown-boundary finding, and two runs of the merged findings are identical
  - Observable: all integration tests pass natively
  - _Boundary: IntegrationTests_
  - _Depends: 4.3, 5.1_
  - _Requirements: 1.3, 1.4, 4.1, 4.2, 4.3, 4.4, 4.5, 8.2, 8.5, 8.7, 8.8, 8.9, 9.1, 9.6, 10.4, 12.5_

- [ ] 5.3 (P) Write the no-panic and performance tests
  - Owns only the no-panic and performance test files; reads the shared helper from 5.1 without changing it
  - Every parser and the scanner fed with empty text, a lone byte-order mark, ten thousand nested headings, ten thousand unclosed fences, one-megabyte lines and 256 documents from a fixed-seed generator over the marker alphabet; each call returns a document with partial data where recognisable
  - Synthetic 16 MiB tasks-shaped document timed through the scanner and parser, asserted under two seconds only in release builds
  - Observable: the no-panic suite passes in debug and the timing assertion passes under `cargo test --release`
  - _Boundary: IntegrationTests_
  - _Depends: 4.3, 5.1_
  - _Requirements: 12.1, 12.2, 12.3_

- [ ] 5.4 Verify both targets and the grammar documentation
  - Crate-level documentation describing the accepted grammar per document type and listing the finding codes, with a doc test asserting the documented code list equals the catalogue and a crate-root doc test that compiles a workspace load, a resolution and a render using only the re-exported names
  - Native test run, `wasm32-wasip2` build, and the wasm test run when the runner is installed; clippy with warnings denied and fmt check
  - Observable: `cargo test` (including the crate-root doc tests), `cargo build --target wasm32-wasip2`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` all succeed from the repository root
  - _Boundary: CrateScaffold_
  - _Depends: 5.2, 5.3_
  - _Requirements: 12.4, 12.6_

- [ ] 5.5 Wire the CI gate: `cargo gate` alias, parallel tests, the wasm dependency audit and the CI files (amendment 2026-11-04, gap-analysis arbitration)
  - `.cargo/config.toml` gains the `gate` alias alongside the existing wasm runner config (running the native parallel test suite; the full multi-step gate sequence — fmt --check, clippy --all-targets with warnings denied, `cargo nextest run`, `cargo test --doc`, the `wasm32-wasip2` engine build — is wired in `.github/workflows/ci.yml`; the `plugins/kiro` component build and `cargo run -p kiro-cli -- check --project . --all --fail-on error` step are NOT in this workflow yet because `plugins/kiro` and `kiro-cli` do not exist yet — cli-gate task 7.4 appends them)
  - Creates `deny.toml` (cargo-deny config: `wasm32-wasip2` in `[graph] targets`, `getrandom`/`rand` and fs-dependent crates banned on that graph, with `wrappers = ["biodivine-lib-bdd", "rand"]` permitted so `biodivine-lib-bdd`'s dead-code-eliminated `rand` dependency does not trip the lockfile check when spec-constraints lands) and `.github/workflows/ci.yml` (dtolnay toolchain + Swatinem/rust-cache + `rustup target add wasm32-wasip2` + `cargo install cargo-nextest cargo-deny --locked` + `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo nextest run` + `cargo test --doc` + `cargo deny check` + `cargo build -p kiro-core --target wasm32-wasip2`)
  - CI runs `cargo nextest run` + `cargo test --doc` per requirement 12.7 and `cargo deny check` so `getrandom`/`rand`/fs stay out of the wasm tree automatically
  - Explicit integration touches: the CI job's plugin build and kiro-cli step (and any gate alias update) are appended by cli-gate task 7.4; the CI procedure is documented by cli-gate task 7.1 (Docs)
  - Observable: the `gate` alias, `deny.toml` and `.github/workflows/ci.yml` exist; every command in `.github/workflows/ci.yml` succeeds on a clean checkout; a nextest run executes the suite in parallel and passes
  - _Boundary: CrateScaffold_
  - _Depends: 5.4_
  - _Requirements: 12.7_
