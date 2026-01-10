# Unbounded PendingRequests Maps (Memory Exhaustion)

**Severity:** CRITICAL
**Category:** Resource Bounds
**Date:** 2026-01-10
**Status:** FIXED (2026-01-10)

## Summary

The Pijul sync handler contains 4 HashMaps without size limits, enabling unbounded memory growth.

## Affected Location

**File:** `crates/aspen-pijul/src/handler.rs` (Lines 100-115, 192)

```rust
struct PendingRequests {
    downloading: HashMap<ChangeHash, Instant>,      // UNBOUNDED
    requested: HashMap<ChangeHash, Instant>,        // UNBOUNDED
    channel_checks: HashMap<String, Instant>,       // UNBOUNDED
    awaiting_changes: HashMap<ChangeHash, Vec<PendingChannelUpdate>>, // UNBOUNDED + nested Vec
}

// Line 192 - unbounded vector growth per hash
self.awaiting_changes.entry(hash).or_default().push(update);
```

## Risk

An attacker sending continuous sync requests can:

- Fill maps with unlimited entries
- Accumulate unlimited updates per change hash
- Cause memory exhaustion and service crash

## Recommendation

1. Add size limits and cleanup:

```rust
const MAX_PENDING_CHANGES: usize = 10_000;
const MAX_AWAITING_UPDATES_PER_CHANGE: usize = 100;

impl PendingRequests {
    fn add_awaiting(&mut self, hash: ChangeHash, update: PendingChannelUpdate) -> Result<()> {
        // Check total map size
        if self.awaiting_changes.len() >= MAX_PENDING_CHANGES {
            self.prune_old_entries();
            if self.awaiting_changes.len() >= MAX_PENDING_CHANGES {
                return Err(Error::TooManyPendingChanges);
            }
        }

        // Check per-hash limit
        let entry = self.awaiting_changes.entry(hash).or_default();
        if entry.len() >= MAX_AWAITING_UPDATES_PER_CHANGE {
            return Err(Error::TooManyUpdatesPerChange);
        }
        entry.push(update);
        Ok(())
    }
}
```

2. Add periodic cleanup based on `Instant` timestamps

## Missing Constants

Add to `crates/aspen-constants/src/lib.rs`:

- `MAX_PENDING_CHANGES = 10_000`
- `MAX_AWAITING_UPDATES_PER_CHANGE = 100`

## Fix Applied

The vulnerability has been fixed with the following changes:

### 1. Added constants to `crates/aspen-pijul/src/constants.rs`

```rust
/// Maximum number of pending changes tracked across all maps.
/// Tiger Style: Prevents memory exhaustion from unbounded map growth.
pub const MAX_PENDING_CHANGES: usize = 10_000;

/// Maximum number of channel updates waiting for a single change.
/// Tiger Style: Limits memory per awaiting_changes entry.
pub const MAX_AWAITING_UPDATES_PER_CHANGE: usize = 100;
```

### 2. Added error variants to `crates/aspen-pijul/src/error.rs`

```rust
/// Too many pending sync requests.
TooManyPendingChanges { count: usize, max: usize },

/// Too many channel updates waiting for a single change.
TooManyAwaitingUpdates { hash: String, count: usize, max: usize },
```

### 3. Updated `PendingRequests` in `crates/aspen-pijul/src/handler.rs`

Added bounds checking to all insertion methods:

- `total_count()` - calculates total entries across tracking maps
- `ensure_capacity()` - cleanup old entries and check limits before insertion
- `mark_downloading()` - now returns `bool`, checks `MAX_PENDING_CHANGES`
- `mark_requested()` - now returns `bool`, checks `MAX_PENDING_CHANGES`
- `mark_channel_checked()` - now returns `bool`, checks `MAX_PENDING_CHANGES`
- `await_change()` - now returns `bool`, checks both `MAX_PENDING_CHANGES` and `MAX_AWAITING_UPDATES_PER_CHANGE`

When at capacity, methods:

1. First attempt to cleanup old/expired entries
2. If still at capacity, log a warning and reject the insertion
3. Return `false` to indicate the entry was not added

This fail-safe approach ensures memory bounds while gracefully degrading under load.
