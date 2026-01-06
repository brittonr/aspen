# Potential Layers for Aspen: Ultra Analysis

**Date**: 2026-01-05
**Status**: Research Document
**Analysis Type**: ULTRA mode - comprehensive layer opportunity assessment

## Executive Summary

Aspen already has a rich layer architecture built on its `KeyValueStore` trait. This analysis identifies **high-value layer opportunities** that complement existing capabilities, drawing from FoundationDB patterns, etcd primitives, and modern distributed systems research.

**Existing Layers**: 12 implemented (Tuple, Subspace, Index, DNS, Coordination, SQL, FUSE, Forge, Pijul, Jobs, Blob, Sharding)

**Recommended New Layers** (by priority):

1. ~~**Watch/Subscription Layer**~~ - Already implemented (`aspen-client::watch`)
2. **Time-Series Layer** - Efficient time-stamped data with retention (HIGH)
3. **Graph Layer** - Property graph on ordered KV (HIGH)
4. **Document Layer** - JSON/BSON document storage with querying (MEDIUM-HIGH)
5. ~~**Session/Lease Layer**~~ - Already exists via TTL + coordination primitives
6. **Pub/Sub Layer** - Topic-based messaging (MEDIUM)
7. **Cache Layer** - Distributed cache with eviction policies (MEDIUM)
8. **Geo Layer** - Geospatial indexing and queries (LOW-MEDIUM)
9. **Multi-Region Layer** - Cross-cluster replication (LOW - complex)
10. **Schema Registry Layer** - Versioned schema management (LOW)

---

## Current Layer Inventory

### Foundation Layers (aspen-layer crate)

| Layer | LOC | Purpose |
|-------|-----|---------|
| **Tuple** | ~800 | FoundationDB-compatible order-preserving encoding |
| **Subspace** | ~250 | Namespace isolation, multi-tenancy |
| **Secondary Index** | ~813 | Client-managed indexes with transactional updates |

### Application Layers

| Layer | Crate | Purpose |
|-------|-------|---------|
| **DNS** | aspen-dns | DNS record management + CRDT sync |
| **SQL** | aspen-sql | DataFusion query engine over Redb |
| **FUSE** | aspen-fuse | Mount cluster as POSIX filesystem |
| **Forge** | aspen-forge | Decentralized Git (Radicle-like) |
| **Pijul** | aspen-pijul | Patch-based VCS |
| **Jobs** | aspen-jobs | Distributed job scheduling |
| **Blob** | aspen-blob | Content-addressed large object storage |
| **Sharding** | aspen-sharding | Horizontal scaling via consistent hashing |
| **Auth** | aspen-auth | UCAN-style capability tokens |

### Coordination Primitives (aspen-coordination)

Already implemented and comprehensive:

- DistributedLock, RWLockManager, SemaphoreManager
- LeaderElection with fencing tokens
- AtomicCounter, SignedAtomicCounter, SequenceGenerator
- QueueManager with visibility timeout and DLQ
- BarrierManager
- ServiceRegistry with health checks
- DistributedRateLimiter
- DistributedWorkerCoordinator

---

## High-Priority Layer Opportunities

### 1. Watch/Subscription Layer - ALREADY IMPLEMENTED

**Status**: Fully implemented in `crates/aspen-client/src/watch.rs`

**Location**: `aspen_client::watch::WatchSession`

**Features**:

- Streaming log subscription via `LOG_SUBSCRIBER_ALPN`
- Prefix-based filtering
- Historical replay from specific index
- TTL-aware events (`SetWithTTL`, `Delete`, `Expire`)
- Cookie-based authentication
- Keepalive heartbeats

**API**:

```rust
// Connect and subscribe
let session = WatchSession::connect(endpoint, node_addr, "cookie").await?;
let mut subscription = session.subscribe("user:", 0).await?;

// Receive events
while let Some(event) = subscription.next().await {
    match event {
        WatchEvent::Set { key, value, index, .. } => { ... }
        WatchEvent::Delete { key, index, .. } => { ... }
        WatchEvent::SetWithTTL { key, value, expires_at_ms, .. } => { ... }
    }
}
```

**Priority**: N/A - Already complete

---

### 2. Time-Series Layer

**Problem**: No efficient storage/query for time-stamped metrics, events, logs.

**Use Cases**:

- Cluster metrics history
- Application telemetry
- Event logs with time-based queries
- IoT sensor data

**Proposed API**:

```rust
pub struct TimeSeriesStore<KV: KeyValueStore> {
    kv: Arc<KV>,
    namespace: Subspace,
}

impl TimeSeriesStore<KV> {
    /// Insert a data point
    pub async fn insert(&self, series: &str, timestamp: i64, value: f64, tags: &Tags) -> Result<()>;

    /// Query time range
    pub async fn query(&self, series: &str, start: i64, end: i64) -> Result<Vec<DataPoint>>;

    /// Aggregate query
    pub async fn aggregate(&self, query: AggregateQuery) -> Result<AggregateResult>;

    /// Configure retention policy
    pub async fn set_retention(&self, series: &str, policy: RetentionPolicy) -> Result<()>;
}

pub enum Aggregation {
    Sum, Avg, Min, Max, Count, Percentile(f64),
    Rate,      // Per-second rate
    Delta,     // Difference from previous
    Histogram { buckets: Vec<f64> },
}
```

**Key Encoding** (using Tuple layer):

```
(namespace, series_name, timestamp_ns) -> (value: f64, tags_hash)
(namespace, "_meta", series_name) -> SeriesMetadata
(namespace, "_tags", tags_hash) -> Tags
```

**Retention/Compaction**:

- Background garbage collection for expired data
- Downsampling for older data (raw -> 1min -> 1hr -> 1day)
- Use existing index layer for tag-based queries

**Complexity**: ~2,000 LOC
**Priority**: HIGH - essential for observability, monitoring

---

### 3. Graph Layer

**Problem**: No native graph traversal or relationship modeling.

**Use Cases**:

- Social graphs
- Dependency graphs (packages, services)
- Knowledge graphs
- ACL/permission graphs

**Proposed API**:

```rust
pub struct GraphStore<KV: KeyValueStore> {
    kv: Arc<KV>,
    vertices: Subspace,
    edges: Subspace,
    indexes: Subspace,
}

impl GraphStore<KV> {
    /// Add a vertex with properties
    pub async fn add_vertex(&self, id: &VertexId, label: &str, props: Properties) -> Result<()>;

    /// Add an edge with properties
    pub async fn add_edge(&self, from: &VertexId, to: &VertexId, label: &str, props: Properties) -> Result<()>;

    /// Traverse outgoing edges
    pub async fn outgoing(&self, from: &VertexId, edge_label: Option<&str>) -> Result<Vec<Edge>>;

    /// Traverse incoming edges (reverse lookup)
    pub async fn incoming(&self, to: &VertexId, edge_label: Option<&str>) -> Result<Vec<Edge>>;

    /// Multi-hop traversal
    pub async fn traverse(&self, query: TraversalQuery) -> Result<TraversalResult>;
}
```

**Key Encoding**:

```
// Vertices
(vertices, vertex_id) -> VertexData { label, properties }

// Outgoing edges (efficient forward traversal)
(edges, "out", from_vertex, edge_label, to_vertex) -> EdgeData { properties }

// Incoming edges (efficient reverse traversal)
(edges, "in", to_vertex, edge_label, from_vertex) -> EdgeData { properties }

// Property indexes (optional)
(indexes, "vertex", property_name, property_value, vertex_id) -> ""
(indexes, "edge", property_name, property_value, edge_key) -> ""
```

**Complexity**: ~1,800 LOC
**Priority**: HIGH - common pattern in modern applications

---

### 4. Document Layer

**Problem**: No structured document storage with nested queries.

**FoundationDB Reference**: Document Layer provides MongoDB-compatible API.

**Proposed API**:

```rust
pub struct DocumentStore<KV: KeyValueStore> {
    kv: Arc<KV>,
    collections: Subspace,
}

impl DocumentStore<KV> {
    /// Insert/update document
    pub async fn upsert(&self, collection: &str, doc: Document) -> Result<DocumentId>;

    /// Find by ID
    pub async fn get(&self, collection: &str, id: &DocumentId) -> Result<Option<Document>>;

    /// Query with filter
    pub async fn find(&self, collection: &str, filter: Filter, opts: FindOptions) -> Result<Vec<Document>>;

    /// Create index on field path
    pub async fn create_index(&self, collection: &str, field_path: &str, index_type: IndexType) -> Result<()>;

    /// Aggregate pipeline
    pub async fn aggregate(&self, collection: &str, pipeline: Vec<Stage>) -> Result<Vec<Document>>;
}

pub enum Filter {
    Eq(String, Value),
    Gt(String, Value),
    In(String, Vec<Value>),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Exists(String),
    Regex(String, String),
}
```

**Key Encoding**:

```
// Documents
(collection, "_doc", doc_id) -> CBOR/MessagePack encoded document

// Indexes (using aspen-layer Index)
(collection, "_idx", index_name, field_value, doc_id) -> ""

// Collection metadata
(collection, "_meta") -> CollectionMetadata { indexes, schema_version }
```

**Complexity**: ~2,500 LOC
**Priority**: MEDIUM-HIGH - common developer expectation

---

### 5. Session/Lease Layer

**Problem**: While coordination has LeaderElection with leases, there's no general-purpose session management.

**etcd Reference**: etcd leases are TTL-based keys that auto-expire.

**Current State**: TTL mentioned in layer-architecture.md but implementation status unclear.

**Proposed API**:

```rust
pub struct SessionManager<KV: KeyValueStore> {
    kv: Arc<KV>,
}

impl SessionManager<KV> {
    /// Create a session with TTL
    pub async fn create(&self, ttl: Duration) -> Result<SessionId>;

    /// Attach key to session (key deleted when session expires)
    pub async fn attach(&self, session: &SessionId, key: &[u8]) -> Result<()>;

    /// Keep session alive
    pub async fn keepalive(&self, session: &SessionId) -> Result<Duration>;

    /// Revoke session (immediate cleanup)
    pub async fn revoke(&self, session: &SessionId) -> Result<()>;

    /// List keys attached to session
    pub async fn list_keys(&self, session: &SessionId) -> Result<Vec<Vec<u8>>>;
}
```

**Implementation**:

- Session metadata stored in system subspace
- Background task for expiration (use existing `idx_expires_at` index)
- Keepalive updates expiration timestamp
- On expiration: delete all attached keys atomically

**Complexity**: ~800 LOC
**Priority**: MEDIUM-HIGH - enables ephemeral service registration, distributed locks

---

### 6. Pub/Sub Layer

**Problem**: No topic-based messaging beyond gossip.

**Use Cases**:

- Event broadcasting
- Decoupled microservices communication
- Real-time notifications

**Proposed API**:

```rust
pub struct PubSub<KV: KeyValueStore> {
    kv: Arc<KV>,
}

impl PubSub<KV> {
    /// Publish message to topic
    pub async fn publish(&self, topic: &str, message: &[u8]) -> Result<MessageId>;

    /// Subscribe to topic
    pub async fn subscribe(&self, topic: &str) -> Result<Subscription>;

    /// Subscribe with options (from offset, filter, batch)
    pub async fn subscribe_with_options(&self, topic: &str, opts: SubscribeOptions) -> Result<Subscription>;

    /// Acknowledge message processing
    pub async fn ack(&self, subscription: &SubscriptionId, message_id: &MessageId) -> Result<()>;
}
```

**Key Encoding**:

```
// Messages (append-only log per topic)
(topics, topic_name, message_id) -> MessageData { payload, timestamp, producer_id }

// Subscription state
(subscriptions, subscription_id) -> SubscriptionState { topic, last_acked, filter }

// Topic metadata
(topics, "_meta", topic_name) -> TopicMetadata { retention, partitions }
```

**Relationship to Existing**:

- Builds on QueueManager patterns
- Uses Watch layer for push notifications
- Could leverage iroh-gossip for fan-out

**Complexity**: ~1,200 LOC
**Priority**: MEDIUM - useful but QueueManager covers some use cases

---

### 7. Cache Layer

**Problem**: No explicit cache abstraction with eviction policies.

**Use Cases**:

- Distributed cache (Redis replacement)
- Session caching
- API response caching

**Proposed API**:

```rust
pub struct DistributedCache<KV: KeyValueStore> {
    kv: Arc<KV>,
}

impl DistributedCache<KV> {
    /// Get with optional refresh
    pub async fn get(&self, key: &str) -> Result<Option<CachedValue>>;

    /// Set with TTL
    pub async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;

    /// Get or compute (cache-aside pattern)
    pub async fn get_or_compute<F>(&self, key: &str, compute: F) -> Result<Vec<u8>>
    where F: FnOnce() -> Future<Output = Result<Vec<u8>>>;

    /// Configure cache limits
    pub async fn configure(&self, namespace: &str, config: CacheConfig) -> Result<()>;
}

pub struct CacheConfig {
    pub max_entries: u64,
    pub max_bytes: u64,
    pub default_ttl: Duration,
    pub eviction_policy: EvictionPolicy,
}

pub enum EvictionPolicy {
    Lru,         // Least recently used
    Lfu,         // Least frequently used
    Fifo,        // First in, first out
    Random,      // Random eviction
}
```

**Implementation**:

- Use sessions/leases for TTL
- Track access patterns for LRU/LFU
- Background eviction based on memory pressure

**Complexity**: ~1,000 LOC
**Priority**: MEDIUM - Raft latency may make this less useful than local caches

---

### 8. Geo Layer

**Problem**: No geospatial indexing or queries.

**Use Cases**:

- Location-based services
- Geofencing
- Proximity search

**Proposed API**:

```rust
pub struct GeoStore<KV: KeyValueStore> {
    kv: Arc<KV>,
}

impl GeoStore<KV> {
    /// Add location for entity
    pub async fn add(&self, namespace: &str, entity_id: &str, location: GeoPoint) -> Result<()>;

    /// Find entities within radius
    pub async fn radius_search(&self, namespace: &str, center: GeoPoint, radius_km: f64) -> Result<Vec<GeoResult>>;

    /// Find entities in bounding box
    pub async fn box_search(&self, namespace: &str, bbox: GeoBoundingBox) -> Result<Vec<GeoResult>>;

    /// Find K nearest neighbors
    pub async fn knn(&self, namespace: &str, point: GeoPoint, k: usize) -> Result<Vec<GeoResult>>;
}
```

**Key Encoding** (using Geohash or S2):

```
// S2 cell indexing for efficient spatial queries
(geo, namespace, s2_cell_id, entity_id) -> GeoPoint { lat, lng, metadata }

// Reverse lookup
(geo, namespace, "_entity", entity_id) -> s2_cell_id
```

**Complexity**: ~1,500 LOC
**Priority**: LOW-MEDIUM - niche use case

---

### 9. Multi-Region Layer

**Problem**: Single-cluster Raft limits geographic distribution.

**Use Cases**:

- Disaster recovery
- Low-latency global reads
- Data sovereignty compliance

**Proposed API**:

```rust
pub struct MultiRegionStore<KV: KeyValueStore> {
    local: Arc<KV>,
    remote_clusters: Vec<ClusterConnection>,
}

impl MultiRegionStore<KV> {
    /// Write to local + async replicate
    pub async fn write(&self, key: &[u8], value: &[u8], consistency: WriteConsistency) -> Result<()>;

    /// Read with consistency level
    pub async fn read(&self, key: &[u8], consistency: ReadConsistency) -> Result<Option<Vec<u8>>>;

    /// Configure replication for key prefix
    pub async fn set_replication_policy(&self, prefix: &[u8], policy: ReplicationPolicy) -> Result<()>;
}

pub enum WriteConsistency {
    Local,           // Ack after local Raft commit
    Quorum,          // Ack after majority of regions
    All,             // Ack after all regions
}

pub enum ReadConsistency {
    Local,           // Read from local (possibly stale)
    Linearizable,    // Read from leader region
    Bounded(Duration), // Accept staleness up to duration
}
```

**Implementation Strategy**:

- Use iroh-docs CRDT for async replication (already exists!)
- Conflict resolution via vector clocks or last-writer-wins
- DocsExporter pattern from DNS layer

**Complexity**: ~2,500 LOC
**Priority**: LOW - complex, requires careful design

---

### 10. Schema Registry Layer

**Problem**: No versioned schema management for structured data.

**Use Cases**:

- Event schema evolution (Avro, Protobuf, JSON Schema)
- API contract versioning
- Data validation

**Proposed API**:

```rust
pub struct SchemaRegistry<KV: KeyValueStore> {
    kv: Arc<KV>,
}

impl SchemaRegistry<KV> {
    /// Register new schema version
    pub async fn register(&self, subject: &str, schema: Schema) -> Result<SchemaVersion>;

    /// Get schema by version
    pub async fn get(&self, subject: &str, version: SchemaVersion) -> Result<Option<Schema>>;

    /// Get latest schema
    pub async fn latest(&self, subject: &str) -> Result<Option<Schema>>;

    /// Check compatibility
    pub async fn check_compatibility(&self, subject: &str, schema: &Schema) -> Result<CompatibilityResult>;

    /// Set compatibility mode
    pub async fn set_compatibility(&self, subject: &str, mode: CompatibilityMode) -> Result<()>;
}

pub enum CompatibilityMode {
    None,
    Backward,       // New can read old
    Forward,        // Old can read new
    Full,           // Backward + Forward
    BackwardTransitive,
    ForwardTransitive,
    FullTransitive,
}
```

**Complexity**: ~1,200 LOC
**Priority**: LOW - niche, can use external registry

---

## Implementation Recommendations

### Phase 1: Data Models (High Value)

1. **Time-Series Layer** - Observability is essential for any production system
2. **Graph Layer** - Relationship modeling for complex domains

### Phase 2: Developer Experience

3. **Document Layer** - Familiar MongoDB-like API for rapid development
4. **Pub/Sub Layer** - Event-driven architecture patterns

### Phase 3: Specialized Needs

5. **Cache Layer** - Performance optimization (evaluate if Raft latency is acceptable)
6. **Geo Layer** - Location services (only if use case emerges)
7. **Multi-Region** - Global distribution (complex, defer until needed)
8. **Schema Registry** - Data governance (can use external registry)

---

## Layer Implementation Checklist

For each new layer, follow this pattern (derived from existing layers):

- [ ] Create `crates/aspen-{layer}/` crate
- [ ] Define clear trait interface
- [ ] Use `Subspace` for key namespace isolation
- [ ] Use `Tuple` for composite key encoding
- [ ] Implement Tiger Style resource bounds
- [ ] Write comprehensive tests (unit, integration, madsim)
- [ ] Add feature flag if optional
- [ ] Re-export from main `aspen` crate

---

## References

- [FoundationDB Layer Concept](https://apple.github.io/foundationdb/layer-concept.html)
- [FoundationDB Record Layer Paper](https://www.foundationdb.org/files/record-layer-paper.pdf)
- [etcd Features](https://etcd.io/docs/v3.5/dev-guide/features/)
- [TiKV Architecture](https://tikv.org/deep-dive/overview/)
- [Consul Sessions](https://developer.hashicorp.com/consul/docs/dynamic-app-config/sessions)
- [ZooKeeper Recipes](https://zookeeper.apache.org/doc/current/recipes.html)
- [Redis Data Types](https://redis.io/docs/data-types/)
- [Cloudflare Quicksilver](https://blog.cloudflare.com/quicksilver-v2-evolution-of-a-globally-distributed-key-value-store-part-1/)

---

## Decision Record

**Decision**: This document catalogs layer opportunities. No immediate implementation planned.

**Rationale**:

- Aspen already has 12 mature layers covering most use cases
- Watch layer is the most critical gap
- Additional layers should be driven by concrete Blixard service needs

**Next Steps**:

1. Audit existing TTL/Lease implementation status
2. Implement Watch layer if not already complete
3. Revisit this analysis when specific needs arise
