## ADDED Requirements

### Requirement: Federation fetch retrieves content

The `federation fetch` command SHALL fetch both ref entries AND git objects from the remote cluster, then create or update a local mirror repo.

#### Scenario: Fetch with content transfer

- **WHEN** a user runs `aspen-cli federation fetch --peer <id> --fed-id <id>`
- **THEN** the CLI connects to the remote, fetches ref entries, fetches git objects for all refs, creates/updates a local mirror repo, and reports counts of fetched vs already-present objects

#### Scenario: Fetch incremental update

- **WHEN** a user runs `federation fetch` for a resource that was previously fetched
- **THEN** only objects not present in the local mirror are transferred, using `have_hashes` from the existing mirror's blob store

#### Scenario: Fetch skips refs already present locally

- **WHEN** Bob already has `heads/main` with hash `aabb...` and Alice still has the same hash
- **THEN** the SyncObjects request SHALL include the existing hash in `have_hashes` and Alice SHALL not re-send that ref

### Requirement: ForgeResourceResolver returns ref entries as SyncObjects

The `ForgeResourceResolver::sync_objects` implementation SHALL scan `forge:refs:{repo_id}:*` and return each ref as a `SyncObject` with `object_type: "ref"`, BLAKE3 hash of the ref entry, and postcard-serialized ref data.

#### Scenario: Resolver returns refs for a federated repository

- **WHEN** a remote cluster calls `SyncObjects` with `want_types: ["refs"]` for a federated repo that has 3 refs
- **THEN** the response SHALL contain 3 `SyncObject` entries with `object_type: "ref"` and valid BLAKE3 hashes

#### Scenario: Resolver filters by have_hashes

- **WHEN** a remote cluster sends `SyncObjects` with `have_hashes` containing the hash of `heads/main`
- **THEN** the response SHALL NOT include the `heads/main` ref object

### Requirement: FederationFetchRefs RPC message

The client API SHALL define a `FederationFetchRefs` request with fields `peer_node_id`, `peer_addr` (optional), and `fed_id`. The response SHALL report the number of refs fetched, refs already present, and any errors.

#### Scenario: Successful fetch RPC

- **WHEN** a client sends `FederationFetchRefs { peer_node_id, fed_id }` and the remote has 5 refs, 2 already mirrored locally
- **THEN** the response SHALL report `fetched: 3, already_present: 2, errors: []`

#### Scenario: Fetch RPC with unreachable peer

- **WHEN** a client sends `FederationFetchRefs` with an unreachable `peer_node_id`
- **THEN** the response SHALL have `is_success: false` with an error message containing "connection"

### Requirement: Federation pull command

A new `federation pull` CLI command SHALL perform incremental sync of a previously mirrored federation resource without requiring the federated ID — it looks up the mirror's origin metadata.

#### Scenario: Pull updates existing mirror

- **WHEN** a user runs `aspen-cli federation pull --repo <mirror-repo-id>`
- **THEN** the system reads the mirror's origin metadata, connects to the origin cluster, fetches only new refs and objects, and updates the local mirror

#### Scenario: Pull on non-mirror repo fails

- **WHEN** a user runs `federation pull` on a repo that is not a federation mirror
- **THEN** the command fails with an error indicating the repo is not a mirror

### Requirement: Fetch reports transfer statistics

The `federation fetch` response SHALL include counts of objects transferred by type (commits, trees, blobs), total bytes transferred, and objects skipped.

#### Scenario: Fetch statistics reported

- **WHEN** a federation fetch completes
- **THEN** the CLI prints per-type object counts, total bytes, and already-present counts
