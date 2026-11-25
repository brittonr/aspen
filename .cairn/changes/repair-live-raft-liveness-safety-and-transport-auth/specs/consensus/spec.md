# Consensus deltas

## ADDED Requirements

### Requirement: Live Raft timer races and role transitions do not terminate healthy replicas
r[molten.consensus.raft_timer_lifecycle_safety] `TokioReplicaPort` MUST cancel the election timer when arming heartbeats and cancel the heartbeat timer when arming elections. Raft transition handling and `Host::serve_bound` MUST treat stale or role-mismatched queued timer events as benign no-ops rather than fatal `Denied` errors.

#### Scenario: Leader survives past prior election timeout
- GIVEN a candidate wins election and transitions to leader
- WHEN a duration exceeding the election timeout elapses or a previously queued `ElectionTimeout` drains from the timer channel
- THEN the leader MUST remain alive and continue serving heartbeats and client operations.

#### Scenario: Follower survives step-down past heartbeat interval
- GIVEN a leader steps down to follower upon observing a higher term
- WHEN a previously armed or queued `HeartbeatTimeout` arrives
- THEN the follower MUST ignore it cleanly without terminating the host.

### Requirement: Raft ReadProbe, AppendEntries reconciliation, commit chunking, and recovery snapshots preserve liveness and monotonicity
r[molten.consensus.raft_replication_and_recovery_liveness] Lagging followers MUST confirm current-term leadership on `ReadProbe` without requiring `last_log_index >= required_index`. `AppendEntries` duplicate `request_ref` validation MUST exclude uncommitted conflicting suffix entries scheduled for truncation. Leader commit advancement MUST chunk within `max_effects_per_step`. Recovery snapshot installation MUST verify boundary term equality before retaining local suffix entries.

#### Scenario: ReadProbe succeeds while one follower lags commit_index
- GIVEN a 3-node cluster where a write committed on nodes A and B while follower C lags by one entry
- WHEN the leader broadcasts `ReadProbe` for a linearizable read
- THEN follower C MUST NOT reject or crash and MUST acknowledge current-term leadership.

#### Scenario: Partitioned follower reconciles retried request_ref
- GIVEN an isolated former leader holds uncommitted `(index 2, term 1, R1)` and the new leader commits `(index 1, term 2, R1)`
- WHEN the partition heals and `AppendEntries` arrives
- THEN the follower MUST truncate the conflicting suffix and accept `(index 1, term 2, R1)` without duplicate-window rejection.

#### Scenario: Large catch-up commit chunks within effect bounds
- GIVEN 128 uncommitted entries become quorum-replicated in one step with `max_effects_per_step = 128`
- WHEN `advance_leader_commit` runs
- THEN commit advancement MUST chunk within the effect budget rather than failing closed permanently.

#### Scenario: Recovery snapshot drops conflicting term suffix
- GIVEN a recovering replica holds uncommitted `(index 2, term 1)` and receives a snapshot at `(last_included_index = 1, last_included_term = 2)`
- WHEN `rejoin::repair::install_snapshot` runs
- THEN it MUST discard the conflicting suffix rather than retaining a term-monotonicity violation.

### Requirement: Reliability service prevents cross-node Teardown/Write replay and enforces fail-stop quarantine
r[molten.consensus.reliability_replay_and_failstop] Local operator operations (`Teardown`) MUST be accepted only when `request.endpoint` matches the local node's endpoint. Signed writes MUST remain replay-protected across leader failover and snapshot compaction. `Host::perform` and `Host::wait` MUST honor fail-stop termination when the replica enters `ReplicaExecutionOutcome::Failed` quarantine.

#### Scenario: Signed Teardown for Node A denies on Node B
- GIVEN a valid signed `Teardown` request addressed to Node A
- WHEN the same signed request is replayed to Node B
- THEN Node B MUST deny the request without shutting down.

#### Scenario: Quarantine failure stops the reliability host
- GIVEN `handle_event` returns `ReplicaExecutionOutcome::Failed` inside `Host::perform`
- WHEN the operation completes
- THEN the host MUST exit fail-stop rather than continuing to serve a stopped replica.
