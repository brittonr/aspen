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
   - Next action: implement the real external Raft/DB bridge behind those traits (replacing the in-crate in-memory stub), then capture a deterministic `madsim` scenario that exercises the same HTTP workflow so CI covers it without relying on best-effort shell scripts.

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
