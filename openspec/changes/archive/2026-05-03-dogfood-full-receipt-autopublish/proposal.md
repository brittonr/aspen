## Why

Dogfood can already publish receipts into Aspen KV, but the acceptance path still depends on an operator remembering to run a separate publish command while the cluster is live. A self-hosting acceptance run should store its own final evidence in Aspen before teardown and should offer an explicit inspection mode when the operator wants to read that evidence back before cleanup.

## What Changes

- `full` dogfood runs publish their validated final success receipt into the running cluster after verify succeeds and before any automatic stop.
- Operators can ask `full` to leave the verified cluster running long enough to inspect the in-cluster receipt with `receipts cluster-show`.
- Failed runs keep the existing local failure receipt behavior and do not pretend in-cluster publication succeeded.

## Impact

- **Files**: `crates/aspen-dogfood/src/main.rs`, `docs/deploy.md`, dogfood evidence OpenSpec.
- **APIs**: `aspen-dogfood full` gains an operator flag for leaving the verified cluster running.
- **Testing**: cargo check/nextest for `aspen-dogfood`, OpenSpec validation, CLI help smoke, and a targeted cluster smoke when feasible.
