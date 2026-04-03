## Tasks

### Task 1: Fix pipeline status — job-not-found treated as incomplete

**File**: `crates/aspen-ci/src/orchestrator/pipeline/status.rs`
**Spec**: pipeline-status-validation

Change `check_active_job_statuses()` at line ~142: `Ok(None)` should set `all_completed = false` (not silently skip). Log a warning so the condition is visible. Add a unit test with a mock job manager that returns None for some jobs.

### Task 2: Deduplicate CI triggers — leader-only processing

**File**: `crates/aspen-ci/src/trigger/service.rs`
**Spec**: ci-pipeline-dedup

In `handle_trigger()`, before calling `self.pipeline_starter.start_pipeline()`, check if this node is the Raft leader. The `TriggerService` needs access to a `ClusterController` (or a simple `is_leader` callback). If not the leader, log and return Ok. Update `TriggerService::new()` to accept the callback. Wire it in `src/bin/aspen_node/setup/`.

### Task 3: Add length-prefix framing to forwarded RPC responses

**File**: `crates/aspen-raft/src/iroh_write_forwarder.rs`
**Spec**: length-framed-forwarding

In `send_rpc_on_stream`:

- **Receiver side**: After `recv.read_to_end()`, read 4-byte BE u32 length prefix first, validate ≤ `MAX_CLIENT_MESSAGE_SIZE`, then `recv.read_exact(len)` for the payload. If the stream ends early, return a framing error (not deserialization).
- **Sender side**: The response is sent by the protocol handler in `crates/aspen-rpc-handlers/`. Find where CLIENT_ALPN responses are written and prepend the 4-byte length. Both sides must change together.

Add regression test: mock a stream that delivers partial data and verify the framing error is returned.

### Task 4: Add length-prefix framing to protocol handler responses

**File**: `crates/aspen-rpc-handlers/src/handler.rs` (or wherever CLIENT_ALPN stream responses are written)
**Spec**: length-framed-forwarding

The protocol handler that receives CLIENT_ALPN requests and writes responses needs to prepend the 4-byte BE u32 length to the serialized response. Find the write path and add `send.write_all(&(response_bytes.len() as u32).to_be_bytes())` before `send.write_all(&response_bytes)`.

### Task 5: Validate with 3-node dogfood

**Depends**: Tasks 1-4

Run `ASPEN_NODE_COUNT=3 nix run .#dogfood-local -- full-loop` end-to-end. Verify:

- Single pipeline triggered (not duplicate)
- All stages complete (check, build, test, deploy)
- No "Hit the end of buffer" errors in logs
- Deploy succeeds (uses the self-connect fix from prior commit)
