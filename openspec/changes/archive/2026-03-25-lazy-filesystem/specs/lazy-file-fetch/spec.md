## ADDED Requirements

### Requirement: No eager content fetch on open

The FUSE `open()` handler SHALL NOT prefetch file content. Content SHALL be fetched on the first `read()` call for the file. The `open()` handler SHALL only verify the file exists and check permissions.

#### Scenario: Open without read does not fetch content

- **WHEN** a process opens a file with `open(O_RDONLY)`
- **AND** the process closes the file without calling `read()`
- **THEN** no file content SHALL have been fetched from the cluster
- **AND** only inode lookup and permission check RPCs SHALL have occurred

#### Scenario: First read triggers content fetch

- **WHEN** a process opens a file and issues `read(offset=0, size=4096)`
- **AND** the file content is not in the data cache
- **THEN** the read SHALL trigger a fetch of the requested byte range from the cluster
- **AND** the fetched data SHALL be placed in the data cache

#### Scenario: Cached data serves without fetch

- **WHEN** a process reads a file that was recently read by another process
- **AND** the data cache entry has not expired
- **THEN** the read SHALL be served from cache
- **AND** no cluster RPC SHALL occur

### Requirement: Sequential readahead preserved

The sequential access detection and readahead prefetch SHALL continue to operate. After detecting 3 sequential reads on the same file, the prefetcher SHALL fetch 512KB ahead of the current read position.

#### Scenario: Sequential readahead triggers after pattern detection

- **WHEN** a process reads a file sequentially (offset 0, 4096, 8192, 12288)
- **THEN** the 4th read SHALL trigger a readahead prefetch of 512KB beyond the current position
- **AND** subsequent sequential reads SHALL serve from the prefetched cache

#### Scenario: Random access does not trigger readahead

- **WHEN** a process reads a file at random offsets (0, 50000, 1000, 80000)
- **THEN** no readahead prefetch SHALL be triggered

### Requirement: Access pattern statistics

AspenFs SHALL maintain per-mount counters for: files opened, files where at least one `read()` occurred, total bytes fetched from cluster, total bytes served from cache, and bytes that would have been prefetched under the eager strategy (for comparison).

#### Scenario: Statistics reflect lazy behavior

- **WHEN** 100 files are opened
- **AND** 15 files are actually read
- **THEN** `files_opened` SHALL be 100
- **AND** `files_read` SHALL be 15
- **AND** `prefetch_savings_bytes` SHALL reflect 85 × 128KB = 10.8MB of avoided prefetch
