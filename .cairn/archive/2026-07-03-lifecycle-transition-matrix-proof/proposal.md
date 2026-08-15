## Why

The lifecycle state machine is finite, but today the strongest proof lives in hand-picked examples and the `allowed_transition`/`action_matches_target` predicates. A complete transition matrix makes the proof reviewable: every state/action/target combination is checked, every allowed edge is intentional, and every unlisted edge denies.

## What Changes

- Add an explicit proof requirement for the lifecycle transition relation table.
- Add an exhaustive action/target matrix requirement for lifecycle receipts.
- Drive implementation toward a pure, bounded functional core that tests all combinations without adapters, clocks, storage, or network.

## Impact

- **Files**: lifecycle core predicates and lifecycle tests.
- **Testing**: exhaustive positive and negative matrix tests for every lifecycle state/action/target combination, plus focused lifecycle validation commands.
