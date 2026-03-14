## ADDED Requirements

### Requirement: Statefulness flag on deploy requests

`DeployRequest` SHALL include a `stateful: bool` field that controls whether the deploy executor tracks lifecycle state in Raft KV.

#### Scenario: Stateful deploy request

- **WHEN** a deploy job is created with `stateful: true`
- **THEN** `DeployRequest.stateful` is `true`

#### Scenario: Stateless deploy request

- **WHEN** a deploy job is created with `stateful: false`
- **THEN** `DeployRequest.stateful` is `false`

#### Scenario: Default statefulness

- **WHEN** the Nickel CI config omits the `deploy.stateful` field
- **THEN** `DeployRequest.stateful` defaults to `true` for backwards compatibility

### Requirement: Stateful lifecycle tracking in KV

When `stateful` is `true`, the deploy executor SHALL write lifecycle state to Raft KV under the key prefix `_deploy:state:{deploy_id}:`.

#### Scenario: State written on deploy initiation

- **WHEN** a stateful deploy is initiated
- **THEN** a KV entry is written at `_deploy:state:{deploy_id}:metadata` containing the artifact reference, strategy, timestamp, and node list

#### Scenario: Per-node state tracking

- **WHEN** a node's deploy status changes during a stateful deploy
- **THEN** the KV entry at `_deploy:state:{deploy_id}:node:{node_id}` is updated with the new status

#### Scenario: Rollback point recorded

- **WHEN** a stateful deploy completes successfully
- **THEN** the previous artifact reference is stored at `_deploy:state:{deploy_id}:rollback` for future rollback operations

### Requirement: Stateless deploy skips KV writes

When `stateful` is `false`, the deploy executor SHALL NOT write any lifecycle state to Raft KV. Only CI job logs (under `_ci:logs:`) are written.

#### Scenario: No state keys for stateless deploy

- **WHEN** a stateless deploy completes (success or failure)
- **THEN** no keys exist under `_deploy:state:{deploy_id}:`

#### Scenario: Logs still written for stateless deploy

- **WHEN** a stateless deploy executes
- **THEN** CI job log entries are still written under `_ci:logs:{run_id}:{job_id}:*`

### Requirement: Nickel config deploy.stateful option

The CI Nickel configuration schema SHALL include a `deploy.stateful` boolean option for deploy stages.

#### Scenario: Stateful option in Nickel config

- **WHEN** a CI pipeline Nickel file sets `deploy.stateful = false` on a deploy stage
- **THEN** the parsed `DeployRequest` has `stateful` set to `false`

#### Scenario: Schema includes stateful field

- **WHEN** the Nickel schema is generated from Rust types
- **THEN** the deploy stage schema includes a `stateful` boolean field with default `true`
