## Context

`handle_proposal` denies once the log holds `max_log_entries`; only the collaboration and live-process harnesses issued `CreateSnapshot`. `completed_requests` gained one entry per commit. The Redb durable adapter has a lifetime record and snapshot limit and no retirement. `ReplicaIngressPump` counted deliveries over its lifetime and ended on any poll error, and the reliability host counted connections over its lifetime.

## Decisions

### Decision: Long-running services, explicit bounds

**Choice:** Reliability and collaboration Raft services are long-running. Every bound is a rate (ingress), a concurrency limit (connections), a size threshold (snapshots) or a retention window (dedup, durable records). They stop only on operator stop or teardown, cancellation, a failed replica effect, or an ingress transport failure.

**Rationale:** The issues show no caller relies on lifetime totals; the TLA+ counter models pass only with rate and windowed bounds.

### Decision: Host-issued threshold snapshots

**Choice:** `ReplicaProfile.snapshot_threshold_entries` (validated 0 < t < `max_log_entries`); the pure predicate `is_snapshot_due` is true when a running voter retains at least t applied entries above its snapshot. Hosts check it after each applied event and issue `CreateSnapshot` with their application state ref. Reliability uses 2,048 of 4,096. Failures use the host's existing error path; no snapshot error is swallowed.

**Rationale:** The application state ref belongs to the host; the transition stays pure. Snapshotting after commits keeps max_log_entries - t slots for unapplied entries.

### Decision: Deterministic duplicate-request window

**Choice:** After applying index i, `completed_requests` holds exactly the refs committed in (i - W, i], keeping the newest index per ref. The leader treats a ref as a committed duplicate only through that map and as pending only through uncommitted log entries; committed entries still in an uncompacted log are ignored. Validation requires the map to equal the window computed from the snapshot map and applied log, snapshot maps to hold distinct indices in (s - W, s], and two copies of one ref in a log to be more than W apart.

**Rationale:** A window over applied positions makes the map, and every dedup decision, a function of the applied prefix: replicas at the same applied index agree whatever their snapshot timing. Evicting at snapshot time would make dedup depend on local timing. Raft has no apply-time dedup; applications receive every committed entry.

Reliability selects W = 256 under its unchanged 65,536-byte protocol payload bound. Configuration/recipe admission and startup encode a complete maximum-width snapshot (including 256-byte node IDs and maximum-width positions) before storage/network effects; an incompatible window is refused rather than deferred to tolerated transport loss. The separate length prefix does not consume the payload bound. The log limit remains 4,096 and the snapshot threshold remains 2,048.

### Decision: Crash-ordered durable compaction under retention authority

**Choice:** `RedbReplicaPort::with_retention_authority` admits compaction only for the namespace's declared authority. After synchronizing the snapshot, persistence writes a fixed-size `raft-compaction-start-v1` header with snapshot identity/boundary, term/vote, expected suffix count and durable commit boundary; one existing single-entry `raft-log-mutation-v1` per retained entry; and a `raft-compaction-end-v1` footer naming the header's content ref. No operation encodes the whole suffix. Retirement and covered truncation follow only the complete synchronized base. Recovery stages at most `max_log_entries` entries and exposes the replacement suffix only after its matching footer, without dropping committed entries. Interrupted untruncated bases retain the original suffix and uncertainty; truncated incomplete bases deny. Retirement removes every metadata row but unlinks each unreferenced shared content identity once, preserving any retained reference. Without authority nothing is retired or truncated and namespace limits bound the service.

**Rationale:** Each step is a separate durable commit, and recovery after any prefix rebuilds the newest valid snapshot plus the hard state and entries above it. The base keeps the hard state and suffix that truncation would otherwise remove. The reliability mechanism profile derives the authority from the service, namespace and signed registration authority instead of relaxing the truncation check.

### Decision: Windowed ingress budget and malformed-frame drops

**Choice:** `ReplicaIngressConfig.delivery_window` (non-zero, at most one hour). The pump refills `delivery_limit` at each window boundary and sleeps, without polling, while the budget is spent. The listener source records whether frame derivation failed; only those sessions become `Undecodable`, are dropped, counted (`dropped_frames`) and logged with `tracing`. Other poll errors end the pump with the error.

**Rationale:** Rate limiting keeps back-pressure without a lifetime cap; classifying by the derivation callback avoids treating dead listeners as malformed frames.

### Decision: Connection concurrency bound

**Choice:** The host reads requests in tasks that each hold a semaphore permit; it accepts only while a permit is free and answers fully read requests in its loop. At the bound new connections wait in the listen backlog. Reliability uses 16; admission allows 1..=256.

## Risks / Trade-offs

- A retry whose original commit is older than W applied entries executes again.
- Election churn without commits still appends hard-state records that no snapshot covers; such records are bounded only by the namespace record limit.
- An undecodable-frame flood is not budgeted; each costs one transport session.
