# Workflow Execution Mode (project memory)

Decision (2026-11-06, user): the default execution mode for this project is **foreground, in the main session, without subagent dispatch** — unless the user explicitly instructs otherwise for a specific run.

## What this means

- Implementation work (`/kiro-impl ...`): run in **manual mode in the main context** — the controller executes each task itself with the TDD cycle (RED → GREEN → REFACTOR → VERIFY), applies the `kiro-review` protocol inline before marking any task `[x]`, and keeps the selective-commit discipline (never `git add -A`).
- Reviews, gap analyses, and research: performed inline in the main session and labeled by role, instead of dispatching reviewer/panel/adversary subagents.
- Fusion (`ultrathink-fusion`), per-task implementers, debug subagents, and any other background dispatch are **opt-in only** — used when the user explicitly asks for them ("use panels", "dispatch", "run in background", etc.).
- Session interruption safety and task state still live in `tasks.md` checkboxes + git commits, as before.

This file is steering: read it at the start of spec/implementation work and follow it unless the user overrides it for the current run.
