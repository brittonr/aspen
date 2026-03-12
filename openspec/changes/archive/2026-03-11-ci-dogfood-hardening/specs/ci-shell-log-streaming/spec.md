## ADDED Requirements

### Requirement: Shell executor streams log chunks to KV during execution

The `LocalExecutorWorker` SHALL write CI log chunks to the KV store during job execution, using the same format and key scheme as `NixBuildWorker`. This enables `ci logs --follow` for shell jobs.

#### Scenario: Shell job produces streaming logs

- **WHEN** a `shell_command` job executes via `LocalExecutorWorker`
- **AND** the worker has a KV store reference and the job payload contains a `run_id`
- **THEN** the worker SHALL write log chunks to `_ci:logs:{run_id}:{job_id}:{chunk_index:010}` keys
- **AND** each chunk SHALL be a JSON-serialized `CiLogChunk` with `index`, `content`, and `timestamp_ms`
- **AND** chunks SHALL be flushed when the buffer exceeds 8KB or every 500ms (whichever comes first)

#### Scenario: Shell job completion writes marker

- **WHEN** a `shell_command` job finishes (success or failure)
- **THEN** the worker SHALL write a `CiLogCompleteMarker` to `_ci:logs:{run_id}:{job_id}:__complete__`
- **AND** the marker SHALL contain `total_chunks`, `timestamp_ms`, and `status`

#### Scenario: Shell job without KV store skips log streaming

- **WHEN** a `shell_command` job executes but the worker has no KV store reference
- **THEN** the worker SHALL execute the job normally without writing log chunks
- **AND** stdout/stderr SHALL still be captured in the `JobOutput` metadata

### Requirement: Log bridge is shared between executors

The `log_bridge` function (channel reader → KV chunk writer with periodic flush) SHALL be extracted to a shared location so both `NixBuildWorker` and `LocalExecutorWorker` use identical logic.

#### Scenario: Shared log bridge produces identical chunk format

- **WHEN** `NixBuildWorker` writes a log chunk for a nix job
- **AND** `LocalExecutorWorker` writes a log chunk for a shell job
- **THEN** both chunks SHALL use the same JSON schema (`CiLogChunk`)
- **AND** both SHALL use the same KV key format (`_ci:logs:{run_id}:{job_id}:{chunk_index:010}`)
- **AND** both SHALL use the same flush threshold (8KB) and periodic interval (500ms)
