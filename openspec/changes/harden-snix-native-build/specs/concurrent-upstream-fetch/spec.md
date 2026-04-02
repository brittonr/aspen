## ADDED Requirements

### Requirement: Concurrent NAR fetching in populate_closure

The `populate_closure` method SHALL fetch NARs from upstream binary caches concurrently, bounded by `MAX_CONCURRENT_FETCHES`. The BFS traversal of references SHALL continue enqueuing new paths while fetches are in flight.

#### Scenario: Multiple paths need fetching

- **WHEN** the BFS queue contains more paths than `MAX_CONCURRENT_FETCHES`
- **THEN** the system fetches up to `MAX_CONCURRENT_FETCHES` NARs in parallel, processes results as they complete, and enqueues newly-discovered references

#### Scenario: Single path needs fetching

- **WHEN** only one path needs fetching
- **THEN** the system fetches it without blocking on the semaphore beyond acquisition

#### Scenario: Fetch failure does not block other fetches

- **WHEN** one NAR fetch fails (network error, decompress error, hash mismatch)
- **THEN** the error is recorded in the report and other concurrent fetches continue unaffected

### Requirement: Transient error retry for NAR fetches

NAR downloads that fail with transient HTTP errors (5xx, timeout, connection reset) SHALL be retried up to 2 times with exponential backoff before recording a failure.

#### Scenario: First attempt fails with 503, retry succeeds

- **WHEN** a NAR download returns HTTP 503 on the first attempt
- **THEN** the system retries after a backoff delay and succeeds on the second attempt

#### Scenario: All retry attempts exhausted

- **WHEN** a NAR download fails on all 3 attempts (initial + 2 retries)
- **THEN** the path is recorded in `PopulateReport.errors` with the final error message

#### Scenario: Non-transient errors are not retried

- **WHEN** a NAR download returns HTTP 404 or a hash mismatch
- **THEN** the system does not retry and immediately records the error
