## ADDED Requirements

### Requirement: Cache entries store content hash

Data cache entries for files backed by iroh-blobs SHALL include the BLAKE3 content hash alongside the cached data and TTL timestamp.

#### Scenario: Cache entry with hash

- **WHEN** file data is fetched from the cluster
- **AND** the file is backed by an iroh-blob with known BLAKE3 hash
- **THEN** the cache entry SHALL store the data, TTL timestamp, and BLAKE3 hash

#### Scenario: Cache entry without hash

- **WHEN** file data is fetched from the cluster
- **AND** the file has no known iroh-blob hash (e.g., small inline KV value)
- **THEN** the cache entry SHALL store the data and TTL timestamp
- **AND** the hash field SHALL be None
- **AND** TTL-based expiry SHALL apply as before

### Requirement: Hash-check revalidation on stale cache

When a cache entry with a known hash expires (TTL elapsed), the FUSE layer SHALL send a lightweight hash-check RPC to the cluster instead of re-fetching the full content. If the hash is unchanged, the cache entry SHALL be refreshed without re-fetching data.

#### Scenario: Hash unchanged extends cache

- **WHEN** a cached file's TTL has expired
- **AND** the cache entry has BLAKE3 hash H
- **AND** the cluster confirms the file's current hash is still H
- **THEN** the cache entry's TTL SHALL be reset
- **AND** no file content SHALL be re-fetched

#### Scenario: Hash changed triggers re-fetch

- **WHEN** a cached file's TTL has expired
- **AND** the cache entry has BLAKE3 hash H1
- **AND** the cluster reports the file's current hash is H2 (H2 ≠ H1)
- **THEN** the stale cache entry SHALL be invalidated
- **AND** the file content SHALL be re-fetched from the cluster
- **AND** the new cache entry SHALL store H2

#### Scenario: Hash-check timeout falls through to full fetch

- **WHEN** the hash-check RPC takes longer than 100ms
- **THEN** the stale cache entry SHALL be invalidated
- **AND** a full content fetch SHALL be issued

### Requirement: Hash-check RPC

The client protocol SHALL support a `HashCheck` request that takes a key and expected BLAKE3 hash, and returns either `Unchanged` or `Changed { new_hash }`. This request SHALL NOT transfer file content.

#### Scenario: Hash-check request size

- **WHEN** a `HashCheck` request is sent
- **THEN** the request payload SHALL be under 200 bytes (key + 32-byte hash + overhead)
- **AND** the response payload SHALL be under 100 bytes

### Requirement: Extended TTL for hash-validated entries

Cache entries revalidated by hash-check SHALL receive a longer TTL (default 60s) than freshly fetched entries (default 5s), since hash validation provides stronger freshness guarantees than time-based expiry.

#### Scenario: Revalidated entry has longer TTL

- **WHEN** a cache entry is revalidated via hash-check (hash unchanged)
- **THEN** the new TTL SHALL be 60 seconds (configurable)
- **AND** subsequent reads within 60s SHALL serve from cache without any RPC
