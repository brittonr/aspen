# V5 Test Evidence

Generated: 2026-04-25T02:25Z

## aspen-redb-storage tests (default + raft-storage)

```
cargo nextest run -p aspen-redb-storage --features raft-storage
Summary: 84 tests run: 84 passed, 0 skipped
```

Covers: CAS validation, chain integrity, snapshot integrity, KV versioning,
lease construction/expiration, property-based tests for all verified functions.

## aspen-raft storage tests (compatibility path)

```
cargo nextest run -p aspen-raft -E 'test(storage)'
Summary: 110 tests run: 110 passed, 602 skipped
```

Covers: SharedRedbStorage (original), storage validation, chain integrity,
snapshot serialization.

## Downstream consumer fixture

```
cargo check --manifest-path tests/fixtures/downstream-redb-raft-kv/Cargo.toml
Finished (no errors)
```

Proves canonical APIs (`aspen_raft_kv_types::*`, `aspen_redb_storage::*`,
`aspen_raft_kv::*`) work without `aspen` package dependency.

## Negative test: app-only APIs unavailable

The downstream fixture does NOT depend on `aspen`, `aspen-raft`, `aspen-cluster`,
or any app-only crate. Attempting to use trust/secrets/SQL/coordination APIs
would fail at compile time. The fixture's `Cargo.toml` has only:
- aspen-kv-types
- aspen-raft-kv-types
- aspen-redb-storage (features = ["raft-storage"])
- aspen-raft-kv
