## Tasks

### 1. Add WriteForwarder trait and RaftNode field

- [ ] Create `crates/aspen-raft/src/write_forwarder.rs` with `WriteForwarder` trait
- [ ] Add `write_forwarder: Option<Arc<dyn WriteForwarder>>` field to `RaftNode`
- [ ] Add `pub fn set_write_forwarder(&mut self, forwarder: Arc<dyn WriteForwarder>)` method
- [ ] Re-export trait from `crates/aspen-raft/src/lib.rs`

### 2. Implement forwarding in KV write path

- [ ] In `kv_store.rs` `write()`, catch `ForwardToLeader` error
- [ ] If `self.write_forwarder` is set and leader hint is available, call `forwarder.forward_write()`
- [ ] If forwarding fails or no forwarder, return `NotLeader` as before
- [ ] Bypass write batcher for forwarded writes (already handled — batcher only runs on leader)
- [ ] Add `info!` log when forwarding a write

### 3. Implement IrohWriteForwarder

- [ ] Create `IrohWriteForwarder` struct holding `iroh::Endpoint` + connection pool reference
- [ ] Connect to leader via CLIENT_ALPN, send `WriteRequest` as `ClientRpcRequest`, parse response
- [ ] 30s timeout on the forwarded write
- [ ] Handle the case where the forwarding target returns `NotLeader` (return it to caller)

### 4. Wire up forwarder during cluster bootstrap

- [ ] In cluster bootstrap (after `RaftNode` and endpoint are created), create `IrohWriteForwarder`
- [ ] Call `raft_node.set_write_forwarder()` before workers start
- [ ] Ensure the forwarder has access to the endpoint and peer address resolution

### 5. Downgrade worker stats write failures to DEBUG

- [ ] In worker service stats writing code, change `warn!("failed to write worker stats")` to `debug!`

### 6. Test

- [ ] Unit test: mock `WriteForwarder` that returns success, verify `write()` succeeds on follower
- [ ] Unit test: mock `WriteForwarder` that returns `NotLeader`, verify error propagates
- [ ] Re-run 3-node dogfood and verify CI pipeline survives leader change
