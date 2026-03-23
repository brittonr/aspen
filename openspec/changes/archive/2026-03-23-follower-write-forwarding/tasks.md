## Tasks

### 1. Add WriteForwarder trait and RaftNode field

- [x] Create `crates/aspen-raft/src/write_forwarder.rs` with `WriteForwarder` trait
- [x] Add `write_forwarder: Option<Arc<dyn WriteForwarder>>` field to `RaftNode`
- [x] Add `pub fn set_write_forwarder(&mut self, forwarder: Arc<dyn WriteForwarder>)` method
- [x] Re-export trait from `crates/aspen-raft/src/lib.rs`

### 2. Implement forwarding in KV write path

- [x] In `kv_store.rs` `write()`, catch `ForwardToLeader` error
- [x] If `self.write_forwarder` is set and leader hint is available, call `forwarder.forward_write()`
- [x] If forwarding fails or no forwarder, return `NotLeader` as before
- [x] Bypass write batcher for forwarded writes (already handled — batcher only runs on leader)
- [x] Add `info!` log when forwarding a write

### 3. Implement IrohWriteForwarder

- [x] Create `IrohWriteForwarder` struct holding `iroh::Endpoint` + connection pool reference
- [x] Connect to leader via CLIENT_ALPN, send `WriteRequest` as `ClientRpcRequest`, parse response
- [x] 30s timeout on the forwarded write
- [x] Handle the case where the forwarding target returns `NotLeader` (return it to caller)

### 4. Wire up forwarder during cluster bootstrap

- [x] In cluster bootstrap (after `RaftNode` and endpoint are created), create `IrohWriteForwarder`
- [x] Call `raft_node.set_write_forwarder()` before workers start
- [x] Ensure the forwarder has access to the endpoint and peer address resolution

### 5. Downgrade worker stats write failures to DEBUG

- [x] In worker service stats writing code, change `warn!("failed to write worker stats")` to `debug!`

### 6. Test

- [x] Unit test: mock `WriteForwarder` that returns success, verify `write()` succeeds on follower
- [x] Unit test: mock `WriteForwarder` that returns `NotLeader`, verify error propagates
- [ ] Re-run 3-node dogfood and verify CI pipeline survives leader change
