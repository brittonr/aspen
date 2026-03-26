## ADDED Requirements

### Requirement: Fetch refs from remote federated repository

The `federation fetch` CLI command SHALL connect to a remote peer, request ref objects for a specific federated resource via `SyncObjects`, and persist them locally under `_fed:mirror:{origin_short}:{local_id_short}:refs/{name}`.

#### Scenario: Fetch refs for a previously synced repository

- **WHEN** Bob runs `federation fetch --peer <alice_node_id> --fed-id <fed_id>` and Alice's repo has refs `heads/main` and `heads/dev`
- **THEN** Bob's local KV SHALL contain keys `_fed:mirror:*:refs/heads/main` and `_fed:mirror:*:refs/heads/dev` with the corresponding hash values

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
