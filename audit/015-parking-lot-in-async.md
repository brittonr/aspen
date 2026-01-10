# parking_lot::RwLock in Async Context

**Severity:** MEDIUM
**Category:** Concurrency
**Date:** 2026-01-10

## Summary

`parking_lot::RwLock` (synchronous lock) is used in async context, causing blocking behavior.

## Affected Location

**File:** `crates/aspen-pijul/src/handler.rs` (Lines 39, 266, 270)

```rust
use parking_lot::RwLock;

pub struct PijulSyncHandler<B: BlobStore, K: KeyValueStore + ?Sized> {
    pending: RwLock<PendingRequests>,
    watched_repos: RwLock<HashSet<RepoId>>,
}

fn on_announcement(&self, announcement: &PijulAnnouncement, signer: &PublicKey) {
    // Synchronous callback
    let mut pending = self.pending.write();  // BLOCKING call
    // ...
    drop(pending);
}
```

## Risk

- Gossip event processing stalls if lock is contended
- Entire gossip receive loop blocks
- Cascading slowdowns across the system

## Recommendation

Option 1: Use `tokio::sync::RwLock` if callback can be async:

```rust
use tokio::sync::RwLock;
```

Option 2: If callback must be sync, use `tokio::task::spawn_blocking`:

```rust
async fn handle_announcement(&self, ...) {
    let pending = self.pending.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = pending.write();
        // blocking operation
    }).await?;
}
```

Option 3: Use `try_write()` with fallback:

```rust
fn on_announcement(&self, ...) {
    match self.pending.try_write() {
        Some(mut guard) => { /* process immediately */ },
        None => { /* queue for later processing */ }
    }
}
```
