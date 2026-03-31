## ADDED Requirements

### Requirement: Single pipeline per push event

A RefUpdate gossip announcement for a given repo+commit MUST result in at most one pipeline run, regardless of how many nodes receive the announcement.

#### Scenario: 3-node cluster receives gossip broadcast

- **WHEN** a git push triggers a RefUpdate announcement
- **AND** the announcement is received by all 3 nodes via gossip
- **THEN** only the Raft leader processes the CI trigger
- **AND** follower nodes drop the trigger silently

#### Scenario: Leadership change during trigger processing

- **WHEN** the leader receives a trigger and begins processing
- **AND** leadership transfers before `start_pipeline()` completes
- **THEN** the new leader may also receive the trigger via gossip replay
- **AND** pipeline creation is idempotent (duplicate runs for same commit are prevented or harmless)
