## ADDED Requirements

### Requirement: Federate repository via RPC

The `federation federate <repo_id>` CLI command SHALL persist `FederationSettings` to KV via `ForgeNode::set_federation_settings`, using a `FederatedId` constructed from the cluster's public key and the repo's BLAKE3 hash.

#### Scenario: Federate a repository as public

- **WHEN** a user runs `federation federate <repo_id> --mode public` on a cluster with federation enabled
- **THEN** the system SHALL write federation settings with mode=Public to KV, and subsequent calls to `federation list-federated` SHALL include that repository

#### Scenario: Federated repository appears in remote resource listing

- **WHEN** a remote cluster calls `ListResources` on a cluster that has a federated repository
- **THEN** the response SHALL include that repository's `FederatedId` and resource type `forge:repo`
