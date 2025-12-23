# Implementation Plan: Gossip Discovery Fixes

**Created**: 2025-12-19
**Author**: Claude Code
**Branch**: v3

## Executive Summary

This plan addresses three issues in the Aspen gossip discovery subsystem:

1. **Gossip fatal on transient errors** - The receiver task breaks on any `Some(Err(e))` from the gossip stream, making transient network errors fatal
2. **Peer announcement flooding** - Announcer broadcasts every 10s without rate limiting on the sending side
3. **ALPN constant duplication** - `RAFT_ALPN` and `CLIENT_ALPN` are defined in both `src/protocol_handlers.rs` and `tests/support/mock_iroh.rs`

## Issue 1: Gossip Fatal on Transient Errors

### Current Behavior

In `gossip_discovery.rs:603-605`:

```rust
Some(Err(e)) => {
    tracing::error!("gossip receiver error: {}", e);
    break;  // FATAL - exits receiver loop
}
```

Any error from the gossip stream causes the entire receiver task to exit. This is problematic because:

- Transient network issues (connectivity blips, relay reconnection) terminate discovery permanently
- No automatic recovery without full node restart
- Silent degradation - node stops discovering peers

### Proposed Solution

Implement exponential backoff with bounded retry for transient errors, reusing the existing `calculate_backoff_duration` pure function from `src/raft/pure.rs`.

**Changes to `src/cluster/gossip_discovery.rs`**:

1. Add import for backoff calculation:

```rust
use crate::raft::pure::calculate_backoff_duration;
```

2. Add constants to `src/raft/constants.rs`:

```rust
/// Maximum gossip stream error retries before giving up (5 retries).
///
/// Tiger Style: Bounded retry count prevents infinite retry loops.
/// After this many consecutive errors, the receiver task exits.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task error handling
pub const GOSSIP_MAX_STREAM_RETRIES: u32 = 5;

/// Gossip stream error backoff durations in seconds.
///
/// Tiger Style: Fixed backoff progression: 1s, 2s, 4s, 8s, 16s (capped).
/// Exponential backoff prevents overwhelming a recovering network.
///
/// Used in:
/// - `gossip_discovery.rs`: Receiver task backoff calculation
pub const GOSSIP_STREAM_BACKOFF_SECS: [u64; 5] = [1, 2, 4, 8, 16];
```

3. Modify the receiver task to track consecutive errors and apply backoff:

```rust
// Inside receiver task, add state tracking
let mut consecutive_errors: u32 = 0;
let backoff_durations: Vec<Duration> = GOSSIP_STREAM_BACKOFF_SECS
    .iter()
    .map(|s| Duration::from_secs(*s))
    .collect();

// In the match arm for errors:
Some(Err(e)) => {
    consecutive_errors += 1;

    if consecutive_errors > GOSSIP_MAX_STREAM_RETRIES {
        tracing::error!(
            "gossip receiver exceeded max retries ({}), giving up: {}",
            GOSSIP_MAX_STREAM_RETRIES,
            e
        );
        break;
    }

    let backoff = calculate_backoff_duration(
        consecutive_errors as usize - 1,
        &backoff_durations
    );

    tracing::warn!(
        "gossip receiver error (retry {}/{}), backing off for {:?}: {}",
        consecutive_errors,
        GOSSIP_MAX_STREAM_RETRIES,
        backoff,
        e
    );

    tokio::time::sleep(backoff).await;
    // Note: continue instead of break - stream may recover
    continue;
}
```

4. Reset error counter on successful message:

```rust
Some(Ok(Event::Received(msg))) => {
    consecutive_errors = 0;  // Reset on success
    // ... rest of handling
}
```

### Edge Cases

- **Permanent stream closure**: After `GOSSIP_MAX_STREAM_RETRIES` (5) consecutive errors, the receiver exits
- **Intermittent errors**: A single success resets the counter, allowing indefinite operation with occasional failures
- **Rapid transient errors**: Backoff prevents CPU spinning and log flooding

## Issue 2: Peer Announcement Flooding

### Current Behavior

The announcer task broadcasts every 10 seconds unconditionally:

- No tracking of announcement success/failure
- No rate limiting based on announcement type or network conditions
- In a 100-node cluster: 100 nodes x 6 announcements/min = 600 announcements/min cluster-wide

The **receiver** has rate limiting (HIGH-6 security enhancement), but the **sender** does not.

### Proposed Solution

Add per-type rate limiting on the announcer side with adaptive backoff on failure.

**Changes to `src/raft/constants.rs`**:

```rust
/// Minimum interval between peer announcements (10 seconds).
///
/// Tiger Style: Fixed floor prevents announcement flooding.
/// This is the normal announcement interval when network is healthy.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task interval
pub const GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS: u64 = 10;

/// Maximum interval between peer announcements (60 seconds).
///
/// Tiger Style: Upper bound on backoff prevents stale discovery.
/// Used when announcements are failing to avoid network flooding.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive interval
pub const GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS: u64 = 60;

/// Consecutive announcement failures before increasing interval (3).
///
/// Tiger Style: Bounded failures before adaptive backoff.
/// Prevents flooding the network when broadcast consistently fails.
///
/// Used in:
/// - `gossip_discovery.rs`: Announcer task adaptive logic
pub const GOSSIP_ANNOUNCE_FAILURE_THRESHOLD: u32 = 3;
```

**Changes to `src/cluster/gossip_discovery.rs`**:

1. Add adaptive interval tracking to the announcer task:

```rust
let mut consecutive_failures: u32 = 0;
let mut current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
let mut ticker = interval(Duration::from_secs(current_interval_secs));
ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

// In the announcement logic:
match signed.to_bytes() {
    Ok(bytes) => {
        match announcer_sender.broadcast(bytes.into()).await {
            Ok(()) => {
                tracing::trace!(
                    "broadcast signed peer announcement for node_id={}",
                    announcer_node_id
                );
                // Reset on success
                if consecutive_failures > 0 {
                    consecutive_failures = 0;
                    current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
                    ticker = interval(Duration::from_secs(current_interval_secs));
                    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    tracing::debug!(
                        "announcement succeeded, resetting interval to {}s",
                        current_interval_secs
                    );
                }
            }
            Err(e) => {
                consecutive_failures += 1;
                tracing::warn!(
                    "failed to broadcast peer announcement ({}/{}): {}",
                    consecutive_failures,
                    GOSSIP_ANNOUNCE_FAILURE_THRESHOLD,
                    e
                );

                // Increase interval after threshold failures
                if consecutive_failures >= GOSSIP_ANNOUNCE_FAILURE_THRESHOLD {
                    let new_interval = (current_interval_secs * 2)
                        .min(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS);
                    if new_interval != current_interval_secs {
                        current_interval_secs = new_interval;
                        ticker = interval(Duration::from_secs(current_interval_secs));
                        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                        tracing::info!(
                            "increasing announcement interval to {}s due to failures",
                            current_interval_secs
                        );
                    }
                }
            }
        }
    }
    // ... serialization error handling unchanged
}
```

### Design Rationale

- **Sender-side rate limiting**: Complements receiver-side rate limiting for defense in depth
- **Adaptive interval**: Network issues cause backoff (10s -> 20s -> 40s -> 60s cap)
- **Fast recovery**: Single success resets to minimum interval
- **Bounded maximum**: 60s cap ensures reasonable discovery latency

## Issue 3: ALPN Constant Duplication

### Current Locations

**Source definitions** (`src/protocol_handlers.rs:283-296`):

```rust
pub const RAFT_ALPN: &[u8] = b"raft-rpc";
pub const RAFT_AUTH_ALPN: &[u8] = b"raft-auth";
pub use crate::raft::log_subscriber::LOG_SUBSCRIBER_ALPN;
pub const CLIENT_ALPN: &[u8] = b"aspen-client";
```

**Test duplicates** (`tests/support/mock_iroh.rs:71-75`):

```rust
/// ALPN protocol identifier for Raft RPC (matches protocol_handlers.rs).
pub const RAFT_ALPN: &[u8] = b"raft-rpc";
/// ALPN protocol identifier for Client RPC (matches protocol_handlers.rs).
pub const CLIENT_ALPN: &[u8] = b"aspen-client";
```

### Proposed Solution

Remove duplicates from `tests/support/mock_iroh.rs` and import from the main crate.

**Changes to `tests/support/mock_iroh.rs`**:

1. Remove local definitions (lines 71-75):

```rust
// DELETE:
// /// ALPN protocol identifier for Raft RPC (matches protocol_handlers.rs).
// pub const RAFT_ALPN: &[u8] = b"raft-rpc";
// /// ALPN protocol identifier for Client RPC (matches protocol_handlers.rs).
// pub const CLIENT_ALPN: &[u8] = b"aspen-client";
```

2. Add import at the top of the file:

```rust
use aspen::{CLIENT_ALPN, RAFT_ALPN};
```

3. Make the constants available for tests that import from mock_iroh:

```rust
// Re-export for test convenience
pub use aspen::{CLIENT_ALPN, RAFT_ALPN};
```

### Verification

After changes, grep for ALPN definitions should show:

- `src/protocol_handlers.rs` - canonical definitions
- `src/raft/log_subscriber.rs:37` - `LOG_SUBSCRIBER_ALPN` definition
- `tests/support/mock_iroh.rs` - re-exports only (no local `const` definitions)

## Implementation Order

1. **ALPN consolidation** (lowest risk, no behavior change)
   - Remove duplicates from mock_iroh.rs
   - Add re-exports
   - Run `cargo test` to verify no breakage

2. **Gossip receiver backoff** (medium risk, changes error behavior)
   - Add constants to `src/raft/constants.rs`
   - Modify receiver task with backoff logic
   - Add unit tests for consecutive error handling
   - Run integration tests

3. **Announcer rate limiting** (medium risk, changes timing behavior)
   - Add constants to `src/raft/constants.rs`
   - Modify announcer task with adaptive interval
   - Add unit tests for interval adaptation
   - Run smoke test with failure injection

## Testing Strategy

### Unit Tests (to add)

```rust
#[test]
fn test_gossip_receiver_backoff_progression() {
    // Verify backoff increases with consecutive errors
}

#[test]
fn test_gossip_receiver_backoff_resets_on_success() {
    // Verify success resets consecutive error count
}

#[test]
fn test_announcer_interval_increases_on_failure() {
    // Verify interval doubles after threshold failures
}

#[test]
fn test_announcer_interval_resets_on_success() {
    // Verify single success resets to minimum interval
}

#[test]
fn test_announcer_interval_capped_at_maximum() {
    // Verify interval never exceeds GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS
}
```

### Integration Tests

- Existing `tests/gossip_e2e_integration.rs` and `tests/gossip_integration_test.rs` should pass unchanged
- Add failure injection test for transient error recovery

## Risk Assessment

| Change | Risk | Mitigation |
|--------|------|------------|
| ALPN consolidation | Low | Compile-time verification; values identical |
| Receiver backoff | Medium | Bounded retries; existing tests; no behavior change on success path |
| Announcer rate limiting | Medium | Conservative thresholds; resets on success; integration tests |

## Rollback Strategy

All changes are contained in two files:

- `src/cluster/gossip_discovery.rs` - revert to previous version
- `src/raft/constants.rs` - remove new constants (no other code depends on them initially)

Test file changes (`tests/support/mock_iroh.rs`) are purely cleanup and can be reverted independently.

## Files to Modify

1. `src/raft/constants.rs` - Add new gossip constants (6 new constants)
2. `src/cluster/gossip_discovery.rs` - Implement backoff and rate limiting
3. `tests/support/mock_iroh.rs` - Remove duplicate ALPN constants, add re-exports

## Success Criteria

1. All existing tests pass (`cargo nextest run`)
2. Gossip receiver continues operating through transient errors
3. Announcer adapts interval based on network conditions
4. No duplicate ALPN constant definitions in codebase
5. `cargo clippy --all-targets -- --deny warnings` passes
