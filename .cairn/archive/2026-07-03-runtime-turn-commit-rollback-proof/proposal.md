## Why

The runtime turn boundary is the foundation for every higher-level state-machine proof. If a denied or failed turn can leak pending assertions, messages, effect intents, or object deltas into committed state, later lifecycle, coordination, protocol, and replay proofs are not meaningful.

## What Changes

- Add requirements that prove committed turn deltas match the turn predicate and denied turns leave committed runtime state unchanged.
- Require generated bounded turn traces that mix assertions, retractions, messages, observations, recorded effect responses, commits, and rollbacks.
- Require receipt/ref evidence tying before snapshots, turns, after snapshots, decisions, and diagnostics together.

## Impact

- **Files**: runtime dataspace state, runtime predicate tests, deterministic replay fixtures where useful.
- **Testing**: positive commit traces, negative denied/rollback traces, stale commit denial, and generated Hegel turn sequences.
