## ADDED Requirements

### Requirement: Forge push triggers CI pipeline

The system SHALL automatically trigger a CI pipeline when a git push updates a ref on a watched repository. The trigger path SHALL use Forge gossip `RefUpdate` announcements received by `CiTriggerHandler`.

#### Scenario: Push to watched repo triggers pipeline

- **WHEN** a user pushes to `refs/heads/main` on a watched repository that contains `.aspen/ci.ncl`
- **THEN** the `TriggerService` SHALL fetch `.aspen/ci.ncl` from the pushed commit via `ForgeConfigFetcher`
- **AND** parse the Nickel config into a `PipelineConfig`
- **AND** start a pipeline run via `PipelineOrchestrator`

#### Scenario: Push to unwatched repo does not trigger

- **WHEN** a user pushes to a repository that is not being watched
- **THEN** no CI pipeline SHALL be triggered

#### Scenario: Push to non-matching ref does not trigger

- **WHEN** a user pushes to `refs/heads/feature-branch` on a watched repo
- **AND** the pipeline config only triggers on `refs/heads/main`
- **THEN** no pipeline SHALL be started

#### Scenario: Missing CI config skips gracefully

- **WHEN** a user pushes to a watched repo that does not contain `.aspen/ci.ncl`
- **THEN** no pipeline SHALL be triggered
- **AND** no error SHALL be logged (only debug-level)

### Requirement: ConfigFetcher reads from Forge git objects

The system SHALL implement `ConfigFetcher` by reading git objects stored in the Forge. The fetcher SHALL walk the commit tree to find the config file without checking out the repository.

#### Scenario: Read config at specific commit

- **WHEN** `ForgeConfigFetcher::fetch_config(repo_id, commit_hash, ".aspen/ci.ncl")` is called
- **THEN** it SHALL resolve the commit's tree, walk to the config path, and return the blob contents as a string
- **AND** return `Ok(None)` if the file does not exist at that commit

#### Scenario: Config file exceeds size limit

- **WHEN** the config file blob exceeds 1MB
- **THEN** the fetcher SHALL return an error
- **AND** no pipeline SHALL be triggered

### Requirement: PipelineStarter drives orchestrator

The system SHALL implement `PipelineStarter` by calling `PipelineOrchestrator::start_run()` with the trigger event data.

#### Scenario: Successful pipeline start

- **WHEN** `OrchestratorPipelineStarter::start_pipeline(event)` is called
- **THEN** it SHALL create a `PipelineRun` via the orchestrator
- **AND** return the run ID

#### Scenario: Pipeline start on non-leader node

- **WHEN** `start_pipeline` is called on a follower node
- **THEN** the orchestrator SHALL return a `NOT_LEADER` error
- **AND** the trigger service SHALL silently drop the trigger (leader will also receive gossip)

### Requirement: Node startup wires Forge-CI integration

When both `forge` and `ci` features are enabled, the node startup sequence SHALL create `ForgeConfigFetcher`, `OrchestratorPipelineStarter`, `TriggerService`, and register `CiTriggerHandler` with the Forge gossip service.

#### Scenario: Both features enabled

- **WHEN** a node starts with both `forge` and `ci` features enabled
- **THEN** the `TriggerService` SHALL be created and registered with Forge gossip
- **AND** the trigger service SHALL be ready to process push events

#### Scenario: Only one feature enabled

- **WHEN** a node starts with only `ci` (no `forge`) or only `forge` (no `ci`)
- **THEN** no Forge-CI integration SHALL be wired
- **AND** no errors SHALL occur during startup
