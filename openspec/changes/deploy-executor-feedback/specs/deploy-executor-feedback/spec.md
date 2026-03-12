## ADDED Requirements

### Requirement: Executor writes failure status to KV

When the background `NodeUpgradeExecutor` task fails at any stage (validation, profile switch, restart), it SHALL write `NodeDeployStatus::Failed(reason)` to the KV key `_sys:deploy:node:{node_id}` before exiting.

#### Scenario: Binary validation fails

- **WHEN** `upgrade_nix()` fails because the expected binary does not exist in the store path
- **THEN** the executor writes `Failed("StorePathUnavailable: /nix/store/...: bin/aspen-node not found")` to `_sys:deploy:node:{node_id}`

#### Scenario: Nix store realise fails

- **WHEN** `nix-store --realise` fails for the store path
- **THEN** the executor writes `Failed` with the realise error to the node status key

#### Scenario: Profile switch fails

- **WHEN** `nix-env --profile --set` returns non-zero
- **THEN** the executor writes `Failed` with the nix-env stderr to the node status key

### Requirement: Coordinator detects pre-restart failures via KV

The deployment coordinator SHALL check the KV-stored node deploy status during health polling. If the status is `Failed`, the coordinator SHALL stop waiting and mark the node as failed immediately.

#### Scenario: Executor fails before restart

- **WHEN** the executor writes `Failed` to KV and the node never restarts
- **THEN** the coordinator's next health poll reads the `Failed` status from KV and marks the node failed in the deployment record

#### Scenario: Executor succeeds and node restarts normally

- **WHEN** the executor succeeds and the node restarts
- **THEN** the coordinator's health poll uses `GetHealth` RPC as before (KV status is stale from pre-restart, but GetHealth returns the live status)

#### Scenario: KV read fails during health poll

- **WHEN** the KV read for node deploy status returns an error
- **THEN** the coordinator logs the error and continues with the `GetHealth` RPC check (KV check is best-effort, not blocking)
