## ADDED Requirements

### Requirement: Two-cluster federation CI dogfood VM test

The system SHALL include a NixOS VM integration test (`federation-ci-dogfood.nix`) that proves the full cross-cluster pipeline: Cluster A pushes code to Forge, federates the repo, Cluster B syncs the federated repo and runs a CI build from the mirrored content.

#### Scenario: Alice creates repo, pushes flake, federates

- **WHEN** Alice's cluster creates a Forge repo and pushes a Nix flake with a buildable package
- **THEN** the repo SHALL be visible via `forge list`
- **AND** `federation federate <repo-id>` SHALL succeed

#### Scenario: Bob trusts Alice and syncs federated repo

- **WHEN** Bob's cluster trusts Alice's cluster key and runs `federation sync`
- **THEN** mirror metadata SHALL appear in Bob's KV store (`_fed:mirror:*` keys)
- **AND** `federation list-federated` on Bob SHALL show Alice's repo

#### Scenario: Bob triggers CI build from mirrored content

- **WHEN** Bob runs `ci trigger` on the mirrored repo
- **THEN** a CI pipeline SHALL start on Bob's cluster
- **AND** the pipeline SHALL build the Nix flake from the federated content

#### Scenario: CI build succeeds and produces runnable binary

- **WHEN** the CI pipeline on Bob's cluster completes
- **THEN** the pipeline status SHALL be "success"
- **AND** the build output SHALL be a valid Nix store path containing an executable
- **AND** running the binary SHALL produce expected output

#### Scenario: Two independent clusters with separate identities

- **WHEN** the test starts two VMs (alice, bob) with different cookies and secret keys
- **THEN** each cluster SHALL have its own cluster identity
- **AND** each cluster SHALL operate independently before federation
