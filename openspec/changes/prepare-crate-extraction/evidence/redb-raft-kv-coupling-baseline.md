# Redb Raft KV Coupling Baseline

This baseline captures the current Redb + OpenRaft KV layout before any storage or facade movement.

## Command evidence

- `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-redb-storage.txt`
- `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-types.txt`
- `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft.txt`
- `openspec/changes/prepare-crate-extraction/evidence/cargo-tree/aspen-raft-network.txt`

Each file was generated with `cargo tree -p <package> -e features`.

## Current module ownership

### `aspen-redb-storage`

Current reusable storage crate files:

- `crates/aspen-redb-storage/src/lib.rs`
- `crates/aspen-redb-storage/src/verified/heuristics.rs`
- `crates/aspen-redb-storage/src/verified/integrity.rs`
- `crates/aspen-redb-storage/src/verified/kv.rs`
- `crates/aspen-redb-storage/src/verified/mod.rs`
- `crates/aspen-redb-storage/tests/proptest_integrity.rs`

Current state: pure verified helpers only. `crates/aspen-redb-storage/src/lib.rs` explicitly says concrete `storage.rs`, `storage_shared/`, and `storage_validation.rs` remain in `aspen-raft` because they depend on `AppTypeConfig`, `StoredSnapshot`, and `NodeId`.

### `aspen-raft-types`

Current reusable-ish Raft type files:

- `crates/aspen-raft-types/src/constants.rs`
- `crates/aspen-raft-types/src/lib.rs`
- `crates/aspen-raft-types/src/member.rs`
- `crates/aspen-raft-types/src/network.rs`
- `crates/aspen-raft-types/src/request.rs`

Current blocker: package depends on `aspen-core` and `aspen-trust`, so it is not yet a neutral `aspen-raft-kv-types` crate.

### `aspen-raft-network`

Current iroh/IRPC adapter files:

- `crates/aspen-raft-network/src/lib.rs`
- `crates/aspen-raft-network/src/rpc/errors.rs`
- `crates/aspen-raft-network/src/rpc/messages.rs`
- `crates/aspen-raft-network/src/rpc/mod.rs`
- `crates/aspen-raft-network/src/rpc/sharding.rs`
- `crates/aspen-raft-network/src/types.rs`
- `crates/aspen-raft-network/src/verified/encoding.rs`
- `crates/aspen-raft-network/src/verified/heuristics.rs`
- `crates/aspen-raft-network/src/verified/network.rs`

Current blocker: default package depends directly on `aspen-core`, `aspen-raft-types`, `aspen-transport`, `aspen-sharding`, `aspen-time`, `iroh`, `irpc`, and `openraft`. This is the adapter layer, but its reusable-vs-runtime feature boundary still needs manifest policy.

### `aspen-raft` storage modules still inside integration crate

Concrete Redb/OpenRaft storage modules currently inside `crates/aspen-raft/src/`:

- `storage/redb_store.rs`
- `storage/mod.rs`
- `storage_shared/chain.rs`
- `storage_shared/error.rs`
- `storage_shared/index.rs`
- `storage_shared/initialization.rs`
- `storage_shared/kv.rs`
- `storage_shared/lease.rs`
- `storage_shared/log_storage.rs`
- `storage_shared/meta.rs`
- `storage_shared/sm_trait.rs`
- `storage_shared/snapshot.rs`
- `storage_shared/state_machine/batch.rs`
- `storage_shared/state_machine/cas.rs`
- `storage_shared/state_machine/delete.rs`
- `storage_shared/state_machine/dispatch.rs`
- `storage_shared/state_machine/lease_ops.rs`
- `storage_shared/state_machine/set.rs`
- `storage_shared/state_machine/transaction.rs`
- `storage_shared/trust.rs`
- `storage_shared/types.rs`
- `storage_validation.rs`
- `integrity.rs`
- storage-related verified helpers under `verified/{integrity,kv,scan,write_batcher}.rs`
- storage specs under `spec/{append,chain_hash,snapshot,storage_state,truncate}.rs`

These are the primary source files that must move or be split for `aspen-redb-storage` readiness.

## Dependency coupling baseline

### Direct dependencies from manifests

- `aspen-redb-storage` currently depends on `aspen-core`, `blake3`, `hex`, and `serde`; its manifest still comments that the future Raft storage feature will need `openraft`, `redb`, `tokio`, and `futures`.
- `aspen-raft-types` currently depends on `aspen-constants`, `aspen-core`, `aspen-trust`, vendored `openraft`, `irpc`, `serde`, and `tracing`.
- `aspen-raft-network` currently depends on `aspen-core`, `aspen-raft-types`, `aspen-transport`, `aspen-sharding`, `aspen-time`, `iroh`, `irpc`, vendored `openraft`, `tokio`, `async-trait`, `postcard`, `serde`, `anyhow`, `parking_lot`, `rand`, and `tracing`.
- `aspen-raft` currently depends on app/runtime concerns including `aspen-core-shell`, `aspen-auth`, `aspen-transport`, `aspen-sharding`, `aspen-client-api`, optional `aspen-coordination`, optional `aspen-sql`, optional `aspen-trust`, optional `aspen-secrets`, concrete `iroh`, `iroh-base`, `irpc`, `tokio`, `redb`, vendored `openraft`, and storage/type crates.

### Current transitive and re-export leak paths to verify later

The captured `cargo tree` evidence shows these current leak paths that the readiness checker must reject or require explicit feature/adapter ownership for reusable defaults:

- `aspen-redb-storage -> aspen-core -> aspen-cluster-types` appears in `cargo-tree/aspen-redb-storage.txt`, so the storage crate is not yet proven independent from the broader core graph.
- `aspen-raft-types -> aspen-core -> aspen-cluster-types` and `aspen-raft-types -> aspen-trust` appear in `cargo-tree/aspen-raft-types.txt`, which blocks a neutral `aspen-raft-kv-types` claim.
- `aspen-raft-network -> aspen-transport -> aspen-auth/aspen-core-shell/iroh/openraft/aspen-sharding` appears in `cargo-tree/aspen-raft-network.txt`; this is acceptable only if `aspen-raft-network` remains the explicit runtime adapter layer.
- `aspen-raft -> aspen-core-shell`, `aspen-raft -> aspen-auth`, `aspen-raft -> aspen-client-api`, `aspen-raft -> aspen-transport`, `aspen-raft -> iroh`, `aspen-raft -> redb`, and optional `aspen-raft -> aspen-trust/aspen-secrets/aspen-sql/aspen-coordination` appear in `cargo-tree/aspen-raft.txt`; these concerns must stay in compatibility/integration layers or named opt-in features.

## Current storage safety rails

Current evidence before movement:

- `crates/aspen-raft/src/storage_shared/mod.rs` documents the single-fsync architecture: log entry insert, state mutation, metadata update, and `txn.commit()` happen in the same Redb transaction; `RaftStateMachine::apply()` is a no-op because state was already applied during append.
- `crates/aspen-raft/src/spec/append.rs` models Redb append crash cases and states that redb rollback before commit plus full durability after commit provide the atomicity proof.
- `crates/aspen-raft/src/spec/truncate.rs` models truncation atomicity and chain-tip synchronization.
- `crates/aspen-raft/src/spec/snapshot.rs` models snapshot integrity, chain hash at snapshot point, and snapshot installation invariants.
- `crates/aspen-raft/src/integrity.rs` provides the background chain verifier and full-chain verification path over `RedbLogStore`.
- `crates/aspen-raft/src/storage/mod.rs` documents existing Redb log persistence, vote/committed-index persistence, chain integrity, truncation, purge, and in-memory state-machine test coverage.

## Baseline conclusion

The Redb Raft KV slice is close to reusable but not extraction-ready:

- pure storage helpers already live in `aspen-redb-storage`;
- concrete Redb/OpenRaft storage still lives in `aspen-raft`;
- `aspen-raft-types` still carries Aspen core/trust coupling;
- `aspen-raft-network` is already an adapter-shaped crate but must be documented as such;
- `aspen-raft` remains the compatibility/integration shell and pulls the full Aspen runtime graph.
