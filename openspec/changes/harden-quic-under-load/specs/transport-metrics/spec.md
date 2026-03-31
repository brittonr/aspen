## ADDED Requirements

### Requirement: Per-priority stream open counters

The connection pool SHALL track total streams opened at each priority level using atomic counters.

#### Scenario: Counter increments on stream open

- **WHEN** a stream is opened with `STREAM_PRIORITY_RAFT`
- **THEN** the `raft_streams_opened` counter increments by 1

#### Scenario: Bulk stream counter

- **WHEN** a stream is opened with `STREAM_PRIORITY_BULK`
- **THEN** the `bulk_streams_opened` counter increments by 1

### Requirement: ReadIndex retry counters

The KV store SHALL count ReadIndex retry attempts and successes-after-retry using atomic counters.

#### Scenario: Retry counted

- **WHEN** a ReadIndex fails and is retried
- **THEN** the `read_index_retry_count` counter increments by 1

#### Scenario: Success after retry counted

- **WHEN** a ReadIndex succeeds after one or more retries
- **THEN** the `read_index_retry_success_count` counter increments by 1

### Requirement: Metrics accessible via ConnectionPoolMetrics

All new counters SHALL be exposed through the existing `ConnectionPoolMetrics` struct so the TUI and logging can display them.

#### Scenario: Metrics snapshot includes priority counters

- **WHEN** `ConnectionPoolMetrics` is read
- **THEN** it includes `raft_streams_opened`, `bulk_streams_opened`, `read_index_retry_count`, and `read_index_retry_success_count`
