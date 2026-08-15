## Why

Molten's CLI is broad and currently wires many domain workflows directly. If parsing, filesystem IO, rendering, and domain decisions live together, domain logic becomes harder to test and harder to move into focused crates.

## What Changes

- Treat CLI modules as thin imperative shells.
- Move command decision logic into library functions with typed in-memory inputs and structured outputs.
- Keep Clap parsing, file reads/writes, stdout/stderr, process exits, and diagnostic formatting in CLI modules.
- Add positive and negative tests for extracted command cores.

## Impact

The CLI should become easier to maintain without changing user-facing commands. Domain workflows become testable without spawning `molten`, and later crate extraction can leave CLI adapters as stable front doors.
