## ADDED Requirements

### Requirement: Forge hook event types

The hooks system SHALL define event types for Forge operations: `RefUpdated`, `PatchCreated`, `PatchMerged`, `PatchClosed`, `PatchApproved`, and `RepoCreated`. Each event SHALL map to a pubsub topic under `hooks.forge.*`.

#### Scenario: RefUpdated event published

- **WHEN** a ref is updated on a Forge repository
- **THEN** a `RefUpdated` hook event SHALL be published to topic `hooks.forge.ref_updated`
- **AND** the event payload SHALL contain the repo ID, ref name, new hash, and old hash

#### Scenario: PatchMerged event published

- **WHEN** a patch is merged via a `Merge` COB operation
- **THEN** a `PatchMerged` hook event SHALL be published to topic `hooks.forge.patch_merged`
- **AND** the event payload SHALL contain the repo ID, patch COB ID, and merged commit hash

#### Scenario: PatchApproved event published

- **WHEN** a reviewer approves a patch via an `Approve` COB operation
- **THEN** a `PatchApproved` hook event SHALL be published to topic `hooks.forge.patch_approved`
- **AND** the event payload SHALL contain the repo ID, patch COB ID, approver key, and approved commit hash

### Requirement: CI hook event types

The hooks system SHALL define event types for CI pipeline lifecycle: `PipelineStarted`, `PipelineCompleted`, `PipelineFailed`, and `DeployCompleted`. Each event SHALL map to a pubsub topic under `hooks.ci.*`.

#### Scenario: PipelineStarted event published

- **WHEN** the pipeline orchestrator creates a new run
- **THEN** a `PipelineStarted` hook event SHALL be published to topic `hooks.ci.pipeline_started`
- **AND** the event payload SHALL contain the repo ID, commit hash, ref name, run ID, and pipeline name

#### Scenario: PipelineCompleted event published

- **WHEN** a pipeline reaches `Success` status
- **THEN** a `PipelineCompleted` hook event SHALL be published to topic `hooks.ci.pipeline_completed`
- **AND** the event payload SHALL contain the repo ID, commit hash, run ID, and duration

#### Scenario: PipelineFailed event published

- **WHEN** a pipeline reaches `Failed` status
- **THEN** a `PipelineFailed` hook event SHALL be published to topic `hooks.ci.pipeline_failed`
- **AND** the event payload SHALL contain the repo ID, commit hash, run ID, and error message if available

### Requirement: Hook handlers can register for CI events

Users SHALL be able to register in-process, job-based, or shell handlers for forge and CI hook events using the existing `HookService::register_handler` API.

#### Scenario: Shell handler on pipeline completion

- **GIVEN** a shell handler is registered for `PipelineCompleted` events with command `/usr/local/bin/notify.sh`
- **WHEN** a pipeline completes successfully
- **THEN** the shell handler SHALL be invoked with `ASPEN_HOOK_EVENT` set to the JSON-serialized event
