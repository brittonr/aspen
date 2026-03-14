# 3. Redb Unified Raft Log + State Machine Storage

**Status:** accepted

## Context

Raft implementations typically separate the log store (append-only entries) from the state machine store (applied state). This means two storage backends, two fsync paths, and two failure modes. Common choices include RocksDB, sled, or SQLite for each.

Redb is an embedded key-value store written in Rust with ACID transactions, MVCC, and a single-file storage model. It supports multiple named tables within a single database file.

## Decision

Use a single redb database for both the Raft log and the state machine. The `aspen-redb-storage` crate implements both openraft's `RaftLogStorage` and `RaftStateMachine` traits against the same redb file.

Log entries and state machine data live in separate redb tables but share a single write transaction when applying entries. This gives single-fsync writes — one disk sync commits both the log advancement and the state application atomically.

Alternatives considered:

- (+) RocksDB: battle-tested, used by TiKV and CockroachDB
- (-) RocksDB: C++ dependency, complex tuning, two-store model still needs coordination
- (+) sled: pure Rust, embedded
- (-) sled: unstable API, uncertain maintenance status
- (+) SQLite: universal, well-understood
- (-) SQLite: C dependency, WAL mode adds complexity for concurrent Raft access
- (+) Redb: pure Rust, ACID, single-file, MVCC, no C dependencies
- (~) Redb: younger project, smaller community than RocksDB

## Consequences

- Write latency of ~2-3ms per Raft apply (single fsync)
- No split-brain between log and state machine — atomic commits prevent inconsistency
- Pure Rust dependency chain — no C/C++ build tooling required
- Single file simplifies backup (copy one file) and recovery
- Verified functions in `aspen-redb-storage/src/verified/` can reason about storage invariants
- Redb's MVCC allows concurrent reads during writes without blocking
