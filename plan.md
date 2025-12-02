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
   - ✅ **Runtime peer discovery implemented** - network factory now supports `add_peer()` for dynamic peer addition
   - ✅ **ALPN configuration added** - Iroh endpoint configured with "raft-rpc" ALPN for protocol negotiation
   - ✅ **Network factory exposed in BootstrapHandle** - tests can now exchange peer addresses at runtime
   - ⚠️  Peer discovery via CLI `--peers` deferred (EndpointAddr construction requires manual building)
   - ⚠️  Deferred: Deterministic simulation with real Iroh transport (incompatible with madsim)
   - ✅ Cleaned up HTTP network types (`HttpRaftNetworkFactory`, `HttpRaftNetwork`)
   - ✅ Verified smoke tests work with IRPC transport
   - ⏸️  Multi-node integration tests updated but require Raft initialization pattern debugging
   - Next action: Debug multi-node Raft init sequence OR implement HTTP peer exchange endpoints
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
1. **Testing Foundation** ✅
   - ✅ Created `src/raft/storage.rs` test module with `StoreBuilder` implementation
   - ✅ Integrated OpenRaft's `Suite::test_all()` - validates 50+ storage scenarios (log, snapshots, membership)
   - ✅ Fixed `install_snapshot()` to properly persist snapshots for `get_current_snapshot()`
   - ✅ All storage tests passing - production-ready validation
   - ✅ Created `src/testing/router.rs` - AspenRouter for deterministic multi-node Raft tests
   - ✅ Implemented in-memory network with configurable delays/failures, wait helpers via OpenRaft's `Wait` API
   - ✅ Added 3 passing unit tests demonstrating router capabilities
   - ✅ **Ported 12 OpenRaft tests across 6 test files** (initialization, election, membership, restart, partition, client writes)
   - ✅ All tests passing except 1 skipped (add_learner multi-node issue requiring investigation)
   - ✅ Validated: cluster init, election logic, client writes, leader recovery, network simulation, membership changes
   - ✅ **FIXED**: Multi-node `add_learner` issue resolved! Root cause was `InMemoryNetworkFactory::new_client` using wrong target parameter
   - ✅ Network factory was sending RPCs to self.target instead of the parameter target, causing nodes to send to themselves
   - ✅ All 13 router tests now passing (previously 1 skipped due to this bug)
   - Next action: Continue Phase 5.2 (porting Priority 1-4 tests)
2. **Deterministic Simulations** (in progress)
   - ✅ **Phase 5.2 Analysis Complete**: Used 4 parallel exploration agents to comprehensively analyze OpenRaft test suite
     - Catalogued 93 OpenRaft tests across 12 categories (membership, client_api, snapshot_streaming, append_entries, etc.)
     - Identified Priority 1-4 tests for porting based on foundational importance
     - Documented test patterns and porting requirements
     - Coverage gaps identified: append_entries (1/12), replication (0/7), snapshot_streaming (0/13)
   - ✅ **Test Infrastructure Enhancements**:
     - Added `remove_node()` - Extract node for direct Raft API testing
     - Added `initialize(node_id)` - Single-node cluster initialization
     - Added `add_learner(leader, target)` - Learner node management
     - Added `external_request<F>(target, callback)` - Internal Raft state inspection
     - Added `new_cluster(voters, learners)` - Complete multi-node cluster setup helper
   - ✅ **New Tests Ported** (2 tests, both passing):
     - `router_t11_append_conflicts.rs` - Comprehensive append-entries conflict resolution
       - Validates all 5 conflict scenarios (empty logs, missing prev_log_id, inconsistent entries, etc.)
       - Tests direct append_entries API without network layer
       - Critical for Raft log consistency guarantees
     - `router_t61_heartbeat_reject_vote.rs` - Leader lease mechanism
       - Validates followers reject vote requests while receiving heartbeats
       - Tests leader lease expiration and vote timing
       - Critical for preventing unnecessary elections
   - ✅ **Test Statistics**: **22/23 router tests passing (95.7% pass rate)**
   - ✅ **Phase 5 Critical Tests Ported** (6 tests, 5 passing):
     - ✅ `router_t62_follower_clear_restart_recover.rs` - Recovery from complete state loss on restart (PASSING)
     - ✅ `router_t10_see_higher_vote.rs` - Election safety when leader sees higher vote (PASSING)
     - ✅ `router_t50_snapshot_when_lacking_log.rs` - Automatic snapshot streaming when follower lacks logs (PASSING)
     - ✅ `router_t50_append_entries_backoff_rejoin.rs` - Replication recovery after network partition (PASSING)
     - ✅ `router_t11_append_inconsistent_log.rs` - Large log conflict resolution (>50 entries) (PASSING)
     - ⚠️  `router_t10_conflict_with_empty_entries.rs` - Conflict detection with empty append-entries (overly complex ported version, needs simplification)
   - ✅ **Runtime Issues Fixed** (5 critical bugs identified and resolved via parallel agent investigation):
     1. **Vote progression bug**: Section 4 was bumping term to 2, preventing section 5 from using term 1
     2. **Leader stepdown race**: Added wait for stepdown before verifying write failures
     3. **Snapshot index off-by-one**: Expected index 20, actual is 19 (last committed log)
     4. **Leader lease timeout**: Missing 1-second sleep before election to allow old leader's lease to expire
     5. **Blank leader log**: Multiple tests not accounting for blank log entry added when leader is elected
   - ✅ **Compilation Issues Fixed**: All type mismatches and RaftMetrics field access issues resolved
     - Fixed 8 `CommittedLeaderId` type mismatches by using `log_id::<AppTypeConfig>()` helper
     - Fixed 3 `metrics.applied_index` field access errors by using Wait API or removing redundant checks
     - Replaced direct storage manipulation with `append_entries` RPCs for proper vote handling
   - 📝 **Key Learnings**:
     - Direct storage manipulation after Raft initialization causes in-memory/storage sync issues; vote updates must go through Raft protocol
     - Leader elections add blank log entries that tests must account for in applied_index expectations
     - Leader leases prevent elections; must wait for expiration before triggering new elections
     - Snapshot creation happens at last_committed_index, not at the trigger threshold index
   - 📝 **Documentation Created**: `.claude/phase5_test_migration.md` with detailed migration decisions and patterns
   - **Deferred** (need additional AspenRouter features):
     - `t10_append_entries_partial_success` - Requires quota simulation + Clone trait
     - `t50_append_entries_backoff` - Requires RPC counting infrastructure
3. **Documentation** (pending)
   - Update `AGENTS.md` with getting-started guide (ractor, Iroh, IRPC, OpenRaft integration)
   - Create `docs/getting-started.md` for single-node & 3-node quickstarts
   - Add Architecture Decision Records (ADRs) for key technology choices
4. **CI Enhancements** (pending)
   - Add storage suite to CI pipeline
   - Run madsim tests with multiple seeds
   - Include smoke tests in CI
   - Upload simulation artifacts on failure
   - Add code coverage reporting
