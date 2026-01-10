# Locks Held Across Await Points (Deadlock Risk)

**Severity:** CRITICAL
**Category:** Concurrency
**Date:** 2026-01-10
**Status:** FIXED (2026-01-10)

## Summary

Several async functions hold locks across `.await` points, creating potential deadlock conditions.

## Issue 1: PijulSyncService::broadcast (Lines 289-298)

**File:** `crates/aspen-pijul/src/sync.rs`

```rust
async fn broadcast(&self, announcement: PijulAnnouncement) -> PijulResult<()> {
    // ...
    if announcement.is_global() {
        let guard = self.global_subscription.lock().await;  // Lock acquired
        if let Some(ref sub) = *guard {
            sub.sender.broadcast(bytes.into()).await.map_err(|e| ...)?;  // Await while locked!
        }
        // Lock released here
    }
    Ok(())
}
```

**Risk:** The `Mutex<TopicSubscription>` lock is held during the `broadcast().await` call. Other tasks waiting for this lock will block indefinitely if broadcast is slow.

## Issue 2: PijulSyncService repo broadcasts (Lines 302-313)

**File:** `crates/aspen-pijul/src/sync.rs`

```rust
let subscriptions = self.repo_subscriptions.read().await;  // Read lock acquired
if let Some(sub) = subscriptions.get(repo_id) {
    sub.sender.broadcast(bytes.into()).await.map_err(|e| ...)?;  // Await while locked!
}
```

**Risk:** Same issue - read lock held during network operation.

## Recommendation

Release locks before async operations:

```rust
async fn broadcast(&self, announcement: PijulAnnouncement) -> PijulResult<()> {
    // Extract sender outside lock
    let sender = {
        let guard = self.global_subscription.lock().await;
        guard.as_ref().map(|sub| sub.sender.clone())
    };  // Lock released

    // Now broadcast without holding lock
    if let Some(sender) = sender {
        sender.broadcast(bytes.into()).await.map_err(|e| ...)?;
    }
    Ok(())
}
```

## Fix Applied

The vulnerability has been fixed in `crates/aspen-pijul/src/sync.rs`:

### Changes to `broadcast()` method

Both lock acquisition patterns were fixed to clone the sender inside the lock scope, then release the lock before calling `.await`:

**Global topic broadcast:**

```rust
// Extract sender from lock before awaiting (prevents deadlock)
let sender = {
    let guard = self.global_subscription.lock().await;
    match guard.as_ref() {
        Some(sub) => sub.sender.clone(),
        None => return Err(...)
    }
}; // Lock released here

// Now broadcast without holding lock
sender.broadcast(bytes.into()).await.map_err(|e| ...)?;
```

**Repo topic broadcast:**

```rust
// Extract sender from lock before awaiting (prevents deadlock)
let sender = {
    let subscriptions = self.repo_subscriptions.read().await;
    subscriptions.get(repo_id).map(|sub| sub.sender.clone())
}; // Lock released here

// Now broadcast without holding lock
if let Some(sender) = sender {
    sender.broadcast(bytes.into()).await.map_err(|e| ...)?;
}
```

This ensures locks are never held across `.await` points, preventing potential deadlocks.
