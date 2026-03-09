## MODIFIED Requirements

### Requirement: Forge push triggers CI pipeline

The system SHALL automatically trigger a CI pipeline when a git push updates a ref on a watched repository. The trigger path SHALL use Forge gossip `RefUpdate` announcements received by `CiTriggerHandler`. The system SHALL buffer recent announcements and replay them when a repository is watched, preventing race conditions between `ci watch` and `git push`.

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

#### Scenario: Push arrives before watch (race condition)

- **WHEN** a `RefUpdate` gossip announcement arrives for repo R
- **AND** repo R is NOT yet being watched
- **AND** within 30 seconds, `ci watch R` is called
- **THEN** the buffered announcement SHALL be replayed
- **AND** the pipeline SHALL be triggered as if the push arrived after the watch

#### Scenario: Buffer expires old announcements

- **WHEN** a `RefUpdate` gossip announcement arrives for repo R
- **AND** more than 30 seconds pass without `ci watch R` being called
- **THEN** the announcement SHALL be evicted from the buffer
- **AND** no pipeline SHALL be triggered if `ci watch R` is called later

#### Scenario: Buffer capacity limit

- **WHEN** the replay buffer contains 32 entries
- **AND** a new announcement arrives
- **THEN** the oldest entry SHALL be evicted to make room
