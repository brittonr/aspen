# DNS Sync Broadcast Fix - DocsExporter Integration

**Created**: 2025-12-26
**Context**: DNS records stored in KV were not syncing to the DNS server because DocsExporter wasn't receiving broadcasts

## Problem Summary

The DNS CLI commands worked correctly, storing records in the Raft KV store. However, the DNS server's cache remained empty because the sync chain was broken:

```
KV Store -> DocsExporter -> iroh-docs -> DNS sync listener -> DNS cache -> DNS server
           ^^^^^^^^^^^^^^
           THIS WAS BROKEN
```

## Root Cause Analysis

The `SharedRedbStorage` struct had a `log_broadcast` field designed to send `LogEntryPayload` messages to subscribers (like DocsExporter), but the field was:

1. Marked with `#[allow(dead_code)]` - indicating it was never used
2. Had a TODO comment: "TODO: Implement log broadcast for Redb backend (Phase 2)"

The field was passed during construction via `with_broadcast()`, but no code in `SharedRedbStorage` ever called `sender.send()` on the broadcast channel.

**Location of issue**: `src/raft/storage_shared.rs:340-343` (before fix)

```rust
/// Optional broadcast sender for log entry notifications.
/// TODO: Implement log broadcast for Redb backend (Phase 2)
#[allow(dead_code)]
log_broadcast: Option<broadcast::Sender<LogEntryPayload>>,
```

## Solution

Implemented the log broadcast in `SharedRedbStorage::apply()` - the RaftStateMachine trait method called when entries are committed.

### Why `apply()` and not `append()`?

The `append()` method in SharedRedbStorage applies state mutations during log append (single-fsync pattern), but broadcasting should happen in `apply()` because:

1. `apply()` is called when entries are truly **committed** by Raft consensus
2. Subscribers (DocsExporter) should only see committed entries, not speculative ones
3. This matches OpenRaft's semantics where `apply()` signals client-visible commits

### Implementation Details

In `apply()`, for each entry:

1. Convert the entry's payload to a `KvOperation` (using existing `From<AppRequest>` impl)
2. Create a `LogEntryPayload` with index, term, timestamp, and operation
3. Best-effort broadcast via `sender.send()` - failures are logged at debug level

**Key design decisions**:

- **Best-effort**: If no receivers or channel is full, log debug and continue (don't fail Raft)
- **All operations**: Broadcasts all committed entries (Set, Delete, Membership, Blank)
- **DocsExporter filtering**: The DocsExporter already filters for relevant operations (Set/Delete with key prefix matching)

## Files Changed

- `src/raft/storage_shared.rs`:
  - Removed `#[allow(dead_code)]` and TODO comment from `log_broadcast` field
  - Added broadcast logic in `apply()` method (~25 lines)
  - Added test `test_log_broadcast_on_apply` to verify broadcast functionality

## Testing

All 77 related tests pass:

- `raft::storage_shared::tests::*` (4 tests)
- `docs::exporter::tests::*` (7 tests)
- `dns::*` (50+ tests)
- `docs_sync_*` integration tests (3 tests)

The new test `test_log_broadcast_on_apply` specifically verifies:

1. Creating storage with a broadcast channel
2. Calling `apply()` with a test entry
3. Receiving the broadcast on the receiver
4. Verifying the `KvOperation` contents match

## Architecture Impact

This fix completes the Raft -> DocsExporter -> iroh-docs data flow:

```
Raft commit via OpenRaft
       |
       v
SharedRedbStorage.apply()
       |
       +--> broadcast::Sender<LogEntryPayload>
              |
              v
       DocsExporter.spawn() loop
              |
              +--> iroh_docs.set() for Set/Delete operations
                     |
                     v
              iroh-docs CRDT replication
                     |
                     v
              DNS sync listener (subscribes to doc events)
                     |
                     v
              AspenDnsClient cache update
                     |
                     v
              DNS server responds with cached records
```

## Future Considerations

1. **Backpressure**: Currently best-effort - if DocsExporter lags behind, it will get `Lagged` error from the broadcast channel and need to request full sync from storage

2. **Filtering at source**: Could optimize by only broadcasting entries that match DocsExporter's key prefix, but current approach is simple and works

3. **Multiple subscribers**: The broadcast channel already supports multiple receivers - other components could subscribe to log entries if needed
