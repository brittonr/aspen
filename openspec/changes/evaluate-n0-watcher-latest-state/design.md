## Context

`n0-watcher` provides watchable values: writers set a current value, watchers observe initialization and updates, and slow watchers may miss intermediate values. This is close to `tokio::sync::watch`, but with n0/Iroh ecosystem alignment and combinators for mapping, joining, initialization, and stream views. Aspen already uses latest-state and broadcast patterns in local subsystems, but not every use has the same delivery requirement.

## Goals / Non-Goals

**Goals:**

- Find one Aspen seam where "latest value wins" is the real contract.
- Make the semantic boundary explicit: observers MUST be able to miss intermediate values and MUST NOT rely on replay.
- Keep resource usage bounded and predictable.
- Compare against the existing Tokio primitive before adding a dependency.

**Non-Goals:**

- Do not use `n0-watcher` for Raft/state-machine history, CI/job logs, Forge events, audit streams, hooks, or any stream where every event is required.
- Do not add `n0-watcher` to workspace-wide dependency bundles by default.
- Do not change network protocols or durable storage formats.

## Decisions

### 1. Prototype before adoption

**Choice:** Implement at most one targeted prototype before accepting the dependency.

**Rationale:** Aspen already has equivalent Tokio primitives available. The dependency is only worthwhile if it removes bespoke watcher code, clarifies skip semantics, or improves API ergonomics in real Aspen code.

**Alternative:** Add `n0-watcher` immediately and migrate all watch-like code. Rejected because broadcast/watch uses include durable event-like flows where skipped values would be incorrect.

**Implementation:** Start with a candidate such as blob replication topology watching or cluster bootstrap readiness, where current-state convergence matters more than every intermediate state.

### 2. Latest-state semantics are explicit and local

**Choice:** Any adopted use MUST document that observers receive the current value and may skip intermediate values.

**Rationale:** This prevents accidental use for logs, ordered transitions, or replayable control flows.

**Alternative:** Hide the watcher behind a generic event-stream trait. Rejected because that would obscure the most important semantic difference.

**Implementation:** Keep wrappers named around latest/current state, not events, logs, or streams. Tests should include slow observers to prove skipped values are acceptable.

### 3. Dependency impact is part of acceptance

**Choice:** The prototype is accepted only if dependency-tree review shows no unwanted leakage into alloc-only or foundational crates.

**Rationale:** Aspen has active crate-decomposition and no-std/alloc-only boundaries. A local ergonomic dependency must not compromise those seams.

**Alternative:** Treat the dependency as harmless because it is small. Rejected because dependency placement matters more than dependency size in Aspen's architecture.

## Risks / Trade-offs

**Semantic misuse** → Mitigate with naming, docs, and negative tests that prohibit use where every event must be observed.

**Dependency churn** → Mitigate by keeping usage in one crate until the prototype is accepted.

**Tokio watch is sufficient** → Mitigate by requiring a comparison note; if `n0-watcher` does not materially improve the seam, close the change without adoption.

## Validation Plan

- Run `cargo tree` or equivalent dependency review for the candidate crate and relevant core/no-default feature graph.
- Add tests for initialization, latest-value convergence, and slow-observer skip behavior.
- Add or update docs/comments describing where latest-state observation is allowed and forbidden.
- Run the targeted crate tests and the appropriate quick verification gate for the touched workspace slice.
