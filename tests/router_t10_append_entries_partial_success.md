# Test Port Status: t10_append_entries_partial_success

## Status
The test has been ported to `/home/brittonr/git/aspen/tests/router_t10_append_entries_partial_success.rs` but **cannot compile** yet due to missing AspenRouter methods.

## Missing AspenRouter Methods

To complete this port, the following methods need to be added to `AspenRouter` in `/home/brittonr/git/aspen/src/testing/router.rs`:

### 1. `set_append_entries_quota(&mut self, quota: Option<u64>)`

Sets a quota for append_entries RPCs to simulate partial success. When quota is set to `Some(n)`, at most `n` entries will be sent per RPC.

**Implementation requirements:**
- Add `append_entries_quota: StdMutex<Option<u64>>` field to `InnerRouter`
- Initialize it in `InnerRouter::new()` as `StdMutex::new(None)`
- Add import: `use openraft::RPCTypes;`
- Modify `InnerRouter::send_append_entries` to:
  - Make `rpc` parameter mutable
  - Before sending, check quota and truncate `rpc.entries` if needed
  - Track how many entries were sent and reduce quota
  - If truncated, return `AppendEntriesResponse::PartialSuccess(truncated_log_id)` instead of `Success`

**Reference implementation:** `/home/brittonr/git/aspen/openraft/tests/tests/fixtures/mod.rs` lines 315-318, 990-1044

### 2. `client_request_many(&self, target: NodeId, client_id: &str, count: usize) -> Result<u64>`

Sends `count` client write requests to the specified node.

**Implementation:**
```rust
pub async fn client_request_many(
    &self,
    target: NodeId,
    client_id: &str,
    count: usize,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    for idx in 0..count {
        let key = format!("{}_{}", client_id, idx);
        let value = format!("value_{}", idx);
        self.write(&target, key, value).await?;
    }
    Ok(count as u64)
}
```

### 3. `Clone` trait for `AspenRouter`

The test spawns a task that needs to clone the router. Add `#[derive(Clone)]` to `AspenRouter`.

**Implementation:**
```rust
#[derive(Clone)]
pub struct AspenRouter {
    config: Arc<Config>,
    inner: Arc<InnerRouter>,
}
```

## Test Overview

The test validates that:
1. With a quota of 2, only 2 log entries are replicated per append_entries RPC
2. Client writes block when quota is exhausted
3. Increasing the quota allows more entries to be replicated
4. The cluster eventually reaches consistency

## Next Steps

1. Add the three missing methods/features to `AspenRouter`
2. Ensure the `new_cluster` method exists (it appears to already exist based on grep results)
3. Run: `nix develop -c cargo nextest run test_append_entries_partial_success`
4. Verify the test passes
