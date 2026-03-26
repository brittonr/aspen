## ADDED Requirements

### Requirement: Sync fetches remote ref heads

The `federation sync --peer` command SHALL call `GetResourceState` for each resource returned by `ListResources` and include ref heads in the sync result.

#### Scenario: Sync returns ref heads from remote repository

- **WHEN** Bob runs `federation sync --peer <alice_node_id> --addr <alice_addr>` and Alice has a federated repository with refs
- **THEN** the sync result SHALL include the repository's ref names and head hashes

### Requirement: Synced refs persisted to local KV

The system SHALL write synced remote ref state to local KV under `_fed:sync:<origin_short>:<local_id_short>:refs/<name>` so the data persists beyond the RPC call.

#### Scenario: Synced refs readable from local KV after sync

- **WHEN** Bob syncs from Alice and Alice's repo has `refs/heads/main`
- **THEN** Bob's local KV SHALL contain a key matching `_fed:sync:*:refs/heads/main` with the hex hash as value
