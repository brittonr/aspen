## Why

Aspen has several local control-plane seams that need many tasks to observe the newest value of changing state without requiring every intermediate transition. Existing code uses a mix of `tokio::sync::watch`, `tokio::sync::broadcast`, and bespoke watcher loops. The `n0-watcher` crate from the Iroh/n0 ecosystem may be a better fit for these latest-state observations because it explicitly favors bounded resource usage, skipped intermediate values for slow observers, and ergonomic watcher combinators.

## What Changes

- Define a narrow evaluation path for `n0-watcher` in Aspen rather than adopting it workspace-wide.
- Identify candidate seams where latest-value semantics are acceptable: bootstrap readiness, replication topology, peer/health status, and UI/status reporting.
- Require evidence that any adoption simplifies code or improves resource-bound clarity without weakening durable or ordered event semantics.

## In Scope

- A spike or prototype in one local latest-state seam.
- Documentation of accepted and rejected watcher semantics.
- Targeted tests proving slow observers may skip intermediate values but always converge on the latest value.
- Dependency review for feature/default impact, `no_std`/alloc-only boundaries, and Tokio/Iroh compatibility.

## Out of Scope

- Replacing Raft, job, audit, CI, forge, hook, or other durable ordered event streams.
- Blanket migration from `tokio::sync::watch` or `broadcast`.
- Introducing unbounded queues to preserve every intermediate state.

## Capabilities

### New Capabilities

- `latest-state-observation`: Aspen can evaluate and, where justified, use latest-value watcher semantics for bounded local control-plane state propagation.

## Impact

- **Files:** `crates/aspen-blob/Cargo.toml`, `crates/aspen-blob/src/replication/topology_watcher.rs`, docs explaining latest-state semantics, and OpenSpec evidence.
- **APIs:** Adds a local `LatestTopologyNodeIds` prototype in the blob replication topology seam; no network or durable protocol changes.
- **Dependencies:** `n0-watcher` remains targeted to `aspen-blob` and must not leak into alloc-only/core crates.
- **Testing:** Targeted unit tests plus dependency-tree review demonstrate bounded latest-state semantics and no accidental durable-stream replacement.

## Verification Expectations

- Cover `r[latest-state-observation.evaluation.candidate-selected]` by documenting the selected topology node-id seam and why latest-state semantics are correct.
- Cover `r[latest-state-observation.semantic-boundary.durable-stream-rejected]` by recording rejected Raft/log/CI/Forge/hook/audit streams.
- Cover `r[latest-state-observation.semantic-boundary.slow-observer]` with focused tests for initialization, latest-value convergence, skipped intermediate states, and disconnect behavior.
- Cover `r[latest-state-observation.dependency-boundary.core-protected]` with `cargo tree` evidence proving `n0-watcher` stays out of `aspen-core --no-default-features`.
- Validate with `openspec validate evaluate-n0-watcher-latest-state --strict`, `scripts/openspec-preflight.sh evaluate-n0-watcher-latest-state`, and `git diff --check` before archive.
