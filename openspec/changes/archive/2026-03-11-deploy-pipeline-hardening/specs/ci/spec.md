## MODIFIED Requirements

### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes. When a cluster cache is available, Nix builds SHALL automatically use it as a substituter. Pipeline runs SHALL be indexed by repository and ref for efficient status queries. Deploy stages SHALL send real RPCs to target nodes via iroh QUIC using the shared IrohNodeRpcClient.

#### Scenario: Sequential stages

- **WHEN** stages `build → test → deploy` with dependencies
- **THEN** `test` SHALL NOT start until `build` completes successfully
- **AND** `deploy` SHALL NOT start until `test` completes successfully

#### Scenario: Parallel jobs within a stage

- **WHEN** a `test` stage with jobs `[unit-tests, lint, clippy]` with no inter-dependencies
- **THEN** all three jobs MAY run in parallel on different workers

#### Scenario: Nix build with cache substituter

- **WHEN** a CI job that runs `nix build` and `use_cluster_cache` is enabled
- **THEN** it SHALL inject `--substituters http://127.0.0.1:{proxy_port}` and `--trusted-public-keys {cache_public_key}` into the Nix invocation
- **AND** the cache proxy SHALL translate HTTP requests to aspen client RPC

#### Scenario: Nix build without cache

- **WHEN** a CI job that runs `nix build` and `use_cluster_cache` is disabled
- **THEN** it SHALL NOT modify the Nix invocation's substituter configuration

#### Scenario: Job failure propagation

- **WHEN** `build` fails in a `build → test → deploy` pipeline
- **THEN** `test` and `deploy` SHALL be skipped
- **AND** the pipeline SHALL be marked as failed

#### Scenario: Pipeline status indexed by ref

- **WHEN** a pipeline run starts for repo `R` on ref `refs/heads/main`
- **THEN** the latest run ID SHALL be written to `_ci:ref-status:{repo_hex}:refs/heads/main`
- **AND** querying `ci status R main` SHALL return the latest pipeline run status

#### Scenario: Pipeline status updated on completion

- **WHEN** a pipeline run completes (success or failure)
- **THEN** the ref-status index SHALL be updated with final status
- **AND** the run record at `_ci:runs:{run_id}` SHALL reflect the terminal state

#### Scenario: Deploy stage sends real RPCs

- **WHEN** a deploy stage executes via the CI pipeline orchestrator
- **THEN** RpcDeployDispatcher SHALL use IrohNodeRpcClient to send NodeUpgrade RPCs to target nodes via iroh QUIC
- **AND** the RPCs SHALL use the same CLIENT_ALPN protocol as the cluster handler's deploy path
