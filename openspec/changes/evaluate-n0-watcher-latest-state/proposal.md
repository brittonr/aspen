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

- **Files:** Candidate crates include `crates/aspen-cluster`, `crates/aspen-blob`, status/TUI-adjacent code, and docs explaining watcher semantics.
- **APIs:** Any public API change must be justified by the prototype; internal-only adoption is preferred.
- **Dependencies:** `n0-watcher` must remain targeted to crates that use it and must not leak into alloc-only/core crates unless separately justified.
- **Testing:** Targeted unit/integration tests plus dependency-tree review must demonstrate bounded latest-state semantics and no accidental durable-stream replacement.
