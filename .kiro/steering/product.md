# Product Overview

Version 2 of spec constraint checking for kiro / cc-sdd spec-driven development: a pure Rust engine (`kiro-core`) that parses `.kiro/` trees, grounds requirement terms in a topic-scoped vocabulary, checks EARS shape and contradictions, and renders the spec graph — exposed through two frontends: an OpenIDEA plugin (`plugins/kiro/`, sandboxed WASM, installed into the sibling repo `../openidea` as a dev plugin) and a CLI gate (`crates/kiro-cli`) for CI. The kiro workspace for this work lives here (`.kiro/specs`, `.kiro/steering`).

Version 1 (`spec-kit-ears-tdd`, `ears-sdd` crate, now retired) proved the idea on the Spec Kit layout and exposed its weak point: term references were matched textually across unrelated topics, so checks fired on words rather than meanings. v2 keeps the idea, fixes the model, and carries v1's remaining features (intentions/precedence, envelope comparison, production-code separation, test traceability) forward as later roadmap waves.

## Core Capabilities

- **Spec constraint checking** (the core idea): acceptance criteria hold EARS shape; the terms a requirement is about are declared and resolve inside the spec's declared topics; contradiction search compares only requirements that share a topic and reports both requirements plus the state that reaches them — or refuses, never hangs.
- **Vocabulary grounding**: `.kiro/vocabulary.toml` committed in the project, seeded from an NVIDIA Auto Ontology export; every term carries a topic, definition, kind, broader/altLabels; specs declare their topics.
- **Spec ↔ code navigation** (IDE): from an open code file, find the spec/task/requirements that own it via the design File Structure Plan and task `_Boundary:_` components; from a spec document, list the components, files and tasks it covers — using only the plugin host API.
- **Visualization** (IDE): render phases and the requirements → criteria → tasks → components → files graph, and task dependencies, as Mermaid/text.
- **CI gate** (CLI): the same checks over a `.kiro` tree with text/JSON findings and exit codes.

## Target Use Cases

- A developer editing code in OpenIDEA asks "which spec owns this file?" without leaving the editor.
- A developer reading `design.md` wants the task breakdown and implementing files at a glance.
- A team running cc-sdd wants requirement terms checked against an agreed, topic-scoped vocabulary instead of drifting prose — in the IDE while writing and in CI before merge.
- A reviewer wants to know whether two requirements in the same topic can both apply in some state, with a concrete witness.

## Value Proposition

Specs stop being terminal-agent-only artifacts: they become a navigable layer inside the IDE and a gate in CI, grounded in an explicit vocabulary where a word means one thing per topic — so checks fire on meaning, not on string matches.

---
_Focus on patterns and purpose, not exhaustive feature lists_
