# Aspen Reboot Plan

We wiped the previous modules to rebuild Aspen around a clean architecture that ties `openraft` into a `ractor` cluster using `iroh`/`irpc` transport. All cluster orchestration will run inside `ractor_cluster::NodeServer` instances so actors can be remoted across hosts. The milestones below describe how we get from an empty crate to a functioning distributed KV core again.

## Phase 1: Core Building Blocks
1. **Define crate boundaries** ✅
   - Top-level modules (`cluster`, `raft`, `storage`, `api`) now exist with Tiger Style doc-comments that describe responsibilities, dependencies, and how they compose.
   - Next action: start threading these modules together via a bootstrap binary once the Raft actor exists (Phase 2).
2. **Actor primitives** ✅
   - Introduced a typed wrapper around `NodeServer` plus helpers for attaching BYO transports and deriving `RactorClusterMessage`.
   - Next action: plug the upcoming Raft actor + storage bindings into `NodeServerHandle` so deterministic sims can drive message flow.

## Phase 2: Raft Integration
1. **Storage backend** (in progress)
   - Created `storage::log` + `storage::state_machine` modules with deterministic in-memory backends and proptest seams so we can validate ordering/snapshot invariants.
   - Added `redb`-backed implementations plus `StoragePlan` wiring so nodes can flip between deterministic and persistent surfaces.
   - Next action: **defer** running the OpenRaft storage suite until the external Raft/DB plan solidifies. Aspen will keep using the in-memory handles via the HTTP façade while the real log/state machine live in a sibling service.
2. **Raft actor** (in progress)
   - Added a placeholder Raft actor/factory that already wires into the NodeServer handle + `StorageSurface`, keeping the transport seams deterministic for `madsim`.
   - Added the `aspen-node` bootstrap binary plus HTTP endpoints (`/health`, `/metrics`, `/init`, `/add-learner`, `/change-membership`, `/write`, `/read`) so we can drive multi-node scripts similar to the OpenRaft example.
   - Added `scripts/aspen-cluster-smoke.sh` and `docs/cluster-smoke.md` so we can spin up five nodes locally and hit the HTTP API the same way the upstream OpenRaft script does; the handlers now depend on trait-based control-plane/key-value interfaces and can proxy to an external Raft service.
   - Added an Iroh transport option (enable via CLI, deterministic keys + endpoint files) so cluster traffic can ride QUIC/relay links in addition to TCP, plus hardened the smoke test + node startup waits so the HTTP layer comes up even if Iroh peers are slow to announce.
   - Added a `netwatch` devshell tool (built from `n0-computer/net-tools`) so we can observe interface churn and confirm which adapters carry Iroh traffic during local testing.
   - Control backend now supports Hiqlite via `--control-backend hiqlite`, wiring `/write`/`/read` into a replicated SQLite table through the upstream client, reflecting live membership from the Hiqlite metrics surface, and forwarding `/add-learner`/`/change-membership` to the real cluster management APIs (expecting `raft_addr` metadata per node).
   - `scripts/run-hiqlite-cluster.sh` provisions a 3-node local Hiqlite cluster (build + configs + lifecycle) and we validate the entire flow by running it alongside `scripts/aspen-cluster-smoke.sh` to ensure the default control-plane smoke test still passes.
   - `scripts/aspen-hiqlite-smoke.sh` now orchestrates the local Hiqlite cluster and reuses the smoke harness with `--control-backend hiqlite`, while `scripts/aspen-cluster-smoke.sh` forwards arbitrary CLI args so additional backends can be validated without editing the script.
   - Added `tests/hiqlite_flow.rs`, a deterministic `madsim` scenario that replays the Hiqlite membership/write flow (including learner promotion) and asserts that the Iroh transport exports healthy counters via the new `/iroh-metrics` endpoint wiring.
   - Extended the deterministic scenario with explicit leader churn and delay injection so we capture failover traces alongside the transport metrics snapshot.
   - OpenRaft is now wired back in: the Raft actor owns a real `openraft::Raft` handle, uses the new in-memory log/state-machine storage, exposes the HTTP Raft RPC routes, and the control-plane writes/reads call straight into OpenRaft.
   - Created `src/simulation.rs` module with `SimulationArtifact` and `SimulationArtifactBuilder` for capturing deterministic simulation data (seeds, event traces, metrics, duration, status). ✅
   - Updated `tests/hiqlite_flow.rs` to persist simulation artifacts to `docs/simulations/` as JSON files with full event trace and metrics snapshots. ✅
   - Added `docs/simulations/README.md` documenting artifact format, usage, and purpose. ✅
   - Integrated artifact collection into `flake.nix` nextest check with `postInstall` hook that copies artifacts to build output for CI visibility. ✅
   - Next action: move to Phase 3 (Network Fabric) to build the real IROH + IRPC transport layer.

## Phase 3: Network Fabric ✅
1. **IROH + IRPC transport** ✅
   - ✅ Created `src/raft/rpc.rs` with serializable type definitions (`RaftVoteRequest`, `RaftAppendEntriesRequest`, `RaftSnapshotRequest`)
   - ✅ Implemented real `IrohEndpointManager` in `src/cluster/mod.rs` with configuration, peer management, and lifecycle
   - ✅ Defined IRPC service protocol `RaftRpcProtocol` with `#[rpc_requests]` macro + `RaftRpcResponse` for wire protocol
   - ✅ Built `IrpcRaftNetworkFactory` and `IrpcRaftNetwork<C>` implementing `RaftNetworkV2` trait in `src/raft/network.rs`
   - ✅ Created IRPC server in `src/raft/server.rs` that processes RPCs via `raft_core.*` methods
   - ✅ Wired IRPC server into `aspen-node.rs`, removed HTTP Raft RPC routes (HTTP control-plane remains)
   - ✅ Upgraded to OpenRaft v2 network API (`RaftNetworkV2`) for better snapshot streaming control
   - ✅ Removed HTTP Raft RPC routes (/raft/vote, /raft/append, /raft/snapshot)
   - ✅ Updated to Iroh 0.95 API (NodeAddr → EndpointAddr, method renames)
   - ✅ Fixed IRPC 0.11.0 usage (request-response pattern, proper serialization)
   - ⚠️  Peer discovery via CLI `--peers` not implemented (EndpointAddr doesn't implement FromStr)
   - ⚠️  Deferred: Deterministic simulation with real Iroh transport (incompatible with madsim)
   - ✅ Cleaned up HTTP network types (`HttpRaftNetworkFactory`, `HttpRaftNetwork`)
   - ✅ Verified smoke tests work with IRPC transport
   - Next action: Move to Phase 4 (Cluster Services) or implement peer address construction from CLI
2. **External transports**
   - Demonstrate BYO transport by piping a `tokio::io::DuplexStream` through `ClusterBidiStream` for local tests.

## Phase 4: Cluster Services
1. **Bootstrap orchestration** ✅
   - ✅ Created `src/cluster/config.rs` with layered config loading (env vars < TOML < CLI args)
   - ✅ Created `src/cluster/metadata.rs` with redb-backed persistent metadata store
   - ✅ Created `src/cluster/bootstrap.rs` orchestrating full node startup and graceful shutdown
   - ✅ Refactored `aspen-node.rs` from ~400 lines to 325 lines using bootstrap module
   - ✅ Updated smoke test scripts to work with new bootstrap
   - ✅ Added 5 comprehensive integration tests in `tests/bootstrap_test.rs`
   - Next action: Move to Phase 4.2 (Client + API)
2. **Client + API** ✅
   - ✅ Created `KvClient` in `src/kv/client.rs` that forwards KV operations to RaftActor
   - ✅ Wired KvClient into HTTP layer with clean separation from cluster control
   - ✅ Added `SetMulti` command for atomic multi-key writes
   - ✅ Created `tests/kv_client_test.rs` with 4 passing integration tests (2 skipped pending IRPC peer discovery)
   - ✅ Verified smoke tests pass with new implementation
   - Next action: Move to Phase 5 (Documentation & Hardening) or implement additional KV features

## Phase 5: Documentation & Hardening
1. **Docs**
   - Update `AGENTS.md`/`docs/` with a getting-started guide for the ractor-based stack (magic cookies, transport options).
2. **Testing & Tooling**
   - Reintroduce deterministic simulations (madsim) that exercise leader election, failover, and network partitions over the new transport layer.
   - Ensure CI covers storage suites, actor unit tests, and integration runs via `cargo nextest`.
