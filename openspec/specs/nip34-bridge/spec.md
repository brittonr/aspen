## ADDED Requirements

### Requirement: Publish repo announcement on create

The system SHALL publish a NIP-34 kind 30617 event to the embedded relay when a repository is created, containing the repo name and ID as tags.

#### Scenario: Repo created

- **WHEN** a new repository is created via ForgeCreateRepo
- **THEN** a kind 30617 event SHALL be published to the relay with d-tag set to the repo ID and a name tag

### Requirement: Publish push event

The system SHALL publish a NIP-34 event to the embedded relay when a ref is updated via git push.

#### Scenario: Push to main branch

- **WHEN** a git push updates refs/heads/main
- **THEN** an event SHALL be published to the relay with repo ID, ref name, and new commit hash as tags

### Requirement: Bridge is decoupled from forge

The NIP-34 bridge SHALL be a hook subscriber. Forge operations SHALL succeed even if the relay is unavailable.

#### Scenario: Relay unavailable during push

- **WHEN** a push occurs but the Nostr relay is not running
- **THEN** the push SHALL succeed and the bridge failure SHALL be logged but not returned as an error
