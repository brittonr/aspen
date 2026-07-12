## Context

Existing persistence is attached to particular Aspen subsystems. System extensions need reusable local persistence primitives while retaining adapter substitution for simulation and preventing accidental claims that a local commit implies replication or consensus.

## Decisions

### 1. Durable state is a family of narrow ports

**Choice:** Define separate versioned contracts for an append-oriented durable log, an ordered key/value store, immutable snapshot objects, and bounded checkpoints. A profile may implement several contracts, but extensions request only the operations they need.

**Rationale:** One universal storage API either leaks a backend or becomes too weak for logs, databases, schedulers, and object stores.

### 2. Atomicity and durability domains are explicit

**Choice:** Every operation declares its adapter, namespace, generation, atomicity domain, durability level, preconditions, resource charge, and completion semantics. Atomic batches cannot silently span adapters, namespaces, or unsupported object classes.

**Rationale:** The dangerous failure mode is treating a convenient batch API as a distributed or cross-engine transaction.

### 3. Durability completion is adapter-acknowledged

**Choice:** The shell reports durable completion only after the selected adapter reaches its declared flush boundary. Buffered, scheduled, durable, failed, cancelled, and uncertain outcomes are distinct.

**Rationale:** An accepted write is not necessarily stable across process or machine loss.

### 4. Effect transactions generalize reserve, commit, abort, and reconcile

**Choice:** Selected shell effects may expose prepare or reserve, commit, abort, inspect, and reconcile operations under a canonical effect-transaction id. The descriptor states whether reservation is durable, expiring, exclusive, idempotent, or compensating.

**Rationale:** This supports bounded coordination across extension lifecycle and side effects without pretending to offer universal two-phase commit.

### 5. Recovery starts from canonical inventories

**Choice:** On activation or restart, an extension can inventory log tails, store generations, snapshots, checkpoints, and unresolved effect transactions before accepting new work. Corruption, gaps, incompatible schema, and uncertain transactions deny automatic recovery unless an explicit policy selects repair or quarantine.

**Rationale:** Recovery must be deterministic and evidence-backed rather than inferred from process memory.

### 6. Simulation models persistence boundaries, not just an in-memory map

**Choice:** The deterministic adapter models buffered versus durable state, crashes at declared boundaries, partial external effects, latency, capacity, and injected corruption classes. It emits the same port outcomes as live adapters.

**Rationale:** An in-memory happy-path mock cannot validate recovery protocols.

## Functional core / imperative shell split

- Pure core: descriptor and operation validation, key/range and sequence ordering, preconditions, batch planning, atomicity-domain checks, durability-state transitions, effect-transaction transitions, recovery decisions, quota accounting, and evidence payloads.
- Shell: open adapters, read and write bytes, flush, truncate, compact, persist snapshots, inspect external effects, inject simulated faults, and record bounded evidence.

## Risks / Trade-offs

- Port contracts can accidentally mirror Redb. Use canonical values and conformance behavior rather than backend transactions or cursors.
- Reserve/commit/abort may be misread as distributed atomic commit. Scope every effect transaction to its declared adapter and non-claims.
- Deterministic crash injection increases test complexity. Centralize fault labels and require positive and negative adapter fixtures.
