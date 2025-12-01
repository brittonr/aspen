# Project Aspen - KV Module Implementation Plan

## Current Progress:

-   **Module rebuild:** Re-created the `src/kv` module with fresh `api`, `error`, `network`, `node`, `store`, and `types` files to give the project a clean, compiling baseline.
-   **`kv/types.rs`:** Declares the OpenRaft `TypeConfig`, plus the `Request`/`Response` enums used by the state machine.
-   **`kv/store.rs`:** Implements `KvLogStore` and `KvStateMachine` backed by `redb`, completes `RaftLogStorage`, `RaftLogReader`, `RaftStateMachine`, and `RaftSnapshotBuilder`, and exposes a simple unit test to keep serialization honest.
-   **`kv/api.rs`:** Provides helper functions for `set`, `delete`, and linearizable `get` operations that wire directly into OpenRaft.
-   **`kv/node.rs`:** Introduces `KvNode::start` to bootstrap a Raft instance with the new storage and a placeholder `KvNetworkFactory`.
-   **`kv/network.rs`:** Implements an actual iroh-backed transport with a Router-based server, framed request/response encoding, and peer registration utilities so Raft RPCs can flow between nodes.
-   **Dependencies:** Added OpenRaft, `redb`, `snafu`, `irpc`, and the testing crates back into `Cargo.toml` so the module can build in isolation once dependencies are fetched.
-   **Property-based tests:** Added `proptest` suites for `KvStateMachine::apply`, covering mixed workloads plus focused idempotent-set and delete semantics to protect the core invariants.
-   **Deterministic cluster harness:** Introduced `tests/kv_cluster.rs` with a `mad-turmoil` seeded router that exercises leader election, replication, and partition healing across three-node clusters.
-   **Txn/lease/watch logic:** Extended `kv/types.rs` and the state machine so Raft entries can encode multi-op transactions, lease lifecycle management, and watch registrations with bounded event queues, plus unit tests recognizing the new invariants.
-   **Crate integration:** Added a crate-level `error` module that wraps `kv::error::Error`, re-exported `KvNode`/`KvClient` from `lib.rs`, and converted `KvNode::start` to surface the unified `aspen::Error` so other modules can rely on a single result type.
-   **Service harness:** Implemented `KvServiceBuilder`/`KvService` to translate config inputs into `KvNode::start`, auto-register peers, and hand back a `KvClient`, plus an `examples/kv_service.rs` entry point that uses env vars to spin up a node.
-   **Harness adoption:** Added `tests/kv_service_smoke.rs` to boot a single node via the builder and exercise CRUD + txn flows, and documented the `ASPEN_KV_*` env contract in `docs/kv-service.md` for operator scripts.
-   **Cluster bootstrap experiments:** Updated `tests/kv_cluster.rs` to follow the OpenRaft docs flow (single-node init ➜ `add_learner` for each follower ➜ `change_membership`). The new sequence still times out because the second learner never finishes replicating; next work focuses on fixing the test network / replication so `cargo nextest run kv_cluster_` passes.

## Next Steps:

1.  **Stabilize deterministic cluster tests:**
    *   Investigate why the second learner never catches up (inspect `TestRouter`/`TestNetwork` wiring, ensure replication RPCs reach the learner, and add instrumentation around `add_learner`/`change_membership`).
    *   Once replication succeeds, rerun `cargo nextest run kv_cluster_`, integrate the service harness into the cluster tests, and document a three-node walkthrough in `docs/kv-service.md`.
