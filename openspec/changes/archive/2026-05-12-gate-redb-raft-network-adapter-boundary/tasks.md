## Phase 1: Inventory

- [x] [serial] Capture current `aspen-raft-network` default/no-default cargo tree and compile failures.
- [x] [depends:inventory] Identify the exact `aspen-transport` and `aspen-sharding` paths that pull app concerns.

## Phase 2: Boundary fix

- [x] [depends:inventory] Feature-gate or reroute adapter dependencies so a minimal adapter graph compiles cleanly.
- [x] [depends:fix] Add negative boundary evidence for app/runtime/handler/bootstrap crates.
- [x] [parallel] Capture runtime compatibility evidence for `aspen-raft`, `aspen-cluster`, and handler consumers.

## Phase 3: Closeout

- [x] [depends:evidence] Run readiness checker evidence for Redb Raft KV adapter readiness and update extraction docs.
- [x] [depends:closeout] Run strict OpenSpec validation, rustfmt, and `git diff --check`.
