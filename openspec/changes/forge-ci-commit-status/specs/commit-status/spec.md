## ADDED Requirements

### Requirement: Commit status storage

The Forge SHALL store commit statuses in the KV store at `forge:status:{repo_hex}:{commit_hex}:{context}`. Each entry SHALL contain a `CommitCheckState` (Pending, Success, Failure, Error), a description string, a pipeline run ID, and a creation timestamp.

#### Scenario: Write a commit status

- **WHEN** CI reports status for repo `R`, commit `C`, context `"ci/pipeline"` with state `Success`
- **THEN** the Forge SHALL write a JSON entry at `forge:status:{R_hex}:{C_hex}:ci/pipeline`
- **AND** the entry SHALL contain state `Success`, the pipeline run ID, a description, and `created_at_ms`

#### Scenario: Overwrite existing status

- **WHEN** CI reports state `Success` for a commit that already has state `Pending` under the same context
- **THEN** the Forge SHALL overwrite the existing entry with the new state
- **AND** the previous state SHALL not be preserved (last-write-wins)

#### Scenario: Multiple contexts per commit

- **WHEN** CI reports status for context `"ci/pipeline"` and a separate system reports for context `"ci/deploy"`
- **THEN** the Forge SHALL store both entries independently
- **AND** querying all statuses for that commit SHALL return both entries

### Requirement: Query commit statuses

The Forge SHALL support querying all commit statuses for a given repo and commit hash via KV scan over the `forge:status:{repo_hex}:{commit_hex}:` prefix.

#### Scenario: List statuses for a commit

- **WHEN** a client queries statuses for repo `R`, commit `C`
- **AND** two statuses exist: `ci/pipeline: Success` and `ci/deploy: Pending`
- **THEN** the query SHALL return both entries

#### Scenario: No statuses for a commit

- **WHEN** a client queries statuses for a commit with no CI activity
- **THEN** the query SHALL return an empty list

### Requirement: Status reporter trait

The CI system SHALL define a `StatusReporter` trait with a single async method `report_status` that accepts a `CommitStatusReport` containing repo ID, commit hash, context string, check state, description, and pipeline run ID.

#### Scenario: Reporter called on pipeline creation

- **WHEN** the pipeline orchestrator creates a new run via `track_run`
- **THEN** it SHALL call `report_status` with state `Pending`

#### Scenario: Reporter called on pipeline completion

- **WHEN** the pipeline orchestrator detects a terminal state (Success, Failed, Cancelled) in `sync_run_status`
- **THEN** it SHALL call `report_status` with the corresponding `CommitCheckState`

#### Scenario: Reporter failure is non-fatal

- **WHEN** `report_status` returns an error
- **THEN** the pipeline orchestrator SHALL log the error at warn level
- **AND** the pipeline run SHALL NOT be affected

### Requirement: Forge status reporter implementation

The CI system SHALL provide a `ForgeStatusReporter` struct that implements `StatusReporter` by writing `CommitStatus` entries to the Forge KV namespace and optionally publishing a `PipelineStatus` gossip announcement.

#### Scenario: Write to Forge KV

- **WHEN** `ForgeStatusReporter::report_status` is called with state `Success` for repo `R`, commit `C`, context `"ci/pipeline"`
- **THEN** it SHALL write a JSON `CommitStatus` to key `forge:status:{R_hex}:{C_hex}:ci/pipeline`

#### Scenario: Publish gossip announcement

- **WHEN** `ForgeStatusReporter` is configured with a gossip service
- **AND** `report_status` is called
- **THEN** it SHALL broadcast a `PipelineStatus` announcement to the repo's gossip topic

### Requirement: Pipeline status gossip announcement

The Forge gossip system SHALL support a `PipelineStatus` announcement variant containing repo ID, commit hash, ref name, run ID, check state, and context string.

#### Scenario: Announce pipeline pending

- **WHEN** a pipeline is created for repo `R`, commit `C`, ref `refs/heads/main`
- **THEN** a `PipelineStatus` announcement SHALL be broadcast with state `Pending`

#### Scenario: Announce pipeline success

- **WHEN** a pipeline completes successfully
- **THEN** a `PipelineStatus` announcement SHALL be broadcast with state `Success`

#### Scenario: Existing handlers ignore new variant

- **WHEN** a node running older code receives a `PipelineStatus` announcement
- **THEN** it SHALL ignore the unknown variant without error
