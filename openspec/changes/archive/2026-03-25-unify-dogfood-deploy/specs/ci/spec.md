## MODIFIED Requirements

### Requirement: Dogfood deploy uses cluster deploy RPC

The dogfood script's `do_deploy` function SHALL use `aspen-cli cluster deploy --wait` to perform rolling deployments instead of manually stopping, restarting, and health-checking nodes in bash.

#### Scenario: Single-node deploy via CLI

- **WHEN** `dogfood-local.sh deploy` is run against a 1-node cluster
- **AND** a successful CI pipeline exists with a build artifact
- **THEN** the script SHALL extract the artifact store path from the pipeline
- **AND** SHALL invoke `aspen-cli cluster deploy <artifact> --wait --timeout 600`
- **AND** SHALL exit successfully when the CLI reports deployment completed

#### Scenario: Multi-node deploy via CLI

- **WHEN** `dogfood-local.sh deploy` is run against a 3-node cluster
- **AND** a successful CI pipeline exists with a build artifact
- **THEN** the script SHALL invoke `aspen-cli cluster deploy <artifact> --wait --timeout 1200`
- **AND** the `DeploymentCoordinator` SHALL handle follower-first ordering and quorum safety
- **AND** the script SHALL NOT manually stop/restart nodes

#### Scenario: Deploy failure propagates exit code

- **WHEN** `dogfood-local.sh deploy` is run
- **AND** the `aspen-cli cluster deploy --wait` exits with non-zero
- **THEN** `do_deploy` SHALL return non-zero
- **AND** SHALL print the CLI's error output
