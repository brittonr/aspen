## ADDED Requirements

### Requirement: Federation sync emits RefUpdate gossip

The system SHALL emit RefUpdate gossip for each mirror ref updated during federation sync.

#### Scenario: Federation pull emits gossip

- **WHEN** federation pull updates mirror ref `refs/heads/main`
- **THEN** a RefUpdate announcement SHALL be broadcast via gossip

### Requirement: TriggerService auto-watches federation mirrors

The system SHALL scan `_fed:mirror:*` KV prefix and auto-watch discovered mirror repos.

#### Scenario: Existing mirrors watched on startup

- **WHEN** TriggerService starts with federation_ci_enabled=true
- **THEN** it SHALL scan and watch all existing mirrors

### Requirement: Federation CI requires opt-in

The system SHALL NOT trigger CI for mirrors unless `federation_ci_enabled` is true.

#### Scenario: Disabled by default

- **WHEN** RefUpdate arrives for mirror repo and federation_ci_enabled=false
- **THEN** the trigger SHALL be skipped
