## 1. Crate scaffolding

- [ ] 1.1 Create `crates/aspen-dogfood/` with `Cargo.toml`, binary target `aspen-dogfood`, deps: `aspen-client`, `aspen-client-api`, `aspen-core`, `aspen-cluster`, `aspen-ci-core`, `aspen-auth`, `tokio`, `clap`, `snafu`, `tracing`, `tracing-subscriber`, `serde`, `serde_json`
- [ ] 1.2 Add `aspen-dogfood` to workspace `Cargo.toml` members
- [ ] 1.3 Scaffold `src/main.rs` with clap subcommands: `start`, `stop`, `status`, `push`, `build`, `deploy`, `verify`, `full-loop`, `full` and top-level flags `--federation`, `--vm-ci`, `--cluster-dir`, `--node-count`

## 2. Error types and state file

- [ ] 2.1 Define snafu error enum in `src/error.rs` covering: client RPC failures, process spawn failures, state file I/O, timeout, node crash
- [ ] 2.2 Define `DogfoodState` struct in `src/state.rs` (node PIDs, cluster tickets, endpoint addresses, mode) with serde JSON serialization
- [ ] 2.3 Implement state file read/write/delete in the cluster directory

## 3. Process management

- [ ] 3.1 Implement `NodeManager` in `src/node.rs` — spawn `aspen-node` via `tokio::process::Command`, store `Child` handles, capture stderr
- [ ] 3.2 Implement health-check polling loop: connect via `AspenClient`, send `GetHealth` RPC, retry with backoff until healthy or timeout
- [ ] 3.3 Implement signal handler (`tokio::signal`) that terminates all managed nodes on SIGINT/SIGTERM with 10s grace period then SIGKILL
- [ ] 3.4 Implement node crash detection: `child.try_wait()` checks during orchestration steps, report stderr on unexpected exit

## 4. Cluster operations via client API

- [ ] 4.1 Implement `ClusterOps` in `src/cluster.rs` — typed wrappers around `AspenClient::send()` for `InitCluster`, `AddLearner`, `ChangeMembership`, `GetHealth`, `GetRaftMetrics`
- [ ] 4.2 Implement single-node cluster init: spawn node, health-check, `InitCluster` RPC, write state file
- [ ] 4.3 Implement federation cluster init: spawn two nodes, health-check both, init both, `AddPeerCluster` + federation trust setup

## 5. Forge and CI operations via client API

- [ ] 5.1 Implement `ForgeOps` in `src/forge.rs` — create repo via Forge RPC, check if repo exists via list RPC
- [ ] 5.2 Implement git push step: configure `aspen://` remote, run `git push` via `tokio::process::Command` (git-remote-aspen is the one required external process)
- [ ] 5.3 Implement `CiOps` in `src/ci.rs` — trigger build via CI RPC, poll status with exponential backoff (1s→10s), fetch logs on failure/timeout
- [ ] 5.4 Implement CI timeout handling: configurable timeout (default 600s), fetch last 50 log lines via RPC on timeout

## 6. Deploy and verify operations

- [ ] 6.1 Implement `DeployOps` in `src/deploy.rs` — trigger deploy via RPC, poll deploy status until complete
- [ ] 6.2 Implement artifact verification: retrieve blob via RPC, execute with `--version`, check output
- [ ] 6.3 Implement post-deploy verification: confirm running node reports new version via `GetRaftMetrics`

## 7. Subcommand wiring

- [ ] 7.1 Wire `start` subcommand: call cluster init (single or federation based on flags), write state file
- [ ] 7.2 Wire `stop` subcommand: read state file, terminate nodes, delete state file and cluster dir
- [ ] 7.3 Wire `status` subcommand: read state file, connect to cluster, display health/metrics
- [ ] 7.4 Wire `push` subcommand: read state file, connect, create repo if needed, git push
- [ ] 7.5 Wire `build` subcommand: read state file, connect, trigger or wait for CI pipeline
- [ ] 7.6 Wire `deploy` subcommand: read state file, connect, trigger deploy, wait for completion
- [ ] 7.7 Wire `verify` subcommand: read state file, connect, run artifact + deploy verification
- [ ] 7.8 Wire `full-loop` subcommand: build → deploy → verify in sequence
- [ ] 7.9 Wire `full` subcommand: start → push → build → deploy → verify → stop, abort on first failure

## 8. VM-CI and federation modes

- [ ] 8.1 Add `--vm-ci` flag handling: configure node with VM executor settings when starting cluster
- [ ] 8.2 Add `--federation` flag handling: start two clusters, push to first, verify sync to second, build on second
- [ ] 8.3 Test that `--federation --vm-ci` composes (federation with VM-isolated CI on the second cluster)

## 9. Nix integration

- [ ] 9.1 Update `flake.nix`: add `aspen-dogfood` to crane build, create `dogfood-local` app entry pointing to new binary
- [ ] 9.2 Ensure `ASPEN_NODE_BIN`, `ASPEN_CLI_BIN`, `GIT_REMOTE_ASPEN_BIN`, `PROJECT_DIR` env vars are injected by the flake wrapper (same as current scripts)
- [ ] 9.3 Update `dogfood-local-vmci` and `dogfood-federation` flake app entries to use the new binary with appropriate flags

## 10. Testing and migration

- [ ] 10.1 Unit tests for state file serialization roundtrip
- [ ] 10.2 Unit tests for error type context messages
- [ ] 10.3 Integration test: start single-node cluster, health-check, stop (requires built `aspen-node`)
- [ ] 10.4 Run the new binary through the same scenarios as the existing `dogfood-local.sh` and confirm identical outcomes
- [ ] 10.5 Update NixOS VM tests (`ci-dogfood-test`, `ci-dogfood-self-build`) to use the new binary
- [ ] 10.6 Move old scripts to `scripts/deprecated/` with a note in each pointing to `aspen-dogfood`
- [ ] 10.7 Update `AGENTS.md` dogfood section with new commands
