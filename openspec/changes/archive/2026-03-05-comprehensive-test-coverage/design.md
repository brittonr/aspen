# Design: Comprehensive Test Coverage

## Approach

Add tests in 5 tiers, from fastest/simplest to slowest/most complex:

### Tier 1: Unit Tests (pure functions, no I/O)

**aspen-crypto** — Add edge case tests to existing test modules:

- `cookie.rs`: empty cookie, max-length cookie, HMAC key determinism, gossip topic determinism, non-ASCII chars
- `identity.rs`: round-trip hex encoding, invalid hex lengths, generate uniqueness

**aspen-fuse cache.rs** — Test the in-memory cache (DashMap-based):

- `get_data`/`put_data` basic round-trip
- `invalidate_data` removes entry
- `get_meta`/`put_meta` round-trip with `CachedMeta`
- `invalidate_prefix` clears matching scans
- `invalidate_all_scans` clears all scan entries but preserves data/meta

**aspen-fuse metadata.rs** — Test `FileMetadata` serialization:

- `to_bytes`/`from_bytes` round-trip
- `now()` produces valid timestamps
- `touch()` updates mtime+ctime
- `from_bytes` handles corrupt/short input

### Tier 2: Unit Tests (with mocked backends)

**aspen-secrets SOPS operations** — Each operation takes a config struct and calls backend methods. Test with `InMemorySecretsBackend`:

- `encrypt_file`: verify output has SOPS metadata, encrypted values, correct MAC
- `decrypt_file`: verify round-trip with encrypt, handles missing keys
- `rotate_file`: verify data key changes but plaintext unchanged
- `edit_file`: verify in-place modification
- `update_keys`: verify recipient list changes
- `encrypt_data_key_for_age`: verify age recipient encryption

### Tier 3: Integration Tests (real Raft node, no network)

**Pub/Sub EventStream** — Address TODO in `pubsub_integration_test.rs`:

- Test `EventStream` with `StreamExt` (currently unused import)
- Test scan verification with proper encoded key format
- Test error cases: subscribe to invalid topic pattern

### Tier 4: Integration Tests (multi-node)

**Snapshot under failure** — Address TODO in `router_snapshot_t10_build_snapshot.rs`:

- Snapshot during concurrent writes
- Snapshot with log compaction race
- Recovery from partial snapshot

### Tier 5: NixOS VM Tests (real cluster, real OS)

**ephemeral-pubsub.nix** — 3-node cluster:

1. Node1 publishes to topic "test/events"
2. Node2 subscribes via CLI, receives events
3. Verify wildcard subscriptions work across nodes
4. Verify events are ephemeral (not in KV after restart)

**fuse-operations.nix** — Single-node with FUSE mount:

1. Initialize cluster, mount aspen-fuse at /mnt/aspen
2. Write files via FUSE, verify via KV scan
3. Read files via FUSE, verify content matches
4. Delete files via FUSE, verify removal
5. Test directory listing, mkdir, nested paths

## Test Infrastructure

- All unit tests use existing test helpers (`SingleNodeTester`, `InMemorySecretsBackend`, etc.)
- VM tests follow established patterns from 32 existing VM tests
- Integration tests use `RealClusterTester` for multi-node scenarios
- No new test infrastructure needed
