## Why

The lifecycle graph should not just reject individual bad edges; it should also prove the overall shape of reachable states and terminal cleanup behavior. This prevents accidental paths that skip startup, escape cleanup, or mutate cleaned entities.

## What Changes

- Add requirements for lifecycle graph reachability from `declared`.
- Add requirements for terminal and cleanup edge restrictions.
- Require positive and negative graph-shape tests in addition to transition matrix checks.

## Impact

- **Files**: lifecycle graph helpers and lifecycle tests.
- **Testing**: reachability assertions, terminal-state negative tests, and cleanup/restart boundary tests.
