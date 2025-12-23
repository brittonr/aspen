# Aspen Layer Architecture Analysis

**Date**: 2025-12-22
**Status**: Research Document (no implementation planned)
**Question**: Should Aspen add a layer on top of its KV/state machine like FoundationDB does?

## Executive Summary

**Answer: YES, when the need arises, with a targeted, minimal approach.**

The FoundationDB layer pattern is well-suited for Aspen. When future-proofing or complex query needs materialize, implement three specific abstractions:

1. **Tuple Encoding** - Type-safe, order-preserving key encoding
2. **Subspaces** - Namespace isolation for multi-tenancy
3. **Secondary Indexes** - Client-managed indexes with transactional guarantees

**Do NOT implement:** Directories, Transaction DSL, Full SQL engine, or Automatic index maintenance.

**Current Decision**: No implementation now. Revisit when concrete use cases emerge.

---

## Research Findings

### FoundationDB Pattern

- **Core**: Minimal ordered KV with strict serializable transactions
- **Layers**: Stateless modules implementing new data models (Record Layer, Document Layer)
- **Key abstractions**: Tuples (ordered encoding), Subspaces (namespaces), Directories (path indirection)
- **Philosophy**: "What features can we take away?" - Keep core simple, let layers add complexity

### Other Systems

| System | Approach | Key Insight |
| ------ | -------- | ----------- |
| **TiKV/TiDB** | Region-based KV + SQL layer | Coprocessor pushdown for performance |
| **CockroachDB** | Unified sorted KV + SQL | Range abstraction hidden from SQL layer |
| **etcd** | KV + Watch + Leases | Coordination primitives built on CAS |
| **Consul** | Minimal KV + Sessions | Let users build abstractions |

### Key Insight
>
> "Once you have a strict serializable key-value store, you can layer a SQL engine and secondary indexes on top. A strict serializable KV store is the foundation upon which you can build distributed databases almost however you want."

---

## Current Aspen State

### What Aspen Already Has

- `KeyValueStore` trait: write, read, delete, scan
- Optimistic transactions (OCC) via `TransactionBuilder`
- etcd-style transactions (If/Then/Else)
- Compare-and-Swap operations
- Revision tracking (version, create_revision, mod_revision)
- Watch API for real-time subscriptions
- Leases and TTL expiration
- Coordination primitives (locks, elections, counters, queues)
- System prefix protection (`_system:` in `src/api/vault.rs`)

### What's Missing

1. **Ordered tuple encoding** - Keys sort as strings, breaking integer ordering
2. **Formal subspaces** - No namespace isolation beyond `_system:` prefix
3. **Secondary indexes** - No transactional index maintenance

---

## Recommended Abstractions

### 1. Tuple Encoding Layer

**Purpose**: Type-safe, order-preserving key encoding for composite keys.

**Problem Solved**: String keys don't preserve numeric order ("user:10" < "user:9").

**API Sketch**:

```rust
// Composite key with proper ordering
let key = Tuple::new()
    .push("tenant-123")   // String
    .push("users")        // String
    .push(42i64)          // Integer - preserves numeric order!
    .pack()?;             // -> Vec<u8>

// Range scan for all users in a tenant
let (start, end) = Tuple::new()
    .push("tenant-123")
    .push("users")
    .range();
```

**Location**: `src/layer/tuple.rs`

**Complexity**: ~500 LOC, follows FoundationDB tuple spec

### 2. Subspace Layer

**Purpose**: Namespace isolation and multi-tenancy.

**Problem Solved**: Formalizes the informal `_system:` pattern.

**API Sketch**:

```rust
// Create isolated namespaces
let tenant_a = Subspace::new(Tuple::new().push("tenant-a"))?;
let tenant_b = Subspace::new(Tuple::new().push("tenant-b"))?;

// Nested subspaces
let users = tenant_a.subspace(Tuple::new().push("users"))?;

// Keys are automatically prefixed
let key = users.pack(&Tuple::new().push(user_id))?;

// Watch all changes in a tenant
let (start, _) = tenant_a.range();
session.subscribe(start, 0).await?;
```

**Location**: `src/layer/subspace.rs`

**Complexity**: ~200 LOC

### 3. Secondary Index Layer

**Purpose**: Client-managed indexes with OCC guarantees.

**Problem Solved**: Finding entities by non-primary-key attributes requires full scans.

**API Sketch**:

```rust
// Define an index
let email_index = IndexDefinition::new("user_by_email", indexes_space, /*unique=*/true);

// Indexed write bundles primary + index updates atomically
let txn = IndexedWrite::new(users_space, user_id)
    .set_value(user_json)
    .add_index(email_index, Tuple::new().push("alice@example.com"))
    .build()?;

// Query by index
let (start, end) = email_index.scan_range(&Tuple::new().push("alice@example.com"))?;
```

**Location**: `src/layer/index.rs`

**Complexity**: ~400 LOC

---

## What NOT to Implement

| Abstraction | Reason |
| ----------- | ------ |
| **Directory Layer** | Path->prefix indirection adds complexity; subspaces provide 90% of value |
| **Transaction DSL** | Existing `TransactionBuilder` in `src/client/transaction.rs` is adequate |
| **Full SQL Engine** | Out of scope; read-only SQL queries via `SqlQueryExecutor` are sufficient |
| **Auto Index Maintenance** | Requires schema in state machine; client-managed indexes are more flexible |

---

## Future Implementation Notes

When concrete use cases emerge (complex queries, multi-tenancy needs), the implementation order should be:

1. **Phase 1: Tuple Encoding** (~500 LOC)
   - Foundation for everything else
   - `src/layer/tuple.rs`
   - Tiger Style: MAX_TUPLE_SIZE = 8KB, MAX_TUPLE_DEPTH = 16

2. **Phase 2: Subspaces** (~200 LOC)
   - Depends on Tuples
   - `src/layer/subspace.rs`
   - Migrate `_system:` to formal Subspace

3. **Phase 3: Secondary Indexes** (~400 LOC)
   - Depends on Tuples + Subspaces
   - `src/layer/index.rs`
   - Build on existing `TransactionBuilder`

---

## Reference Files (for future implementation)

- `src/client/transaction.rs` - Pattern for fluent builders (excellent model)
- `src/api/mod.rs` - KeyValueStore trait to build upon
- `src/api/vault.rs` - Existing `_system:` prefix pattern
- `src/raft/storage_sqlite.rs` - Key storage format

---

## When to Revisit This Decision

Consider implementing layers when:

- A Blixard service needs multi-tenant data isolation
- Query patterns emerge that require secondary indexes
- Integer/UUID ordering becomes a pain point
- Performance profiling shows full-scan queries as bottlenecks

---

## Resources

- [FoundationDB Layer Concept](https://apple.github.io/foundationdb/layer-concept.html)
- [FoundationDB Record Layer Paper](https://www.foundationdb.org/files/record-layer-paper.pdf)
- [TiKV Architecture](https://docs.pingcap.com/tidb/stable/tikv-overview/)
- [binary_tuples crate](https://github.com/Myrannas/binary-tuples) - Potential Rust implementation
