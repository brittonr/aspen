## ADDED Requirements

### Requirement: Multinode deploy test uses validate_only mode

The `ci-dogfood-deploy-multinode` test's deploy stage SHALL use `validate_only = true` with `expected_binary = "bin/cowsay"` to verify the deploy coordination path without requiring actual nix profile switching.

#### Scenario: Deploy stage succeeds with validate_only

- **WHEN** the CI pipeline reaches the deploy stage
- **THEN** the deploy executor SHALL validate the cowsay artifact exists at the expected path
- **AND** the deploy stage SHALL complete with status `success`

### Requirement: Cluster health survives deploy pipeline

The 3-node Raft cluster SHALL remain healthy after the full pipeline (build + deploy) completes.

#### Scenario: All nodes healthy and KV data survives

- **WHEN** the deploy pipeline completes
- **THEN** all 3 nodes SHALL be healthy voters
- **AND** sentinel KV data written before deploy SHALL still be readable
- **AND** new KV writes after deploy SHALL succeed

### Requirement: ci-dogfood-deploy-multinode VM test passes

The `ci-dogfood-deploy-multinode-test` NixOS VM test SHALL pass within its configured timeouts.

#### Scenario: Full test passes

- **WHEN** `nix build .#checks.x86_64-linux.ci-dogfood-deploy-multinode-test` is run
- **THEN** all subtests SHALL pass including cluster formation, plugin install, build stage, deploy stage, and post-deploy health checks
