## 1. Test binary: aspen-job-primitives-test

- [x] 1.1 Create `crates/aspen-job-primitives-test/` with Cargo.toml depending on `aspen-jobs`, `aspen-testing`, `aspen-core`, `aspen-kv-types`, `tokio`, `clap`, `serde_json`, `tracing`, `tracing-subscriber`
- [x] 1.2 Scaffold `src/main.rs` with clap subcommands: `dependency`, `scheduler`, `dlq`, `saga`, `workflow`, `affinity`, `replay`, `all`
- [x] 1.3 Implement `dependency` subcommand: build diamond DAG (A→B, A→C, B+C→D), submit to `DependencyGraph`, execute with `DeterministicKeyValueStore`, assert topological ordering. Second scenario: fail B, assert D is blocked.
- [x] 1.4 Implement `scheduler` subcommand: register delayed job (5s) and cron job (`*/10 * * * * *`), use `tokio::time::pause()`/`advance()`, assert delayed job fires after 5s, cron job fires 2+ times in 25s.
- [x] 1.5 Implement `dlq` subcommand: submit always-failing job with max_retries=2, run through `JobManager`, assert 2 retries then DLQ placement with correct metadata. Then retry from DLQ, assert job moves back to Pending.
- [x] 1.6 Implement `saga` subcommand: build 4-step saga via `SagaBuilder`, fail step 3, assert compensation for steps 2,1 in LIFO order. Second scenario: all succeed, assert no compensation.
- [x] 1.7 Implement `workflow` subcommand: define workflow with conditional transitions (A→B on success, A→C on failure) via `WorkflowBuilder`, run with A succeeding, assert B runs and C doesn't. Second run with A failing, assert C runs and B doesn't.
- [x] 1.8 Implement `affinity` subcommand: register two workers with different tags, submit job with `gpu=true` affinity, assert routed to matching worker. Submit job with `region=eu`, assert stays Pending.
- [x] 1.9 Implement `replay` subcommand: execute job with `DeterministicJobExecutor` recording enabled, replay the `ExecutionRecord`, assert output matches. Second scenario: replay with modified worker, assert divergence detected.
- [x] 1.10 Implement `all` subcommand that runs every test and reports pass/fail summary.
- [x] 1.11 Add crate to workspace `Cargo.toml` members list.

## 2. Distributed pool test (CLI-based in Nix test)

- [x] 2.1-2.6 Distributed pool test uses `aspen-cli` directly in the NixOS VM test script (matches project conventions). No separate binary needed — the multi-node test in task group 4 covers this.

## 3. NixOS VM test: job-primitives.nix

- [x] 3.1 Create `nix/tests/job-primitives.nix` following the `durable-workflow-failover.nix` pattern: single VM, `environment.systemPackages = [jobPrimitivesTestBin]`, 1024MB RAM, 2 cores.
- [x] 3.2 Add subtests for each subcommand: dependency, scheduler, dlq, saga, workflow, affinity, replay. Each subtest calls `aspen-job-primitives-test <subcommand> 2>&1` and asserts `PASS` in output.
- [x] 3.3 Add `ciVmTestBin` entry in `flake.nix` for `aspen-job-primitives-test`.
- [x] 3.4 Add `job-primitives-test` check in `flake.nix` that imports `nix/tests/job-primitives.nix` and passes the test binary.

## 4. NixOS VM test: distributed-worker-pool.nix

- [x] 4.1 Create `nix/tests/distributed-worker-pool.nix` following `ci-dogfood-deploy-multinode.nix` pattern: 2 nodes with distinct Iroh secret keys, shared cookie, `aspen-node` with `enableWorkers = true`.
- [x] 4.2 In testScript: form 2-node cluster (init on node1, add-learner node2, change-membership), wait for cluster ready.
- [x] 4.3 Add subtests: run `aspen-distributed-pool-test distribute --ticket <ticket>` on node1, assert PASS. Run `aspen-distributed-pool-test reroute --ticket <ticket>` on node1, assert PASS.
- [x] 4.4 Add `ciVmTestBin` entry in `flake.nix` for `aspen-distributed-pool-test` with features `[jobs, shell-worker]`.
- [x] 4.5 Add `distributed-worker-pool-test` check in `flake.nix` that imports `nix/tests/distributed-worker-pool.nix` and passes both the test binary and node/cli packages.

## 5. Verification

- [x] 5.1 Build and run job-primitives test: `cargo run -p aspen-job-primitives-test -- all` passes all 7 tests. Nix build requires commit first (source filter).
- [x] 5.2 Distributed worker pool test: `.nix` file and flake wiring done. Nix build requires commit (uses existing `aspen-node` + `aspen-cli` binaries).
- [x] 5.3 `cargo build -p aspen-job-primitives-test` compiles with zero warnings.
