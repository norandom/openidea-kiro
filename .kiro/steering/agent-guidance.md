# Agent Guidance — the (c) contract

Decision of the 2026-11-04 ultrathink-fusion review: "guide the agent with the logics derived" is implemented as a **feedback contract between the agent harness and `kiro-cli`** — not as a plugin feature, and not as a NVIDIA formal-methods integration. The plugin sandbox cannot observe agents (no network, no processes, no filesystem), and nothing in the specs or steering grounds any NVIDIA formal-methods-for-agents work; the only NVIDIA artifact in this project is the Auto Ontology **export consumed as vocabulary data** (vocabulary-grounding). Any deeper integration stays out of scope until a grounded source exists.

## The contract (three mechanisms)

1. **Machine-readable findings with witnesses.** The agent harness runs `kiro-cli check --project . --all --format json` after spec edits and injects the findings into the agent's context. Contradiction findings carry a BDD witness state, so remediation starts from a concrete counterexample instead of re-deriving the search. Exit codes drive the loop.
2. **Pre-committed vocabulary.** `.kiro/vocabulary.toml` plus each `spec.json`'s `topics` are committed, machine-readable constraints on the words a spec may use. The cc-sdd harness's steering/templates reference them in agent prompts; violations surface as `TERM_TOPIC_MISMATCH` / `TERM_UNDECLARED` at the next gate run.
3. **Annotation grammar as a steering convention.** `_Terms:_`, `_Guard:_`, `_Effect:_`, `_Conflicts:_` (spec-constraints) are emitted by construction through the cc-sdd templates and steering, so the formal machinery (EARS shape, guard/finitisation, BDD contradiction search) always has something to check.

## Ownership

- The plugin (`plugins/kiro`) is the **human readout** of these logics; it never runs or observes agents.
- `kiro-cli` is the **agent input**; its JSON report is the durable record.
- The harness wiring (run `kiro-cli check` after spec edits, feed findings back) is configuration of the OpenCode + cc-sdd harness, outside this repository's code; it needs no new engine machinery.

## Honest limits (must be stated in the docs)

- A document without annotations produces no term or contradiction failure; the `CHECK_COVERAGE` info finding (spec-constraints, amendment 2026-11-04) makes this visible instead of silently green.
- The contradiction search compares only pairs within a spec's declared topics; cross-topic contradictions are a documented blind spot.
- Guards compare a term to a literal only; anything else is `GUARD_UNPARSED`, never silently skipped.
