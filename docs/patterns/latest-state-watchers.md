# Latest-State Watchers

Latest-state watchers are for local control-plane state where observers need the newest value and may safely miss intermediate transitions. They are not durable event streams.

## Allowed use

Use a latest-state watcher when all of these are true:

- the value represents current local state, not history;
- a slow observer may skip intermediate values without violating correctness;
- convergence on the newest value is sufficient;
- resource use must remain bounded without per-observer queues.

The initial Aspen `n0-watcher` prototype is `LatestTopologyNodeIds` in `aspen-blob` replication topology watching. Blob replica placement only needs the current membership-derived node-id set; once a newer topology exists, earlier intermediate sets are stale.

## Forbidden use

Do not use `n0-watcher` or any latest-state primitive for:

- Raft logs, state-machine history, or snapshots;
- CI/job logs, job timelines, or operator receipts;
- Forge event streams;
- audit streams;
- hook streams;
- any protocol or storage surface where every transition must be observed in order.

Those flows need durable or ordered delivery semantics. Collapsing them to the latest value would lose evidence.

## Tokio comparison

`tokio::sync::watch` remains acceptable for existing shutdown and upstream metrics channels, especially when it is already the provider API. Prefer `n0-watcher` only where its `Watchable`/`Watcher` API makes latest-state semantics clearer in Aspen-owned code and dependency placement stays local to the implementing crate.
