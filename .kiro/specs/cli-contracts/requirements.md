# Requirements Document

## Introduction

Common CLI Contracts. Owns shared parameter groups, command capability profiles, compatible migration, output routing and exit conventions. Does not own individual code rules, evidence semantics or external analyzers.

## Scope and Adjacent Expectations

Consumes: cli-gate. The owner authorized the reviewed amendments and progression to design on 2026-11-09. Each spec delivers and accepts its own behavior; original criterion identities are preserved in [the split map](../code-constraints/split-map.md).

## Requirements

### Requirement 1: Shared Parameters and Output Contract
**Objective:** As a user of the project's command lines, I want every command to accept the same global, output and gate parameters with the same names, defaults, environment fallbacks and exit codes, provided by one shared library, so that learning one command teaches the rest and a second binary inherits the standard.

#### Acceptance Criteria
1. The workspace shall provide one library for reusable global, report-output and gate parameter groups and the 0/1/2 exit model, with each command composing only the groups meaningful to its behavior.
2. The kiro-cli shall preserve existing command names, positionals, defaults and accepted flag spellings, including --root, --table and --out, while declaring compatibility exceptions for flags whose existing meaning differs from a shared group.
3. The kiro-cli shall take each shared parameter from, in order, the command line, an environment variable (`KIRO_PROJECT`, `KIRO_FORMAT`, `KIRO_FAIL_ON`, `NO_COLOR` and `CLICOLOR_FORCE` for colour), then the built-in default, documenting the precedence.
4. The kiro-cli shall follow one naming rule for every parameter: long flags in kebab-case, short flags only for verbosity, quiet, help and version, repeatable flags written once per value, enumerated values in lowercase, and paths printed project-relative with `/` separators.
5. When JSON report output is selected, the kiro-cli shall write only the schema-versioned document to its selected report destination and diagnostics to standard error, preserving a separate destination for commands that generate an artifact.
6. The kiro-cli shall offer `completions <shell>` printing a shell completion script generated from the command definitions, and `--version`.
7. The repository shall provide an executable conformance suite covering every command's declared capability profile, shared defaults, accepted aliases, allowed short flags and help-document agreement.

### Requirement 2: Compatibility and Executable Conformance
**Objective:** As an existing CLI user, I want predictable extensions without breaking established invocations.

#### Acceptance Criteria
1. The kiro-cli shall retain vocab import --format as a legacy input-format spelling, introduce --input-format for explicit input selection, and use --report-format for the import command's diagnostic report without changing imported TOML output.
2. The kiro-cli shall preserve graph and tasks default rendering and --table behavior, reject conflicting output selectors, and document the formats supported by each command.
3. The conformance suite shall validate applicable shared groups from a command-capability manifest and execute representative old and new invocations, including environment precedence and file output failures.
4. The first CLI deliverable shall migrate one existing check command through the shared library with equivalent behavior before migrating the remaining commands.
