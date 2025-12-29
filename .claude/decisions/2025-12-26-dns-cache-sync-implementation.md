# DNS Cache Sync Implementation

**Date**: 2025-12-26
**Status**: Implemented

## Problem

The DNS server in Aspen reads from `AspenDnsClient`, but the client was being created empty in `aspen-node.rs` without any connection to the docs sync layer. This meant DNS records stored in the cluster via Raft consensus and exported to iroh-docs were not being populated into the DNS client cache, rendering the DNS server unable to serve any records.

## Solution

Implemented `spawn_dns_sync_listener()` function that connects the iroh-docs sync layer to the DNS client cache, enabling real-time DNS record synchronization from the cluster.

### Architecture

```
Server Side (Aspen Cluster)           Client/Node Side
+-------------------------------+     +-------------------------------+
| DnsStore (set dns:* keys)     |     | iroh-docs sync subscription   |
|             |                 |     |             |                 |
|             v                 |     |             v                 |
| DocsExporter (to iroh-docs)   | --> | spawn_dns_sync_listener()     |
|             |                 |     |             |                 |
|             v                 |     |             v                 |
| iroh-docs namespace           |     | AspenDnsClient.process_sync() |
+-------------------------------+     |             |                 |
                                      |             v                 |
                                      | DnsProtocolServer (UDP/TCP)   |
                                      +-------------------------------+
```

### Key Components

1. **`spawn_dns_sync_listener()`** (`src/dns/client.rs:615-772`)
   - Subscribes to iroh-docs sync events for the namespace
   - Filters for `dns:*` keys only
   - Fetches content from blob store by hash
   - Forwards entries to `AspenDnsClient.process_sync_entry()`
   - Handles tombstones (deletions) properly
   - Updates sync status (Syncing -> Synced -> Stale)
   - Graceful shutdown via CancellationToken

2. **Wiring in `aspen-node.rs`** (lines 672-693)
   - Creates DNS client
   - Checks if both `docs_sync` and `blob_store` are available
   - Spawns sync listener if resources are available
   - Logs appropriate status messages

### Tiger Style Compliance

- **Bounded channel**: 1000 event buffer to prevent unbounded memory use
- **Explicit filtering**: Only processes `dns:*` keys, ignores others
- **Non-blocking**: Blob fetches are async, skips unavailable content
- **Graceful shutdown**: CancellationToken for coordinated cleanup
- **Status tracking**: Explicit sync status (Disconnected -> Syncing -> Synced)

### Error Handling

- Failed sync subscriptions: Logged as warning, DNS serves static records only
- Missing blob content: Skipped with debug log, retried on next sync
- Parse failures: Logged as warning, continues processing other entries
- Channel errors: Logs error, sets status to Disconnected, stops listener

## Files Changed

1. `src/dns/client.rs` - Added `spawn_dns_sync_listener()` function
2. `src/dns/mod.rs` - Exported `spawn_dns_sync_listener`
3. `src/bin/aspen-node.rs` - Wired up DNS sync listener to docs_sync layer

## Testing

- All 63 DNS unit tests pass
- All 1787 tests in quick profile pass
- No regressions in existing functionality

## Future Considerations

1. **Initial sync**: Currently marks as "Synced" after first successful entry. Could add option to wait for initial sync completion before serving.

2. **Staleness detection**: The client has `check_staleness()` method. Could add periodic health check task.

3. **Zone filtering**: The DNS client ticket already supports zone filters. The sync listener respects these filters.

4. **Metrics**: Cache stats are tracked. Could expose via Prometheus.
