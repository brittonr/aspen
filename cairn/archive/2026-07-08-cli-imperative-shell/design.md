## Context

The root CLI dispatch fans out across many command modules. Some modules already separate command, IO, and ops files, but the boundary is not enforced uniformly. A modular codebase needs command handlers to parse and orchestrate while domain cores decide.

## Design

### Command shell responsibilities

CLI modules own:

- Clap data structures and argument parsing;
- path resolution and file reads/writes;
- stdout/stderr rendering and process exit behavior;
- invoking adapters and importing/exporting artifacts;
- converting structured domain results into user-facing diagnostics.

### Command core responsibilities

Command cores own:

- in-memory validation of command-specific inputs;
- deterministic operation planning;
- domain decisions and receipt input construction;
- structured success, denial, and diagnostic outputs.

### Migration pattern

Each command family can be migrated one workflow at a time. The CLI handler should convert parsed args into a typed input, call the command core, then perform the shell effects requested by the result. Existing command names, flags, and output contracts stay stable unless another change owns the UX break.

### Testing

Core tests should exercise happy paths and denial paths without invoking the binary. CLI-level smoke tests may remain for parsing and end-to-end behavior, but they should not be the only evidence for domain decision logic.

## Non-goals

- Do not redesign the CLI surface in this change.
- Do not remove current commands or aliases.
- Do not push filesystem or rendering into core crates.
