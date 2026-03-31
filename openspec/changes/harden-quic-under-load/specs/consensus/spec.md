## MODIFIED Requirements

### Requirement: ReadIndex retry is adaptive

The ReadIndex consistency check SHALL use Raft metrics to decide whether a retry is warranted. Retries SHALL only occur when the current node believes it is still the leader and the log is reasonably current. The maximum retry count SHALL be configurable via `READ_INDEX_MAX_RETRIES`.

#### Scenario: Retry on transient leader contention

- **WHEN** ReadIndex fails with a NotLeader error
- **AND** the Raft metrics indicate `current_leader == self`
- **AND** the gap between `last_log_id` and `committed` is less than 100 entries
- **THEN** the system retries after a jittered backoff of 50-100ms

#### Scenario: No retry on real leadership change

- **WHEN** ReadIndex fails with a NotLeader error
- **AND** the Raft metrics indicate `current_leader != self` or `current_leader == None`
- **THEN** the system does NOT retry and returns the error immediately

#### Scenario: Retry budget is bounded

- **WHEN** ReadIndex fails repeatedly
- **THEN** the system retries at most `READ_INDEX_MAX_RETRIES` times
- **AND** returns the last error after exhausting retries

#### Scenario: Jittered backoff prevents thundering herd

- **WHEN** multiple concurrent reads fail ReadIndex simultaneously
- **THEN** each retry uses a random backoff between `READ_INDEX_RETRY_BASE_MS` and `2 × READ_INDEX_RETRY_BASE_MS`
- **AND** the backoff values are not identical across concurrent retries
