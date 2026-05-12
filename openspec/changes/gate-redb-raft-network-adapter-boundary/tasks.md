## Phase 1: Inventory

- [ ] [serial] Capture current `aspen-raft-network` default/no-default cargo tree and compile failures.
- [ ] [depends:inventory] Identify the exact `aspen-transport` and `aspen-sharding` paths that pull app concerns.

## Phase 2: Boundary fix

- [ ] [depends:inventory] Feature-gate or reroute adapter dependencies so a minimal adapter graph compiles cleanly.
- [ ] [depends:fix] Add negative boundary evidence for app/runtime/handler/bootstrap crates.
- [ ] [parallel] Capture runtime compatibility evidence for `aspen-raft`, `aspen-cluster`, and handler consumers.

## Phase 3: Closeout

- [ ] [depends:evidence] Run readiness checker evidence for Redb Raft KV adapter readiness and update extraction docs.
- [ ] [depends:closeout] Run strict OpenSpec validation, rustfmt, and `git diff --check`.
