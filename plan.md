# Aspen Reboot Plan

We wiped the previous modules to rebuild Aspen around a clean architecture that ties `openraft` into a `ractor` cluster using `iroh`/`irpc` transport. All cluster orchestration will run inside `ractor_cluster::NodeServer` instances so actors can be remoted across hosts. The milestones below describe how we get from an empty crate to a functioning distributed KV core again.

## Phase 1: Core Building Blocks
1. **Define crate boundaries**
   - Sketch the top-level modules (`cluster`, `raft`, `storage`, `api`).
   - Add skeletal files with doc-comments describing the responsibilities and dependencies (e.g., `cluster` depends on `ractor_cluster` + `iroh::Endpoint`).
2. **Actor primitives**
   - Implement a thin wrapper around `NodeServer` that configures magic-cookie auth and exposes a typed API for launching local actors.
   - Provide helpers for encoding/decoding messages (derive `RactorClusterMessage`, prost helpers, etc.).

## Phase 2: Raft Integration
1. **Storage backend**
   - Reintroduce the `redb`-backed log/state machine from the old repo but split into `storage::log` and `storage::state_machine` modules with unit tests.
   - Run `openraft::testing::Suite::test_all` to validate the implementation in isolation.
2. **Raft actor**
   - Wrap `openraft::Raft<TypeConfig>` in a `ractor` actor that owns storage + network handles.
   - Expose RPC-oriented messages (`ClientWrite`, `ReadState`) plus lifecycle commands (`JoinCluster`, `InstallSnapshot`).

## Phase 3: Network Fabric
1. **IROH + IRPC transport**
   - Build an `irpc` service that exposes the Raft RPC surface (AppendEntries, Vote, InstallSnapshot) over `iroh` sessions.
   - Implement `RaftNetworkFactory` using the new `irpc` client and plug it into the Raft actor.
2. **External transports**
   - Demonstrate BYO transport by piping a `tokio::io::DuplexStream` through `ClusterBidiStream` for local tests.

## Phase 4: Cluster Services
1. **Bootstrap orchestration**
   - Write a `cluster::bootstrap` module that spins up a `NodeServer`, launches a Raft actor, and registers it with the cluster metadata store.
   - Provide CLI/Config parsing for node IDs, data directories, and peer endpoints.
2. **Client + API**
   - Recreate the KV API (`set`, `get`, `txn`) but go through a `ractor` client actor that forwards to the Raft actor.
   - Add smoke tests using `cargo nextest` that bring up two nodes via the new cluster harness.

## Phase 5: Documentation & Hardening
1. **Docs**
   - Update `AGENTS.md`/`docs/` with a getting-started guide for the ractor-based stack (magic cookies, transport options).
2. **Testing & Tooling**
   - Reintroduce deterministic simulations (madsim) that exercise leader election, failover, and network partitions over the new transport layer.
   - Ensure CI covers storage suites, actor unit tests, and integration runs via `cargo nextest`.
