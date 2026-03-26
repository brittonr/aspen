## ADDED Requirements

### Requirement: Mirror repo creation from fetched federation data

After fetching git objects from a remote federated cluster, the system SHALL create a local forge repo tagged as a mirror. The mirror repo SHALL be readable via standard forge operations.

#### Scenario: First fetch creates mirror

- **WHEN** `federation fetch` retrieves refs and git objects for a federated resource that has no local mirror
- **THEN** a new forge repo is created, objects are imported via `GitBridgeImporter`, refs are set, and the repo is tagged in KV as `_fed:mirror:{fed_id}` with the origin cluster key

#### Scenario: Subsequent fetch updates mirror

- **WHEN** `federation fetch` runs for a federated resource that already has a local mirror
- **THEN** only new/changed objects are imported, refs are updated to match remote state, and existing objects are untouched

### Requirement: Mirror repos are read-only

Mirror repos SHALL reject write operations (push, ref update) through forge handlers. The mirror tag in KV indicates the repo is externally managed.

#### Scenario: Push to mirror rejected

- **WHEN** a user attempts to `git push` to a mirror repo via git-remote-aspen
- **THEN** the push is rejected with an error indicating the repo is a read-only federation mirror

### Requirement: Mirror repos are discoverable

Mirror repos SHALL appear in `forge list` output with a `[mirror]` indicator and the origin cluster identity.

#### Scenario: List repos shows mirrors

- **WHEN** a user runs `forge list` on a cluster with federation mirrors
- **THEN** mirror repos are listed with their origin cluster name and federation ID

### Requirement: Mirror repo tracks origin metadata

Each mirror repo SHALL store the federated ID, origin cluster public key, and last sync timestamp in KV metadata.

#### Scenario: Mirror metadata queryable

- **WHEN** a user runs `forge info` on a mirror repo
- **THEN** the output includes origin cluster key, federated ID, and last sync time
