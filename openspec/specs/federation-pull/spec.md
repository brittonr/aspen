## ADDED Requirements

### Requirement: Cold pull from remote cluster

The system SHALL allow pulling a repo from a remote cluster by specifying the peer's iroh node ID and the remote repo ID (hex-encoded). If no local mirror exists, one SHALL be created. The remote's cluster identity key SHALL be obtained via federation handshake and used to construct the FederatedId.

#### Scenario: First pull creates mirror

- **WHEN** user runs `federation pull --peer <node-id> --repo <remote-repo-hex>`
- **THEN** the system connects to the remote peer, performs a federation handshake, fetches all refs and git objects for the specified repo, creates a local mirror repo, stores mirror metadata with the origin's cluster key, node ID, and address hint, and reports the number of objects fetched

#### Scenario: Cold pull with address hint

- **WHEN** user runs `federation pull --peer <node-id> --repo <remote-repo-hex> --addr 192.168.1.5:12345`
- **THEN** the system uses the address hint to connect to the remote peer without relay or discovery, and the address hint is stored in mirror metadata for future pulls

#### Scenario: Remote repo not found

- **WHEN** user runs `federation pull --peer <node-id> --repo <nonexistent-hex>`
- **THEN** the system returns an error indicating the remote resource was not found

#### Scenario: Remote peer unreachable

- **WHEN** user runs `federation pull --peer <node-id> --repo <hex>` and the peer is offline
- **THEN** the system returns a connection error within the handshake timeout (10s)

### Requirement: Incremental pull updates

The system SHALL perform incremental fetches on subsequent pulls. Only objects not already present in the local mirror SHALL be transferred. The system SHALL send BLAKE3 hashes of locally stored objects as `have_hashes` in the SyncObjects request.

#### Scenario: Second pull transfers only new objects

- **WHEN** a mirror was created by a previous pull, and the origin repo has new commits since then
- **THEN** running `federation pull --repo <mirror-id>` transfers only the new objects and updates refs, reporting both fetched and already-present counts

#### Scenario: Pull with no changes

- **WHEN** a mirror is up to date with the origin
- **THEN** running `federation pull --repo <mirror-id>` reports 0 fetched and the existing object count as already-present

### Requirement: Mirror metadata persistence

Mirror metadata SHALL store `origin_cluster_key`, `origin_node_id`, `fed_id`, `local_repo_id`, and optionally `origin_addr_hint`. The `origin_node_id` is the iroh transport public key used for reconnection. The `origin_cluster_key` is the federation identity key used for trust verification.

#### Scenario: Stored metadata enables subsequent pull without --peer

- **WHEN** a mirror was created by a cold pull with `--peer <node-id>`
- **THEN** running `federation pull --repo <mirror-id>` (without `--peer`) connects to the origin using the stored node ID and succeeds

#### Scenario: Address hint stored from cold pull

- **WHEN** a cold pull is performed with `--addr <hint>`
- **THEN** the mirror metadata contains the address hint and subsequent pulls use it

### Requirement: CLI argument validation

The CLI SHALL validate that pull arguments form a valid combination. Two modes are supported: cold pull (`--peer` + `--repo` as remote ID) and mirror pull (`--repo` as local mirror ID).

#### Scenario: Cold pull requires both --peer and --repo

- **WHEN** user runs `federation pull --peer <node-id>` without `--repo`
- **THEN** the CLI returns an error requiring `--repo`

#### Scenario: Ambiguity resolved by --peer presence

- **WHEN** user runs `federation pull --peer <node-id> --repo <hex>`
- **THEN** `--repo` is interpreted as the remote repo ID, not a local mirror ID
