# Candidate seam inventory

Status: captured
Verification-IDs: r[latest-state-observation.evaluation.candidate-selected], r[latest-state-observation.semantic-boundary.durable-stream-rejected]

## Selected seam
- Selected: `crates/aspen-blob/src/replication/topology_watcher.rs` local blob replication topology node set.
- Why latest-state is correct: placement only needs the newest membership-derived node set; stale intermediate node sets do not carry durable meaning once a newer set exists.
- Prototype: `LatestTopologyNodeIds` wraps `n0_watcher::Watchable<Vec<u64>>`, normalizes node IDs, exposes `current`, `watch`, and `publish`, and documents skipped-value semantics.

## Rejected seams
- Raft log subscribers and snapshot broadcasts: ordered/durable stream semantics; lag is a diagnostic, not an accepted skip contract.
- CI/job logs and receipts: every log line/stage transition is operator evidence and must remain durable/readable.
- Forge, hook, audit, Nostr, and blob/docs event streams: event ordering and delivery semantics matter; latest-value collapse would lose information.
- Shutdown watch channels: Tokio watch remains sufficient; adopting n0-watcher would add no topology/API clarity.

## Tokio comparison
- `tokio::sync::watch` remains appropriate for OpenRaft-provided metrics and simple shutdown flags.
- `n0-watcher` is accepted only for the local topology-state prototype because its API names `Watchable`/`Watcher`, `get`, `updated`, and disconnect behavior make latest-state semantics explicit without queues.
