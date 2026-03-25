## ADDED Requirements

### Requirement: Cache entry storage format

The execution cache SHALL store entries in Raft KV with key format `_exec_cache:{blake3_hex}` where the hash is computed over the canonical cache key (command, args, env hash, sorted input file hashes). The value SHALL be JSON containing exit code, stdout blob hash, stderr blob hash, output file mappings, creation timestamp, and TTL.

#### Scenario: Store a cache entry after successful execution

- **WHEN** a process completes with exit code 0
- **AND** the process read files with BLAKE3 hashes [h1, h2, h3]
- **AND** the command was "rustc src/main.rs -o target/main"
- **THEN** the cache key SHALL be `BLAKE3(command || args || env_hash || sort([h1, h2, h3]))`
- **AND** a KV entry SHALL be written at `_exec_cache:{key_hex}`
- **AND** the value SHALL contain the exit code, stdout hash, stderr hash, and output file hashes

#### Scenario: Store a cache entry after failed execution

- **WHEN** a process completes with a non-zero exit code
- **AND** cached execution is enabled
- **THEN** the cache entry SHALL still be stored with the non-zero exit code
- **AND** stderr SHALL be captured and stored as a blob

### Requirement: Cache key determinism

The cache key computation SHALL be deterministic: the same command, arguments, environment subset, and input file contents SHALL always produce the same cache key, regardless of file access order, timestamps, or the node that computes it.

#### Scenario: Same inputs produce same key regardless of access order

- **WHEN** process A reads files [x.rs, y.rs, z.rs] in that order
- **AND** process B reads files [z.rs, x.rs, y.rs] in that order
- **AND** both run the same command with the same args and env
- **THEN** both processes SHALL produce the same cache key

#### Scenario: Timestamp changes do not affect cache key

- **WHEN** a file's mtime changes but its content is unchanged
- **THEN** the cache key SHALL remain the same

### Requirement: Cache lookup by key

The cache SHALL support point lookups by cache key, returning the cached result or a miss indication. Lookups SHALL be bounded by a timeout.

#### Scenario: Cache hit returns stored result

- **WHEN** a cache entry exists for key K
- **AND** the entry has not expired
- **THEN** lookup(K) SHALL return the cached exit code, stdout hash, stderr hash, and output mappings

#### Scenario: Cache miss returns nothing

- **WHEN** no cache entry exists for key K
- **THEN** lookup(K) SHALL return a cache miss indication within the lookup timeout

#### Scenario: Expired entry treated as miss

- **WHEN** a cache entry exists for key K
- **AND** the entry's TTL has elapsed
- **THEN** lookup(K) SHALL return a cache miss indication

### Requirement: TTL-based expiration

Each cache entry SHALL have a configurable TTL (default 24 hours). Expired entries SHALL be treated as cache misses. A background eviction process SHALL remove expired entries periodically.

#### Scenario: Entry expires after TTL

- **WHEN** a cache entry is created at time T with TTL of 24 hours
- **AND** the current time is T + 24h + 1ms
- **THEN** the entry SHALL be treated as a cache miss

#### Scenario: Entry is valid within TTL

- **WHEN** a cache entry is created at time T with TTL of 24 hours
- **AND** the current time is T + 12h
- **THEN** the entry SHALL be treated as a cache hit

### Requirement: LRU eviction under storage pressure

When total cache storage exceeds a configurable bound (default 10 GB per node), the cache SHALL evict least-recently-used entries until storage drops below the bound. Eviction SHALL remove both the KV index entry and unreference the output blobs.

#### Scenario: Eviction triggers at storage limit

- **WHEN** cache storage reaches the configured maximum
- **AND** a new entry is added
- **THEN** the least-recently-used entries SHALL be evicted until storage is below the maximum
- **AND** evicted entries' output blobs SHALL be unreferenced from iroh-blobs

### Requirement: Cache entry resource bounds

Cache entries SHALL enforce resource bounds: maximum output blob size (1 GB), maximum number of output files per entry (10,000), maximum number of input files per entry (100,000).

#### Scenario: Reject oversized output

- **WHEN** a process produces an output file larger than 1 GB
- **THEN** the execution result SHALL NOT be cached
- **AND** the process result SHALL still be returned to the caller
