# Missing Task Abort on Timeout

**Severity:** MEDIUM
**Category:** Concurrency
**Date:** 2026-01-10

## Summary

When task timeouts occur in `tokio::select!`, the task handle is dropped without calling `.abort()`, leaving the task running.

## Affected Location

**File:** `crates/aspen-pijul/src/sync.rs` (Lines 401-411)

```rust
if let Some(task) = self.announcer_task.lock().await.take() {
    tokio::select! {
        result = task => {
            if let Err(e) = result {
                warn!("announcer task panicked: {}", e);
            }
        }
        _ = tokio::time::sleep(timeout) => {
            warn!("announcer task did not complete within timeout");
            // BUG: task JoinHandle is dropped without abort
        }
    }
}
```

## Risk

- Task continues executing after timeout
- Task may reference freed memory
- Resource exhaustion from orphaned tasks
- Unexpected behavior during shutdown

## Recommendation

```rust
if let Some(task) = self.announcer_task.lock().await.take() {
    tokio::select! {
        result = task => {
            if let Err(e) = result {
                warn!("announcer task panicked: {}", e);
            }
        }
        _ = tokio::time::sleep(timeout) => {
            warn!("announcer task did not complete within timeout, aborting");
            task.abort();  // Properly abort the task
        }
    }
}
```
