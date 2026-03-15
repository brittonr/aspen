## ADDED Requirements

### Requirement: Cache lookup before execution

When cached execution is enabled, the system SHALL compute the cache key from the command, arguments, environment subset, and input file hashes, then check the cache before launching the process. On cache hit, the cached result SHALL be returned without executing the process.

#### Scenario: Cache hit skips execution

- **WHEN** a CI job runs `rustc src/lib.rs -o target/lib.o`
- **AND** a cache entry exists for the same command with the same input file contents
- **AND** the entry has not expired
- **THEN** the process SHALL NOT be launched
- **AND** the cached exit code, stdout, and stderr SHALL be returned
- **AND** cached output files SHALL be materialized from iroh-blobs

#### Scenario: Cache miss runs normally

- **WHEN** a CI job runs `rustc src/lib.rs -o target/lib.o`
- **AND** no cache entry exists for the computed key
- **THEN** the process SHALL be launched with read tracking enabled
- **AND** on completion, the result SHALL be stored in the cache

#### Scenario: Cache lookup timeout falls through to execution

- **WHEN** the cache lookup takes longer than the lookup timeout (default 100ms)
- **THEN** the process SHALL be launched normally with tracking enabled
- **AND** the result SHALL be cached on completion

### Requirement: Output materialization from cache

On cache hit, cached output files SHALL be materialized into the filesystem from iroh-blobs. stdout and stderr SHALL be replayed from cached blobs.

#### Scenario: Output files restored from blobs

- **WHEN** a cache hit occurs for a compilation step
- **AND** the cached entry lists output file `target/lib.o` with blob hash H
- **THEN** the file `target/lib.o` SHALL be written to the filesystem with the content from blob H
- **AND** the file SHALL have the same size as the original

#### Scenario: stdout and stderr replayed

- **WHEN** a cache hit occurs
- **AND** the cached entry has stdout hash H1 and stderr hash H2
- **THEN** stdout content from blob H1 SHALL be returned to the caller
- **AND** stderr content from blob H2 SHALL be returned to the caller

### Requirement: Output capture on cache miss

On cache miss, the system SHALL capture the process's stdout, stderr, exit code, and all files written to the tracked filesystem. These SHALL be stored as iroh-blobs and indexed in the cache.

#### Scenario: Successful build outputs captured

- **WHEN** a process runs with tracking enabled
- **AND** the process writes files [target/main.o, target/main] to the filesystem
- **AND** the process exits with code 0
- **THEN** both output files SHALL be stored as iroh-blobs
- **AND** stdout and stderr SHALL be stored as iroh-blobs
- **AND** a cache entry SHALL be created mapping the cache key to all output references

#### Scenario: Failed build outputs captured

- **WHEN** a process runs with tracking enabled
- **AND** the process exits with code 1
- **THEN** stderr SHALL be stored as an iroh-blob
- **AND** a cache entry SHALL be created with exit code 1
- **AND** future runs with the same inputs SHALL return the cached failure

### Requirement: Opt-in activation

Cached execution SHALL be opt-in, activated per CI job via job spec flag or environment variable `ASPEN_CACHED_EXEC=1`. It SHALL NOT be enabled by default on any FUSE mount.

#### Scenario: Enabled via job spec

- **WHEN** a CI job spec includes `cached_execution: true`
- **THEN** the job's FUSE mount SHALL have read tracking enabled
- **AND** process executions within the job SHALL use cache lookup

#### Scenario: Enabled via environment variable

- **WHEN** the environment variable `ASPEN_CACHED_EXEC=1` is set in the job environment
- **THEN** cached execution SHALL be enabled for that job

#### Scenario: Disabled by default

- **WHEN** no flag or environment variable enables cached execution
- **THEN** no read tracking SHALL occur
- **AND** no cache lookups SHALL be performed

### Requirement: Recursive process tree caching

When a tracked process spawns child processes, each child SHALL be cached independently based on its own read set. The parent's cache entry SHALL reference child cache keys as dependencies.

#### Scenario: cargo build with multiple rustc invocations

- **WHEN** `cargo build` spawns `rustc src/lib.rs` and `rustc src/main.rs`
- **AND** `src/lib.rs` has not changed since the last build
- **THEN** `rustc src/lib.rs` SHALL hit the cache and skip execution
- **AND** `rustc src/main.rs` SHALL miss the cache and execute
- **AND** the `cargo build` parent entry SHALL reference both child cache keys

#### Scenario: All children cached means parent cached

- **WHEN** a `cargo build` cache entry exists
- **AND** all child process cache entries are still valid
- **AND** the parent's own inputs have not changed
- **THEN** the entire `cargo build` SHALL hit the cache
- **AND** all outputs SHALL be materialized without running any process

### Requirement: Environment variable handling

The cache key SHALL include a hash of a curated environment variable subset. The default set SHALL include `PATH`, `HOME`, `CC`, `CXX`, `CFLAGS`, `CXXFLAGS`, `RUSTFLAGS`, `NIX_STORE`, and `ASPEN_EXEC_CACHE_ENV` (which specifies additional vars to include). Variables not in the tracked set SHALL be ignored for cache key purposes.

#### Scenario: PATH change invalidates cache

- **WHEN** a process ran with PATH=/usr/bin:/bin and produced cache entry K1
- **AND** the same process runs again with PATH=/opt/bin:/usr/bin:/bin
- **THEN** the cache key SHALL differ from K1
- **AND** the process SHALL miss the cache

#### Scenario: Untracked variable change does not invalidate

- **WHEN** a process ran with TERM=xterm and produced cache entry K1
- **AND** the same process runs again with TERM=xterm-256color
- **THEN** the cache key SHALL be the same as K1
- **AND** the process SHALL hit the cache
