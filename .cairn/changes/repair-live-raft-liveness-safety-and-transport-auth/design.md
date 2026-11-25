# Design

## Benign Stale/Role-Mismatched Timer Handling

- In `src/fabric_consistency/raft/time/mod.rs`:
  - `arm_heartbeat_timer` MUST abort `self.election_handle` before spawning the heartbeat timer.
  - `arm_election_timer` MUST abort `self.heartbeat_handle` before spawning the election timer.
- In `src/fabric_consistency/raft/transition/election.rs` and `src/reliability/service/host/runtime.rs`:
  - Because Tokio channel delivery is asynchronous, an already-queued `ElectionTimeout` or `HeartbeatTimeout` can still arrive after a role transition or `AppendEntries` timer reset. Return a typed benign `Ignored` / no-op transition (emitting zero protocol effects and preserving state) for stale `timer_ref` or role-mismatched timer ticks instead of `Err`/`Denied` that terminates `Host::serve_bound`.

## Raft Protocol Safety and Liveness Fixes

- **Lagging-follower `ReadProbe` (`src/fabric_consistency/raft/transition/message/read.rs`):** Remove the `required_index > last_log_index` rejection. In Raft ReadIndex, a follower confirms leadership for `input.term` if `input.term >= state.current_term` (updating term/stepping down as needed via `observe_higher_term` and persisting term changes before replying), independent of whether the follower's log has caught up to `required_index`.
- **Duplicate `request_ref` validation across conflicting suffix (`src/fabric_consistency/raft/transition/message/replication.rs`, `validation/p000/body.rs`):** Determine the conflict/truncation boundary before checking duplicate `request_ref`s, and validate incoming entries only against retained log entries (`existing.index <= truncate_from_exclusive`) plus earlier entries within the same incoming batch.
- **Chunked commit advancement and log-bound safety (`replication.rs`, `runtime.rs`):** Cap each commit-advancement step to `max_batch = (profile.max_effects_per_step - 4)` entries (leaving room for `PersistCommit`, `ApplyCommitted`, and both intent fences) and schedule continuation until `commit_index` reaches `highest_quorum_index`. On followers approaching `max_log_entries`, allow `commit_index` advancement and snapshot compaction without rejecting valid `AppendEntries`.
- **Recovery snapshot suffix retention (`rejoin/repair.rs`):** Check `term_at(snapshot.last_included_index) == Some(snapshot.last_included_term)` before retaining entries with `index > snapshot.last_included_index`; clear the suffix and set `is_suffix_retained: false` on mismatch, matching `message/snapshot.rs`.

## Authenticated Raft Transport, Fault Isolation, and Replay Fencing

- **Actual peer identity verification (`fabric_transport/cross_process/iroh/parts/shell/p001/body.rs`, `raft/iroh/mod.rs`):** Extract the authenticated remote QUIC/Iroh `EndpointId` from the accepted connection, derive `observed_peer_context_ref` from the admitted peer table rather than echoing `self.endpoint_artifact.descriptor.expected_peer_context_ref`, and verify in `received_event` that `received.evidence.remote_transport_identity_ref` matches the admitted identity of `envelope.from`.
- **Per-connection error isolation (`raft/iroh/pump.rs`):** Treat remote handshake stalls, peer-close timeouts, and malformed peer frames as counted per-connection ingress drops/diagnostics rather than fatal `Err` exits from `ReplicaIngressPump`.
- **Operator replay & quarantine enforcement (`reliability/service/host/runtime.rs`, `runtime/operation.rs`, `operator/access.rs`):**
  - Require `request.endpoint == self.admission.endpoint` for local non-forwarded operations (`Phase::Teardown`, local status/control), preventing a signed `Teardown` targeting Node A from being accepted on Nodes B or C.
  - Bound write-request replay protection across leader failover and snapshot compaction (e.g., reject writes older than the compacted dedup horizon or persist replicated client sequence/nonce high-water marks in Raft state/snapshots rather than node-local files only).
  - Propagate `ReplicaExecutionOutcome::Failed` (quarantine) out of `Host::perform` and `Host::wait` to terminate the host per the fail-stop contract, and call `self.snapshot_if_due().await` after `handle_event` in `Host::perform`.
- **Fast-path ledger (`src/fabric_consistency/fastpath/stable.rs`):** Key duplicate detection by `(session_ref, session_sequence)` (verifying matching `command_ref` on retry and rejecting conflicting payloads) rather than global `command_ref`, and require `acknowledged` replicas in `evaluate_stable_view` to be members of the admitted replica set.
