## Why

The generated distributed simulation layer can explore fault interleavings, but release review also needs a compact named suite of high-value composite regressions. Without a promotion and budget policy, generated coverage can either drift silently or become too broad to run as a reliable CI signal.

## What Changes

- Add a named composite fault regression suite for duplicate-after-restart, partition-with-stale-evidence, reorder-with-ack-reconciliation, crash-during-dispatch, and resource-pressure-during-quorum cases.
- Define promotion rules for turning generated failing seeds into named deterministic fixtures with preserved seed, topology, fault plan, command refs, and invariant refs.
- Add a coverage budget and shard metadata so fast, protocol, VM, and soak profiles can state which composite cases they cover or intentionally exclude.
- Add negative coverage proving retry-only success, undeclared variance, ambient-state drift, and missing denial refs cannot satisfy distributed pass evidence.

## Impact

Generated exploration becomes a stable regression workflow instead of a one-off property run. Reviewers can see which composite fault classes are covered, which are deferred, and which evidence remains diagnostic-only.
