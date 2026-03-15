## ADDED Requirements

### Requirement: Stale cache serving on cluster failure

When the cluster is unreachable and a cached entry exists (even with expired TTL), AspenFs SHALL serve the stale cached data instead of returning an I/O error. A warning SHALL be logged on first degraded-mode access.

#### Scenario: Stale read on connection failure

- **WHEN** a process reads a file
- **AND** the data cache has an expired entry for the file
- **AND** the cluster is unreachable (RPC timeout or connection refused)
- **THEN** the stale cached data SHALL be returned
- **AND** a warning SHALL be logged: "serving stale cached data for {path} (cluster unreachable)"

#### Scenario: No cache entry and cluster unreachable

- **WHEN** a process reads a file for the first time
- **AND** the data cache has no entry for the file
- **AND** the cluster is unreachable
- **THEN** the read SHALL return EIO (I/O error)

#### Scenario: Writes fail in offline mode

- **WHEN** a process attempts to write a file
- **AND** the cluster is unreachable
- **THEN** the write SHALL return EIO
- **AND** no data SHALL be buffered for later sync

### Requirement: Degraded mode indicator

AspenFs SHALL expose a `is_degraded: bool` flag that indicates the cluster connection is unhealthy. This flag SHALL be set when an RPC fails due to connection issues and cleared when the next RPC succeeds.

#### Scenario: Degraded flag set on failure

- **WHEN** an RPC to the cluster fails with a connection error
- **THEN** `is_degraded` SHALL be set to true

#### Scenario: Degraded flag cleared on recovery

- **WHEN** `is_degraded` is true
- **AND** a subsequent RPC to the cluster succeeds
- **THEN** `is_degraded` SHALL be set to false

### Requirement: Reconnection revalidation

When the cluster becomes reachable again after a degraded period, AspenFs SHALL revalidate all entries served from stale cache by checking their content hashes. Entries with changed hashes SHALL be invalidated.

#### Scenario: Stale entries revalidated on reconnection

- **WHEN** the cluster becomes reachable after a degraded period
- **AND** 10 files were served from stale cache during the outage
- **THEN** hash-check RPCs SHALL be issued for all 10 files
- **AND** entries with unchanged hashes SHALL have their TTLs reset
- **AND** entries with changed hashes SHALL be invalidated

### Requirement: Offline mode resource bounds

The stale cache SHALL NOT grow beyond the normal cache size limits during offline mode. Stale entries SHALL still be subject to LRU eviction if the cache is full.

#### Scenario: Cache limits enforced in offline mode

- **WHEN** the cache is at capacity (64MB)
- **AND** a new stale read would exceed the limit
- **THEN** the least-recently-used entry SHALL be evicted
- **AND** the new stale entry SHALL be cached
