## Context

In 3-node dogfood, after a 34K-object git push the Raft log grows to 600K+ entries. Snapshot transfers to followers saturate QUIC bandwidth. Forwarded RPCs (reads/scans from followers to leader) get truncated responses, causing postcard deserialization to fail with "Hit the end of buffer". Workers can't dequeue jobs, workflows stall, and the pipeline falsely reports success.

Three bugs compound:

1. `send_rpc_on_stream` in `IrohWriteForwarder` uses `send.finish()` + `recv.read_to_end()` with no length-prefix framing. If the QUIC stream resets mid-transfer, the receiver gets partial bytes and postcard fails with a confusing deserialization error rather than a clear framing error.

2. `announce_ref_update()` calls both `notify_local_handler()` (direct) and `gossip.broadcast()` (to all nodes). Every node's `CiTriggerHandler` receives the announcement and calls `start_pipeline()` independently. Result: N pipelines for N nodes.

3. `check_active_job_statuses()` in `status.rs` line 142 treats `Ok(None)` (job not found) as completed. When forwarded reads fail or return stale data, jobs appear "not found" and the pipeline is marked Success despite unfinished work.

## Goals / Non-Goals

**Goals:**

- Fix forwarded RPC truncation detection so partial responses are retried, not deserialized
- Deduplicate CI pipeline triggers so each commit+repo gets exactly one pipeline run
- Fix pipeline status validation so "job not found" is not treated as success

**Non-Goals:**

- Reducing QUIC bandwidth usage from snapshot transfers (separate optimization)
- Making forwarded RPCs survive indefinitely under load (retry + backoff is sufficient)
- Changing the gossip broadcast mechanism itself

## Decisions

### D1: Length-prefix framing for forwarded RPCs

Add a 4-byte big-endian length prefix before the serialized response in `send_rpc_on_stream`. The receiver reads the length first, then reads exactly that many bytes. If the stream ends before the full message, it's a clear truncation error (not a confusing postcard error) that triggers retry.

**Location**: `crates/aspen-raft/src/iroh_write_forwarder.rs` — both the sender (protocol handler) and the receiver (`send_rpc_on_stream`).

**Wire format**: `[4 bytes: payload length BE u32][payload bytes]`. The sender writes the framed response; the receiver reads 4 bytes, validates length ≤ `MAX_CLIENT_MESSAGE_SIZE`, then reads exactly that many bytes.

**Compatibility**: The protocol handler that sends responses is in `crates/aspen-rpc-handlers/`. The forwarder that receives them is in `crates/aspen-raft/`. Both run the same binary version in a cluster, so the change is atomic per deployment. No backwards compatibility needed — all nodes upgrade together.

### D2: Deduplicate CI triggers via leader-only processing

The `CiTriggerHandler.on_announcement()` currently fires on every node that receives the gossip. Fix: only process triggers on the Raft leader. Before calling `start_pipeline()`, check `controller.get_metrics()` — if this node is not the leader, drop the trigger silently.

**Location**: `crates/aspen-ci/src/trigger/service.rs` — `handle_trigger()` method.

**Alternative considered**: CAS-based dedup in KV (write `_ci:trigger:{repo}:{commit}` with CAS). Rejected because the leader check is simpler and handles the common case (only one node is leader at any time). Edge case: leadership changes mid-trigger may cause a missed trigger, but the gossip replay buffer handles this.

### D3: Fix "job not found" status logic

Change `check_active_job_statuses()` to treat `Ok(None)` as "not completed" instead of "completed". A missing job means the read failed or the job hasn't been replicated yet — not that the job finished.

**Location**: `crates/aspen-ci/src/orchestrator/pipeline/status.rs` — line 142.

## Risks

- **D1**: If the protocol handler doesn't add the length prefix (missed code path), the receiver will read garbage as the length. Mitigated by adding a sanity check on the length value.
- **D2**: Leadership changes during trigger processing could cause a trigger to be processed by two nodes (old leader started, new leader replays). The KV-backed pipeline creation should handle this via idempotent run creation.
- **D3**: If jobs are genuinely deleted (cleanup), pipelines could get stuck as "running" forever. Mitigated by the existing pipeline timeout mechanism.
