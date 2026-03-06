## MODIFIED Requirements

### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes. When a cluster cache is available, Nix builds SHALL automatically use it as a substituter.

#### Scenario: Sequential stages

- GIVEN stages `build → test → deploy` with dependencies
- WHEN the pipeline executes
- THEN `test` SHALL NOT start until `build` completes successfully
- AND `deploy` SHALL NOT start until `test` completes successfully

#### Scenario: Parallel jobs within a stage

- GIVEN a `test` stage with jobs `[unit-tests, lint, clippy]` with no inter-dependencies
- WHEN the stage executes
- THEN all three jobs MAY run in parallel on different workers

#### Scenario: Nix build with cache substituter

- GIVEN a CI job that runs `nix build` and `use_cluster_cache` is enabled
- WHEN the executor runs the build command
- THEN it SHALL inject `--substituters http://127.0.0.1:{proxy_port}` and `--trusted-public-keys {cache_public_key}` into the Nix invocation
- AND the cache proxy SHALL translate HTTP requests to aspen client RPC

#### Scenario: Nix build without cache

- GIVEN a CI job that runs `nix build` and `use_cluster_cache` is disabled
- WHEN the executor runs the build command
- THEN it SHALL NOT modify the Nix invocation's substituter configuration
