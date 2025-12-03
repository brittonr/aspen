# ADR-007: Trait-Based API Design

**Status:** Accepted

**Date:** 2025-12-03

**Context:** Aspen provides distributed primitives for cluster coordination and key-value storage. The API must support multiple implementations (Raft-based, in-memory, future alternatives) while maintaining a stable interface for HTTP clients and tests.

## Decision

Define narrow, focused trait-based APIs (`ClusterController` and `KeyValueStore`) with multiple implementations for different contexts (production Raft backend, deterministic test backend).

## Implementation

Located in `src/api/mod.rs`:

### Core Traits

```rust
#[async_trait]
pub trait ClusterController: Send + Sync {
    async fn init(&self, request: InitRequest)
        -> Result<ClusterState, ControlPlaneError>;

    async fn add_learner(&self, request: AddLearnerRequest)
        -> Result<ClusterState, ControlPlaneError>;

    async fn change_membership(&self, request: ChangeMembershipRequest)
        -> Result<ClusterState, ControlPlaneError>;

    async fn current_state(&self)
        -> Result<ClusterState, ControlPlaneError>;

    async fn get_metrics(&self)
        -> Result<RaftMetrics<AppTypeConfig>, ControlPlaneError>;

    async fn trigger_snapshot(&self)
        -> Result<Option<LogId<AppTypeConfig>>, ControlPlaneError>;
}

#[async_trait]
pub trait KeyValueStore: Send + Sync {
    async fn write(&self, request: WriteRequest)
        -> Result<WriteResult, KeyValueStoreError>;

    async fn read(&self, request: ReadRequest)
        -> Result<ReadResult, KeyValueStoreError>;
}
```

### Implementation Variants

| Implementation | Purpose | Backend |
|----------------|---------|---------|
| `DeterministicClusterController` | Unit/integration tests | In-memory `BTreeMap` |
| `DeterministicKeyValueStore` | Unit/integration tests | In-memory `HashMap` |
| `RaftControlClient` | Production | Raft consensus + redb |

### Observability APIs

Added in recent commits:

```rust
/// Get the current Raft metrics for observability.
async fn get_metrics(&self)
    -> Result<RaftMetrics<AppTypeConfig>, ControlPlaneError>;

/// Trigger a snapshot to be taken immediately.
async fn trigger_snapshot(&self)
    -> Result<Option<LogId<AppTypeConfig>>, ControlPlaneError>;

/// Get the current leader ID, if known.
async fn get_leader(&self)
    -> Result<Option<u64>, ControlPlaneError> {
    Ok(self.get_metrics().await?.current_leader)
}
```

These provide:
- Raft state (Leader/Follower/Candidate/Learner)
- Current leader ID and term
- Log indices (last_log, last_applied, snapshot, purged)
- Replication state (leader-only metrics)

### HTTP Façade

HTTP endpoints (`src/http.rs`) are implemented in terms of traits:

```rust
async fn handle_init(
    controller: Arc<dyn ClusterController>,
    request: InitRequest,
) -> Response {
    match controller.init(request).await {
        Ok(state) => json_response(state),
        Err(e) => error_response(e),
    }
}
```

This decouples HTTP layer from Raft implementation details.

## Rationale

### Why Trait-Based?

1. **Multiple implementations**: Test in-memory backend, production Raft backend, future Hiqlite backend
2. **Testability**: Inject deterministic implementations in tests
3. **Decoupling**: HTTP layer doesn't depend on Raft internals
4. **Future-proofing**: Can swap backends without changing API contracts

### Why Two Traits?

Separate concerns:
- **ClusterController**: Cluster membership, consensus operations
- **KeyValueStore**: Data plane operations

This follows the "single responsibility principle" and allows independent evolution.

### Tiger Style Alignment

From `tigerstyle.md`:

- **Simplify function signatures**: Traits use simple request/result types
- **Minimize dimensionality**: Each trait has one responsibility
- **Clear naming**: `ClusterController` and `KeyValueStore` are unambiguous
- **Explicit error types**: `ControlPlaneError` and `KeyValueStoreError` are distinct

Example from implementation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    pub id: u64,                    // Tiger Style: u64, not usize
    pub addr: String,
    pub raft_addr: Option<String>,
}
```

## Alternatives Considered

### 1. Direct Raft Exposure

**Approach:** HTTP endpoints call openraft's `Raft` struct directly

**Pros:**
- Fewer abstraction layers
- Direct access to all Raft features

**Cons:**
- Tight coupling to openraft API
- Hard to test (requires real Raft instances)
- Leaky abstraction (exposes Raft internals)
- API changes when openraft changes

**Rejected:** Violates decoupling principles. Traits provide stability.

### 2. Single Monolithic API

**Approach:** One `AspenAPI` trait with all methods

```rust
trait AspenAPI {
    async fn init(...);
    async fn add_learner(...);
    async fn write(...);
    async fn read(...);
    // etc.
}
```

**Pros:**
- Single trait to implement
- Simple for small implementations

**Cons:**
- Violates single responsibility principle
- Forces implementations to support everything
- Hard to extend (trait evolution problem)
- Unclear ownership of different concerns

**Rejected:** Mixing control plane and data plane operations in one trait is poor separation of concerns.

### 3. gRPC/Protobuf

**Approach:** Define API in `.proto` files, generate Rust code

**Pros:**
- Language-agnostic interface
- Built-in versioning story
- Code generation

**Cons:**
- Over-engineered for in-process Rust APIs
- Serialization overhead
- Code generation complexity
- Less idiomatic Rust (no `Result<T, E>`, must use gRPC status codes)

**Rejected:** Aspen is Rust-native. Traits are more idiomatic and zero-cost.

### 4. Builder Pattern Facades

**Approach:** Fluent builders instead of traits

```rust
cluster.init()
    .with_members(vec![node1, node2])
    .execute()
    .await?;
```

**Pros:**
- Ergonomic API
- Type-safe construction

**Cons:**
- Doesn't solve polymorphism problem
- More complex than simple traits
- Harder to mock/test

**Rejected:** Builders are useful for construction, but traits are better for polymorphism.

## Consequences

### Positive

1. **Testability**: Easy to inject deterministic backends
2. **Flexibility**: Can swap Raft for other consensus algorithms
3. **Stability**: API remains stable even if implementation changes
4. **Clarity**: Traits document the contract explicitly

### Negative

1. **Indirection**: One extra layer between HTTP and Raft
   - **Mitigation**: Cost is negligible (virtual dispatch is ~1 ns)

2. **Trait evolution**: Adding methods breaks existing implementations
   - **Mitigation**: Provide default implementations or use versioned traits

### Usage Examples

**Production (future Raft implementation):**

```rust
let raft_controller = RaftControlClient::new(raft_handle);
let raft_kv = RaftKeyValueStore::new(raft_handle);

let http_server = HttpServer::new(
    Arc::new(raft_controller) as Arc<dyn ClusterController>,
    Arc::new(raft_kv) as Arc<dyn KeyValueStore>,
);
```

**Testing:**

```rust
let test_controller = DeterministicClusterController::new();
let test_kv = DeterministicKeyValueStore::new();

let http_server = HttpServer::new(
    Arc::new(test_controller) as Arc<dyn ClusterController>,
    Arc::new(test_kv) as Arc<dyn KeyValueStore>,
);
```

## Implementation Notes

### Request/Response Types

All requests and responses are `Serialize + Deserialize` for HTTP/JSON compatibility:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitRequest {
    pub initial_members: Vec<ClusterNode>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterState {
    pub nodes: Vec<ClusterNode>,
    pub members: Vec<u64>,
    pub learners: Vec<ClusterNode>,
}
```

### Error Types

Explicit error enums for each domain:

```rust
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlPlaneError {
    #[error("invalid request: {reason}")]
    InvalidRequest { reason: String },

    #[error("cluster not initialized")]
    NotInitialized,

    #[error("operation failed: {reason}")]
    Failed { reason: String },
}
```

Tiger Style: Named fields, explicit error messages, no implicit conversions.

### Observability Integration

The `get_metrics()` and `trigger_snapshot()` methods expose Raft internals for observability without polluting the core API:

```rust
// HTTP endpoint can inspect Raft state
let metrics = controller.get_metrics().await?;
println!("Current leader: {:?}", metrics.current_leader);
println!("Raft state: {:?}", metrics.state);
```

This follows the "centralize control flow" principle: high-level API methods call lower-level Raft operations.

## References

- Trait definitions: `src/api/mod.rs`
- In-memory implementations: `src/api/inmemory.rs`
- Raft implementation: `src/raft/control.rs` (planned)
- HTTP façade: `src/http.rs` (planned)
- Tiger Style: `tigerstyle.md`
