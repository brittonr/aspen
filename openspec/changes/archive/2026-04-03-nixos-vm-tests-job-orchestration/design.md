## Context

Job orchestration has 22K lines across subsystems (dependency DAGs, distributed pools, scheduling, DLQ, saga, workflows, affinity, replay) with 229 unit/integration tests in `cargo nextest`. But NixOS VM tests only cover the CI pipeline path and a bundled durable-workflow test. The lower-level primitives lack isolated VM tests that exercise real async runtimes, real disk I/O, and real process boundaries.

Existing patterns to follow:

- **`aspen-durable-workflow-test`**: standalone test binary using `DeterministicKeyValueStore`, subcommand per scenario, prints `PASS`/`FAIL`, called from a `.nix` test file
- **`job-index.nix`**: spins up a real single-node cluster, uses `aspen-cli` to exercise job CRUD over Iroh RPC
- **`ci-dogfood-deploy-multinode.nix`**: 3-node cluster pattern for distributed scenarios

## Goals / Non-Goals

**Goals:**

- VM-level integration tests for each untested job orchestration subsystem
- Follow the established test binary pattern (`ciVmTestBin` in flake, `.nix` wrapper in `nix/tests/`)
- Each subsystem testable in isolation without the full CI pipeline
- Tests run in under 2 minutes each (single-node tests) or 5 minutes (multi-node)

**Non-Goals:**

- Testing the CI pipeline orchestrator (already covered by 10+ dogfood tests)
- Modifying production job code
- WASM/Hyperlight plugin executor testing (requires separate plugin build infrastructure)
- Job analytics dashboard testing (read-only aggregation, low risk)

## Decisions

### 1. Two test binaries, not eight

Create two new test binary crates:

- **`aspen-job-primitives-test`**: exercises dependency tracker, scheduler, DLQ, workflow engine, affinity, and replay against `DeterministicKeyValueStore` (no cluster needed)
- **`aspen-distributed-pool-test`**: exercises distributed worker pool routing, requires a live multi-node cluster

**Rationale**: The durable-workflow-test pattern works well — a single binary with subcommands keeps the flake manageable. Most subsystems work against the in-memory store. Only distributed pool routing genuinely needs multiple nodes.

**Alternative rejected**: One binary per subsystem. Would add 8 entries to `ciVmTestBin` in `flake.nix` and 8 `.nix` files. The overhead isn't worth it for tests that share the same store setup.

### 2. Two `.nix` test files

- **`nix/tests/job-primitives.nix`**: single VM, runs `aspen-job-primitives-test all` with subtests per subsystem
- **`nix/tests/distributed-worker-pool.nix`**: 2-node cluster, runs `aspen-distributed-pool-test` which submits jobs with affinity and verifies cross-node routing

**Rationale**: Matches the split between tests that need a cluster and those that don't. The single-VM test is fast (~60s). The multi-node test is slower but covers the only subsystem that genuinely requires multiple nodes.

### 3. DeterministicKeyValueStore for most tests

Dependency tracker, scheduler, DLQ, workflow engine, and replay can all be tested against the in-memory deterministic store. This avoids cluster setup overhead and makes failures easier to reproduce.

The distributed pool test needs a real cluster because it exercises Iroh-based job routing between nodes.

### 4. Saga tests stay in durable-workflow-failover

The existing `durable-workflow-failover.nix` already covers saga compensation in LIFO order. No need to duplicate. The new primitives test covers the standalone `SagaExecutor` API separately (builder pattern, multi-step with partial failure, compensation ordering).

### 5. Scheduler tests use tokio time rather than wall clock

Cron and delayed scheduling tests advance time via `tokio::time::pause()` + `tokio::time::advance()` instead of sleeping for real wall-clock seconds. Keeps the VM test fast.

## Risks / Trade-offs

**[Risk] DeterministicKeyValueStore doesn't catch async timing bugs** → These subsystems have unit tests for the logic; the VM test confirms the binary runs correctly in a NixOS environment with real process isolation. Multi-node timing bugs are caught by the distributed pool test.

**[Risk] Multi-node test is flaky due to Raft election timing** → Use the same deterministic secret keys and `wait_for_cluster_ready` pattern from `ci-dogfood-deploy-multinode.nix`. Cap retry loops at 60s.

**[Risk] Adding 2 test binaries increases CI build time** → Both binaries are small (~500-800 lines each). The `ciVmTestBin` builder shares the workspace artifact cache. Marginal build cost.
