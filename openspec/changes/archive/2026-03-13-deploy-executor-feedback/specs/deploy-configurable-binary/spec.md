## ADDED Requirements

### Requirement: Configurable binary validation path

The `NodeUpgradeConfig` SHALL include an `expected_binary` field (`Option<String>`) that specifies which binary to validate inside a Nix store path. When `Some(path)`, `upgrade_nix()` SHALL check for that path instead of hardcoded `bin/aspen-node`. When `None`, `upgrade_nix()` SHALL skip binary existence validation and only verify the store path is available.

#### Scenario: Default binary check

- **WHEN** `expected_binary` is `Some("bin/aspen-node")` and the store path contains `bin/aspen-node`
- **THEN** validation passes and the profile switch proceeds

#### Scenario: Custom binary check

- **WHEN** `expected_binary` is `Some("bin/cowsay")` and the store path contains `bin/cowsay`
- **THEN** validation passes and the profile switch proceeds

#### Scenario: Custom binary check fails

- **WHEN** `expected_binary` is `Some("bin/cowsay")` and the store path does not contain `bin/cowsay`
- **THEN** `upgrade_nix()` returns `StorePathUnavailable` error

#### Scenario: No binary check

- **WHEN** `expected_binary` is `None` and the store path exists
- **THEN** validation passes (binary existence not checked), profile switch proceeds

### Requirement: NodeUpgrade RPC carries expected_binary

The `ClientRpcRequest::NodeUpgrade` variant SHALL include an `expected_binary: Option<String>` field. The deploy handler SHALL pass this value through to `NodeUpgradeConfig` when constructing the executor.

#### Scenario: RPC with expected_binary set

- **WHEN** a NodeUpgrade RPC arrives with `expected_binary: Some("bin/cowsay")`
- **THEN** the handler creates `NodeUpgradeConfig` with `expected_binary: Some("bin/cowsay")`

#### Scenario: RPC without expected_binary (backwards compatibility)

- **WHEN** a NodeUpgrade RPC arrives with `expected_binary: None` (or from an older node that doesn't send the field)
- **THEN** the handler creates `NodeUpgradeConfig` with `expected_binary: Some("bin/aspen-node")` (the default)

### Requirement: Deploy CI config supports expected_binary

The deploy job configuration in CI SHALL support an optional `expected_binary` field that flows through the deploy pipeline to the NodeUpgrade RPC.

#### Scenario: CI config specifies expected_binary

- **WHEN** a deploy job has `expected_binary = "bin/cowsay"` in its config
- **THEN** the ClusterDeploy request carries `expected_binary` and the coordinator passes it to each NodeUpgrade RPC

#### Scenario: CI config omits expected_binary

- **WHEN** a deploy job has no `expected_binary` field
- **THEN** the default `bin/aspen-node` is used
