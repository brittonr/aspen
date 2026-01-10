# Code Duplication in Router Setup

**Severity:** LOW
**Category:** Code Quality
**Date:** 2026-01-10

## Summary

`spawn_router()` and `spawn_router_with_blobs()` contain 55+ lines of duplicate setup code.

## Affected Location

**File:** `src/node/mod.rs` (Lines 414-629)

## Duplicate Code (Lines 415-470)

Both methods contain identical code for:

- Raft handler setup and transmute (lines 415-430)
- Authenticated handler registration (lines 433-456)
- Gossip handler registration (lines 458-463)
- Client RPC handler registration (lines 465-470)

## Recommendation

Extract common setup into helper function:

```rust
fn setup_base_router_handlers(
    &self,
    router: &mut Router,
    raft_core: &Arc<RaftNode>,
) -> Result<()> {
    // Common Raft handler setup
    let raft_core_transport = unsafe { std::mem::transmute(raft_core.clone()) };
    router.register_raft_handler(raft_core_transport)?;

    // Common authenticated handler
    router.register_authenticated_handler(...)?;

    // Common gossip handler
    router.register_gossip_handler(...)?;

    // Common client RPC handler
    router.register_client_rpc_handler(...)?;

    Ok(())
}

pub fn spawn_router(&self, ...) -> Result<RouterHandle> {
    let mut router = Router::new();
    self.setup_base_router_handlers(&mut router, &self.raft)?;
    // spawn_router-specific setup
    Ok(router.spawn())
}

pub fn spawn_router_with_blobs(&self, ...) -> Result<RouterHandle> {
    let mut router = Router::new();
    self.setup_base_router_handlers(&mut router, &self.raft)?;
    // blob-specific setup
    router.register_blob_handler(...)?;
    Ok(router.spawn())
}
```
