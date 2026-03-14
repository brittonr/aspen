## 1. ADR Framework

- [x] 1.1 Create `docs/adr/` directory and `docs/adr/README.md` with format conventions (Context/Decision/Consequences, +/~/- annotations, numbering scheme)
- [x] 1.2 Write ADR-001: Iroh-only networking (no HTTP API)
- [x] 1.3 Write ADR-002: Vendored openraft v0.10
- [x] 1.4 Write ADR-003: Redb unified Raft log + state machine storage
- [x] 1.5 Write ADR-004: Functional Core Imperative Shell pattern
- [x] 1.6 Write ADR-005: Verus two-file verification architecture
- [x] 1.7 Write ADR-006: Tiger Style resource bounds
- [x] 1.8 Write ADR-007: Nickel for CI configuration
- [x] 1.9 Write ADR-008: Content-addressed blobs via iroh-blobs
- [x] 1.10 Write ADR-009: ALPN-based protocol routing
- [x] 1.11 Write ADR-010: Madsim deterministic simulation testing

## 2. flake-parts Migration

- [x] 2.1 Add `flake-parts` to flake inputs, remove `flake-utils`
- [x] 2.2 Create `nix/flake-modules/` directory structure
- [x] 2.3 Extract dev shell into `nix/flake-modules/dev.nix` — verify `nix develop` works
- [x] 2.4 Extract Rust/crane build logic into `nix/flake-modules/rust.nix` — verify `nix build` parity
- [x] 2.5 Extract checks (clippy, fmt, audit) into `nix/flake-modules/checks.nix` — verify `nix flake check`
- [x] 2.6 Extract NixOS VM tests into `nix/flake-modules/tests.nix`
- [x] 2.7 Extract dogfood pipeline into `nix/flake-modules/dogfood.nix`
- [x] 2.8 Extract Verus verification into `nix/flake-modules/verus.nix`
- [x] 2.9 Extract VM images/microvm configs into `nix/flake-modules/vms.nix`
- [x] 2.10 Extract runnable apps into `nix/flake-modules/apps.nix`
- [x] 2.11 Reduce root `flake.nix` to inputs + imports only — verify all outputs match pre-migration

## 3. Schema-Driven CI Agent Protocol

- [x] 3.1 Add `#[derive(JsonSchema)]` to `HostMessage`, `AgentMessage`, `ExecutionRequest`, `ExecutionResult`, `LogMessage` in `crates/aspen-ci/src/agent/protocol.rs`
- [x] 3.2 Create `schemas/` directory at repo root
- [x] 3.3 Write snapshot test that generates `schemas/ci-agent-protocol.json` and asserts it matches checked-in version
- [x] 3.4 Add `UPDATE_SNAPSHOTS=1` env var support to regenerate the snapshot file
- [x] 3.5 Add `#[derive(JsonSchema)]` to `DeployRequest`, `DeployInitResult`, `DeployStatusResult`, `DeployNodeStatus` in deploy_executor.rs
- [x] 3.6 Write snapshot test for `schemas/deploy-protocol.json`
- [x] 3.7 Check in initial schema files and verify `cargo nextest run` passes

## 4. Deploy Resource Statefulness

- [x] 4.1 Add `stateful: bool` field to `DeployRequest` with `#[serde(default = "default_true")]`
- [x] 4.2 Add `deploy.stateful` option to the Nickel CI config schema (default: `true`)
- [x] 4.3 Update pipeline config loader to parse `stateful` from Nickel config into `DeployRequest`
- [x] 4.4 Implement stateful path in `DeployExecutor::execute()` — write lifecycle state to `_deploy:state:{deploy_id}:metadata`, per-node state, and rollback point
- [x] 4.5 Implement stateless path in `DeployExecutor::execute()` — skip all `_deploy:state:` KV writes
- [x] 4.6 Write unit test: stateful deploy writes expected KV keys
- [x] 4.7 Write unit test: stateless deploy writes zero `_deploy:state:` keys
- [x] 4.8 Write unit test: default statefulness is `true` when field omitted

## 5. CI Process Isolation

- [x] 5.1 Create `NixBuildSupervisor` struct in `crates/aspen-ci/src/orchestrator/` with `spawn_build()` method
- [x] 5.2 Implement child process management using `tokio::process::Command` with stdout/stderr capture
- [x] 5.3 Implement configurable timeout (`nix_build_timeout_secs`, default 1800) with SIGKILL on expiry
- [x] 5.4 Implement `tokio::sync::oneshot` channel for result delivery to orchestrator
- [x] 5.5 Implement `tokio::sync::mpsc` channel for real-time log streaming
- [x] 5.6 Integrate `NixBuildSupervisor` into `PipelineOrchestrator` for shell/Nix executor builds
- [x] 5.7 Write test: build timeout kills child process and reports failure
- [x] 5.8 Write test: orchestrator continues processing after build process death
- [x] 5.9 Write test: pipeline state in KV reflects failed job after supervisor timeout
