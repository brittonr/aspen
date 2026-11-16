## Why

Issues #56 and #59: the reliability Raft service never compacted its log and denied every proposal after 4,096 commits; where services did snapshot, `completed_requests` grew without bound; the durable store under the Raft port never retired snapshots or covered records, so a snapshotting service still failed at its namespace's snapshot or record limit; Raft ingress stopped after a lifetime delivery total and on the first undecodable frame; and the reliability host exited after `max_connections` connections in total. No document said these services were bounded runs.

## What Changes

- Reliability and collaboration Raft services are classified as long-running services and their bounds become rates, concurrency limits, or retention windows.
- Replica profiles declare a validated snapshot threshold (0 < threshold < `max_log_entries`) and a validated duplicate-request window W. Hosts issue `CreateSnapshot` with the application state ref when the threshold is reached; a denied or failed snapshot takes the host's fail-stop path.
- `completed_requests` keeps exactly the request refs committed in (applied - W, applied]. Snapshots carry that window; install restores it; validation rejects refs outside it. Two copies of a request ref in one log must be more than W apart.
- Under the namespace's explicit retention authority, snapshot persistence writes a compaction base, retires superseded snapshots, and truncates only snapshot-covered records. Recovery accepts a compacted record prefix only behind a covering snapshot and its compaction base.
- Ingress admits `delivery_limit` deliveries per `delivery_window`, pauses polling when the budget is spent, drops, counts and logs undecodable frames, and still ends with the error on transport failure.
- The reliability host's `max_connections` bounds simultaneously served connections.

## Impact

- **Files**: Raft profile, admission, transition, validation, recovery, durability port and ingress pump; durable-state core and Redb/simulated adapters; reliability recipe, decoder, mechanism profile and host; collaboration native runtime configuration and driver; live-process and collaboration harnesses; `docs/distributed-system-fabric.md`.
- **Compatibility**: Replica and service configurations gain required fields (`snapshot_threshold_entries`, `request_retention_entries`, `ingress_delivery_window_ms`). A retry whose original commit is older than W is a new request.
- **Testing**: transition, durable-core, durable-port crash, ingress (paused time) and connection-intake tests.
