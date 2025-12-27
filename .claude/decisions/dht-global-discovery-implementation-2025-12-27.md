# DHT Global Discovery Implementation

**Date:** 2025-12-27
**Author:** Claude Code (ULTRA mode implementation)

## Overview

Implemented real BitTorrent Mainline DHT operations for global content discovery, enabling Aspen nodes to announce and find blob providers across the global DHT network.

## Changes Summary

### 1. Feature-Gated DHT Client (`src/cluster/content_discovery.rs`)

Added a `DhtClient` wrapper struct that encapsulates the mainline crate:

**With `global-discovery` feature:**
- Creates a real `mainline::AsyncDht` instance
- Configures DHT port, server mode, and bootstrap nodes from `ContentDiscoveryConfig`
- Implements `announce_peer(infohash)` for DHT announcements
- Implements `get_peers(infohash)` for provider discovery with timeout
- Waits for DHT bootstrap in background (non-blocking)

**Without `global-discovery` feature:**
- Stub implementation that logs operations but doesn't perform real DHT queries
- Allows the codebase to compile without the mainline dependency

### 2. Real DHT Operations

**announce_peer:**
- Uses mainline crate's `announce_peer` with implicit port (None)
- The DHT nodes detect the source port automatically
- Rate-limited to prevent spam (5-minute minimum between re-announces)

**get_peers:**
- Returns a stream of peer socket addresses
- Collects results with a 30-second timeout
- Limits results to MAX_PROVIDERS (50)

### 3. Auto-Announce Local Blobs

When `auto_announce = true` in `ContentDiscoveryConfig`:

1. After node bootstrap, waits 5 seconds for DHT to initialize
2. Scans local blob store (up to 10,000 blobs)
3. Bulk announces all blobs to DHT via `announce_local_blobs()`
4. Runs in background - doesn't block node startup

### 4. Updated AnnounceTracker

Changed internal tracking to store `(Instant, u64, BlobFormat)` tuples:
- Enables proper republishing with size and format metadata
- `get_stale_announces()` now returns full blob info for republishing

## Configuration

Enable with:
```bash
cargo build --features global-discovery
```

Environment variables:
- `ASPEN_CONTENT_DISCOVERY_ENABLED=true` - Enable content discovery
- `ASPEN_CONTENT_DISCOVERY_AUTO_ANNOUNCE=true` - Auto-announce local blobs
- `ASPEN_CONTENT_DISCOVERY_SERVER_MODE=false` - DHT client vs server mode
- `ASPEN_CONTENT_DISCOVERY_DHT_PORT=0` - UDP port (0 = random)

## Design Decisions

### Using announce_peer vs put_mutable

We use `announce_peer` (BEP-0005) rather than `put_mutable` (BEP-0044) because:
1. `announce_peer` is simpler - just IP:port pairs
2. Lower overhead than storing signed mutable records
3. Iroh handles the actual blob verification via QUIC connection
4. Provider verification is implicit when downloading

### Hash Mapping

BLAKE3 (Iroh blob hash) -> 20-byte DHT infohash:
- SHA-256(hash || format_byte) truncated to 20 bytes
- Deterministic and format-sensitive (Raw vs HashSeq)

### Background Bootstrap

DHT bootstrap runs in a spawned task:
- Doesn't block node startup
- Operations that occur before bootstrap may fail silently
- 5-second delay before auto-announce gives time for bootstrap

## Testing

All 8 content discovery tests pass:
- `test_dht_infohash_deterministic`
- `test_dht_infohash_format_differs`
- `test_dht_announce_roundtrip`
- `test_signed_announce_verify`
- `test_announce_tracker_rate_limiting`
- `test_announce_tracker_capacity`
- `test_announce_tracker_stale_announces`
- `test_content_discovery_service_lifecycle`

Full test suite: 1805 tests passed

## Files Modified

- `src/cluster/content_discovery.rs` - Main DHT implementation
- `src/cluster/bootstrap.rs` - Auto-announce integration

## Security Considerations

1. **No authentication from DHT peers** - DHT returns IP:port only; actual verification happens when connecting via Iroh QUIC
2. **Rate limiting** - 5-minute minimum between announces per hash
3. **Republishing** - 30-minute background republish to maintain presence

## Future Improvements

1. Store bootstrap nodes in persistent cache for faster reconnection
2. Consider using BEP-0044 mutable items for richer provider metadata
3. Add metrics for DHT operations (announce success rate, query latency)
