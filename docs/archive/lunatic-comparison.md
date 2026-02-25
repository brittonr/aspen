# Aspen vs Lunatic: Comparative Analysis

## Context

This analysis compares two Rust-based distributed systems frameworks that both draw inspiration from Erlang/BEAM but take fundamentally different architectural approaches:

- **Aspen**: Production-ready distributed orchestration platform (~392K LOC, 83 crates)
- **Lunatic**: Erlang-inspired WebAssembly actor runtime (~4K LOC, 22 crates)

---

## Executive Summary

| Aspect | Aspen | Lunatic |
|--------|-------|---------|
| **Primary Goal** | Self-hosted distributed infrastructure | Universal WebAssembly actor runtime |
| **Maturity** | Production-ready (A- 8.5/10) | Experimental (no releases since May 2023) |
| **Architecture** | Direct async APIs + Raft consensus | WebAssembly actors + supervision trees |
| **Isolation Model** | Process-level (OS) | WebAssembly sandboxing (per-actor) |
| **Networking** | Iroh P2P (QUIC) | QUIC + mTLS |
| **Consensus** | Raft (vendored OpenRaft) | None (eventual consistency) |
| **Language Support** | Rust-only | Any language compiling to WebAssembly |
| **Formal Verification** | 27 Verus-verified modules | None |
| **Testing** | Deterministic simulation (madsim) | Integration tests |

---

## Architectural Philosophy

### Aspen: Distributed Consensus First

Aspen is a **foundational orchestration layer** focused on strong consistency guarantees:

```
+-------------------------------------------------------------+
|                    Aspen Architecture                        |
+-------------------------------------------------------------+
|  Applications: Forge (Git), CI/CD, Jobs, DNS, Secrets       |
+-------------------------------------------------------------+
|  Coordination: Locks, Elections, Queues, Barriers, Semaphores|
+-------------------------------------------------------------+
|  Consensus: Raft (linearizable operations)                  |
+-------------------------------------------------------------+
|  Storage: Single-fsync Redb (1.65ms writes)                 |
+-------------------------------------------------------------+
|  Network: Iroh P2P (QUIC, NAT traversal, gossip discovery)  |
+-------------------------------------------------------------+
```

**Key design decisions:**

1. **No actors** - Removed actor architecture (Dec 2025) for simpler direct async APIs
2. **Raft consensus** - All cluster state goes through linearizable Raft operations
3. **Single-fsync storage** - Bundled log + state machine for 6x write performance
4. **P2P native** - No HTTP API, all communication via Iroh QUIC
5. **Self-hosting goal** - Hosts its own Git (Forge) and CI on Aspen

### Lunatic: WebAssembly Actors First

Lunatic is an **actor runtime** focused on isolation and language portability:

```
+-------------------------------------------------------------+
|                   Lunatic Architecture                       |
+-------------------------------------------------------------+
|  Application (Rust/C/Go compiled to WebAssembly)            |
+-------------------------------------------------------------+
|  Supervision Trees (Erlang-style links/monitors)            |
+-------------------------------------------------------------+
|  Actors: WebAssembly instances with isolated memory         |
+-------------------------------------------------------------+
|  Runtime: Wasmtime JIT + Tokio work-stealing scheduler      |
+-------------------------------------------------------------+
|  Distribution: QUIC + mTLS (control plane + data plane)     |
+-------------------------------------------------------------+
```

**Key design decisions:**

1. **WebAssembly sandboxing** - Each actor has isolated memory, stack, syscalls
2. **Erlang process model** - Links, monitors, supervision trees, "let it crash"
3. **Language agnostic** - Any language compiling to WebAssembly works
4. **Preemptive scheduling** - Fuel-based yielding ensures fairness
5. **Fine-grained permissions** - Per-process resource controls (memory, CPU, filesystem)

---

## Detailed Comparison

### 1. Concurrency Model

**Aspen: Direct Async with Tokio**

```rust
// Direct async method calls on RaftNode
let result = raft_node.write(WriteRequest { key, value }).await?;
let state = raft_node.current_state().await?;

// Coordination primitives built on CAS operations
let lock = DistributedLock::new(store, "my-lock")?;
let guard = lock.acquire().await?;
```

- Uses `tokio::sync` primitives
- `JoinSet`/`TaskTracker` for task supervision
- Never hold locks across `.await` points (Tiger Style)
- Functions kept under 70 lines

**Lunatic: WebAssembly Actors**

```rust
// Spawn isolated WebAssembly processes
let child = Process::spawn(|mailbox| {
    loop {
        let msg = mailbox.receive()?;
        // Handle message in isolated memory space
    }
});

// Link for failure propagation
Process::link(&child);
child.send(MyMessage { ... });
```

- Each process = WebAssembly instance
- Isolated linear memory per actor
- Fuel-based preemption (100k instructions/yield)
- Links propagate failures, monitors observe

**Verdict**: Aspen provides stronger consistency guarantees via Raft; Lunatic provides stronger isolation via WebAssembly sandboxing.

---

### 2. Distributed Coordination

**Aspen: Raft Consensus + Coordination Primitives**

| Primitive | Implementation | Guarantee |
|-----------|----------------|-----------|
| DistributedLock | CAS with fencing tokens | Linearizable |
| LeaderElection | Raft-backed with renewal | Exactly one leader |
| AtomicCounter | CAS operations | Race-free |
| Queue | Visibility timeout + DLQ | At-least-once delivery |
| RWLock | Distributed read-write | Fairness guarantees |
| Barrier | Raft-coordinated | Synchronization point |

All primitives are **formally verified** with Verus (27 modules).

**Lunatic: Message Passing + Links**

| Primitive | Implementation | Guarantee |
|-----------|----------------|-----------|
| Process Links | Bidirectional death notification | Cascade failures |
| Monitors | Unidirectional observation | Death notification |
| Registry | Named process lookup | Cluster-wide discovery |
| Messages | Mailbox with tags | Selective receive |

No built-in distributed consensus; relies on application-level coordination.

**Verdict**: Aspen is designed for strong consistency (distributed transactions, locks); Lunatic is designed for fault tolerance (supervision trees, crash recovery).

---

### 3. Networking

**Aspen: Iroh P2P (No HTTP)**

- All communication via QUIC
- ALPN-based protocol routing (CLIENT_ALPN, RAFT_ALPN, GOSSIP_ALPN)
- Multiple discovery mechanisms (mDNS, gossip, DNS, DHT)
- NAT traversal built-in
- Ed25519-signed gossip announcements

**Lunatic: QUIC + mTLS**

- Control plane for node registration
- Data plane for inter-process messaging
- Certificate-based access control
- Dynamic code loading across nodes

**Verdict**: Similar transport (QUIC), but Aspen is more P2P-native with multiple discovery mechanisms; Lunatic uses traditional control/data plane separation.

---

### 4. Storage

**Aspen: Single-Fsync Redb Architecture**

```
Traditional:                    Aspen:
+-------------+                +-----------------+
| Log Store   | <- fsync #1    | SharedRedbStorage|
+------+------+                | (log + state)   | <- single fsync
       |                       +--------+--------+
+-------------+                         |
|StateMachine | <- fsync #2    +-----------------+
+-------------+                | DataFusion SQL  |
                               +-----------------+
```

**Performance** (Ryzen 9950X3D + Samsung 9100 PRO NVMe):

- Single-node writes: **1.65ms** (6x faster than traditional)
- 3-node writes: **3.2ms** (5.8x faster)
- Read latency: **5.0us**

**Lunatic: No Built-in Persistent Storage**

- SQLite API for per-process database access
- No distributed storage layer
- State lives in process memory (lost on crash)

**Verdict**: Aspen provides production-grade distributed storage; Lunatic requires external storage solutions.

---

### 5. Formal Verification

**Aspen: 27 Verus-Verified Modules**

Verified invariants include:

- LOCK-1: Fencing token monotonicity
- LOCK-2: Mutual exclusion (at most one holder)
- QUEUE-1: FIFO ordering preservation
- SEQ-1: Sequence uniqueness

Architecture separates verified pure functions (`src/verified/`) from Verus specs (`verus/`).

**Lunatic: None**

No formal verification. Relies on Rust's type system and testing.

**Verdict**: Aspen has significant investment in formal correctness proofs; Lunatic trusts WebAssembly's runtime guarantees.

---

### 6. Testing

**Aspen: Deterministic Simulation (madsim)**

```rust
#[madsim::test]
async fn test_leader_crash_and_reelection() {
    let mut t = AspenRaftTester::new(3, "leader_crash").await;
    t.crash_node(leader).await;
    madsim::time::sleep(Duration::from_secs(10)).await;
    let new_leader = t.check_one_leader().await?;
}
```

- Reproducible failures (seeded RNG)
- Time manipulation
- Network partition simulation
- 350+ passing tests
- Property-based testing (proptest)

**Lunatic: Integration Testing**

- Tests compile to wasm32-wasi
- cargo-lunatic test runner
- Real runtime execution (not mocks)
- Limited simulation capabilities

**Verdict**: Aspen's deterministic simulation testing is significantly more advanced for distributed systems validation.

---

### 7. Language Support

**Aspen: Rust Only**

- Deep integration with Rust ecosystem
- Uses Rust's type system extensively
- No cross-language support

**Lunatic: Any Language compiling to WebAssembly**

- Rust, C/C++, AssemblyScript, Go
- Language-agnostic actor model
- Dynamic code loading across architectures

**Verdict**: Lunatic wins on polyglot support; Aspen wins on Rust ecosystem depth.

---

### 8. Production Readiness

**Aspen**:

- Status: **Production-ready** (A- 8.5/10)
- ~392,000 lines of code, 83 crates
- 350+ passing tests
- Active development (commits in Feb 2026)
- Self-hosting goal (Forge + CI)

**Lunatic**:

- Status: **Experimental**
- Last release: May 2023 (v0.13.2)
- No documented production deployments
- WASI compatibility incomplete
- Y Combinator W21 company

**Verdict**: Aspen is production-ready; Lunatic is experimental/research-stage.

---

## Use Case Recommendations

### Use Aspen When

1. **Building distributed infrastructure** (databases, queues, locks)
2. **Need strong consistency** (linearizable operations)
3. **Self-hosting requirements** (Git, CI, secrets)
4. **Production deployment** needed
5. **Formal correctness** matters
6. **P2P networking** is desired (NAT traversal, gossip discovery)

### Use Lunatic When

1. **Running untrusted code** (WebAssembly sandboxing)
2. **Polyglot environments** (multiple languages)
3. **Erlang-style supervision** patterns
4. **Research/experimentation** (not production)
5. **Fine-grained process isolation** needed

---

## Summary Table

| Feature | Aspen | Lunatic | Winner |
|---------|-------|---------|--------|
| Production readiness | A- (8.5/10) | Experimental | **Aspen** |
| Consistency model | Linearizable (Raft) | Eventual | **Aspen** |
| Actor isolation | OS process | WebAssembly sandbox | **Lunatic** |
| Language support | Rust only | Any to WASM | **Lunatic** |
| Formal verification | 27 modules | None | **Aspen** |
| Distributed testing | madsim simulation | Integration tests | **Aspen** |
| Storage | Single-fsync Redb | None built-in | **Aspen** |
| Fault tolerance | Raft consensus | Supervision trees | Tie |
| Networking | Iroh P2P | QUIC + mTLS | **Aspen** |
| Self-hosting | Forge + CI | N/A | **Aspen** |
| Code size | 392K LOC | 4K LOC | Tie (different scope) |

---

## Integration Levels: How Lunatic Could Run on Aspen

Based on deep exploration of both codebases, there are **four integration levels** ranging from lightweight to deep integration:

### Level 1: Lunatic as External Client (Minimal Integration)

**Approach**: Lunatic processes connect to Aspen as external clients via the RPC protocol.

```
+----------------------+     CLIENT_ALPN      +----------------------+
|   Lunatic Runtime    |<----(QUIC)---------->|    Aspen Cluster     |
|   (separate process) |                      |    (Raft consensus)  |
|                      |                      |                      |
|  WASM Actors:        |                      |  Services:           |
|  - KV get/put -------+--------------------->|  - KeyValueStore     |
|  - Lock acquire -----+--------------------->|  - DistributedLock   |
|  - Queue enqueue ----+--------------------->|  - QueueManager      |
+----------------------+                      +----------------------+
```

**Implementation**:

- Create `lunatic-aspen-api` crate with host functions
- WASM actors call `lunatic::aspen::kv::get()`, `lunatic::aspen::lock::acquire()`, etc.
- Host functions delegate to `aspen-client` RPC

**Key Files**:

- Lunatic: `/crates/lunatic-sqlite-api/` (template for external storage)
- Aspen: `/crates/aspen-client-api/src/messages/` (RPC protocol)

**Pros**: Minimal changes to both projects, clear separation
**Cons**: Network latency for every primitive access, two separate deployments

---

### Level 2: Lunatic as Aspen Worker Plugin (Recommended)

**Approach**: Lunatic runtime embedded as an Aspen job worker, executing WASM actors as jobs.

```
+-----------------------------------------------------------------+
|                        Aspen Node                               |
|  +------------------------------------------------------------+ |
|  |                     Worker Pool                             | |
|  |   +-------------+  +-------------+  +------------------+   | |
|  |   | ShellWorker |  | NixWorker   |  | LunaticWorker    |   | |
|  |   +-------------+  +-------------+  | (WASM executor)  |   | |
|  |                                     |  +-------------+  |   | |
|  |                                     |  | WASM Actor  |  |   | |
|  |                                     |  | WASM Actor  |  |   | |
|  |                                     |  | WASM Actor  |  |   | |
|  |                                     |  +-------------+  |   | |
|  |                                     +------------------+    | |
|  +------------------------------------------------------------+ |
|                              |                                  |
|  +---------------------------v--------------------------------+ |
|  |   Core Services (KV, Raft, Coordination, Blobs)            | |
|  +------------------------------------------------------------+ |
+-----------------------------------------------------------------+
```

**Implementation**:

```rust
// crates/aspen-lunatic-worker/src/lib.rs
pub struct LunaticWorker {
    runtime: LunaticRuntime,
    kv: Arc<dyn KeyValueStore>,
    coordination: Arc<CoordinationService>,
}

#[async_trait]
impl Worker for LunaticWorker {
    async fn execute(&self, job: Job) -> JobResult {
        // 1. Load WASM module from iroh-blobs (by hash)
        let wasm_bytes = self.blobs.get(&job.payload.hash).await?;

        // 2. Spawn Lunatic process with Aspen context
        let process = self.runtime.spawn_with_state(
            wasm_bytes,
            AspenProcessState::new(self.kv.clone(), self.coordination.clone())
        ).await?;

        // 3. Wait for completion
        process.join().await
    }

    fn job_types(&self) -> Vec<String> {
        vec!["wasm-actor".into(), "lunatic-process".into()]
    }
}
```

**Key Files**:

- Aspen: `/crates/aspen-jobs/src/worker.rs` (Worker trait, lines 32-67)
- Aspen: `/crates/aspen-jobs/src/vm_executor/hyperlight.rs` (VM executor template)
- Lunatic: `/src/state.rs` (ProcessState, lines 150-165)

**Pros**: Single deployment, direct access to Aspen primitives, job scheduling
**Cons**: Tighter coupling, requires Lunatic as Rust dependency

---

### Level 3: Lunatic Distribution Replaced by Aspen (Deep Integration)

**Approach**: Replace Lunatic's QUIC-based distribution with Aspen's Iroh P2P networking.

```
+--------------------------------------------------------------------+
|                    Lunatic + Aspen Hybrid                           |
|                                                                    |
|  +---------------------------------------------------------+      |
|  |                  Lunatic Process Supervisor              |      |
|  |   +----------+  +----------+  +----------+  +----------+|      |
|  |   | Actor A  |  | Actor B  |  | Actor C  |  | Actor D  ||      |
|  |   | (local)  |  | (local)  |  | (remote) |  | (remote) ||      |
|  |   +----+-----+  +----+-----+  +----+-----+  +----+-----+|      |
|  |        |             |             |             |       |      |
|  |        +-------------+-------------+-------------+       |      |
|  |                         |                                |      |
|  |  +----------------------v------------------------------+ |      |
|  |  |          Aspen-Backed Distribution Layer             | |      |
|  |  |  - Process registry -> Aspen ServiceRegistry (Raft)  | |      |
|  |  |  - Message routing -> Aspen Iroh P2P (QUIC + NAT)   | |      |
|  |  |  - Remote spawn -> Aspen JobManager                  | |      |
|  |  |  - Node discovery -> Aspen gossip + DHT              | |      |
|  |  +----------------------------------------------------- + |      |
|  +---------------------------------------------------------+      |
|                              |                                     |
|  +---------------------------v-----------------------------------+ |
|  |              Aspen Core (Raft, KV, Iroh, Blobs)               | |
|  +---------------------------------------------------------------+ |
+--------------------------------------------------------------------+
```

**Implementation Changes to Lunatic**:

1. **Replace Registry** (`/crates/lunatic-registry-api/src/lib.rs`):

```rust
// Before: Local HashMap
pub type ProcessRegistry = Arc<RwLock<HashMap<String, (u64, u64)>>>;

// After: Aspen ServiceRegistry
impl RegistryCtx for AspenProcessState {
    async fn put(&self, name: &str, process_id: u64) -> Result<()> {
        self.aspen.service_registry_register(name, process_id).await
    }

    async fn get(&self, name: &str) -> Result<Option<u64>> {
        self.aspen.service_registry_lookup(name).await
    }
}
```

2. **Replace Distributed Client** (`/crates/lunatic-distributed/src/distributed/client.rs`):

```rust
// Before: Quinn QUIC
pub struct Client { quinn_client: quinn::Client }

// After: Aspen Iroh
pub struct AspenDistributedClient {
    iroh_endpoint: iroh::Endpoint,
    aspen_client: AspenClient,
}

impl AspenDistributedClient {
    async fn send(&self, node_id: u64, message: Signal) -> Result<()> {
        // Route via Aspen's Iroh with automatic NAT traversal
        let conn = self.iroh_endpoint.connect(node_id, LUNATIC_ALPN).await?;
        conn.send(&message).await
    }

    async fn spawn_remote(&self, node_id: u64, wasm: &[u8]) -> Result<ProcessId> {
        // Submit as Aspen job to target node
        self.aspen_client.job_submit(JobRequest {
            job_type: "lunatic-spawn",
            node_id: Some(node_id),
            payload: wasm.into(),
        }).await
    }
}
```

**Key Files**:

- Lunatic: `/crates/lunatic-distributed/src/distributed/client.rs` (lines 98-284)
- Lunatic: `/crates/lunatic-registry-api/src/lib.rs` (line 73)
- Aspen: `/crates/aspen-coordination/src/registry.rs`
- Aspen: `/crates/aspen-cluster/src/endpoint_manager.rs`

**Pros**: Full P2P networking (NAT traversal, DHT), fault-tolerant registry
**Cons**: Significant modification to Lunatic, tight coupling

---

### Level 4: Lunatic as Aspen Plugin (Future Architecture)

**Approach**: Leverage Aspen's planned plugin system for first-class Lunatic support.

Aspen has a **detailed plugin architecture plan** (`/docs/planning/plugin-system.md`, 1244 lines) that defines:

```rust
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> &PluginManifest;
    async fn init(&mut self, ctx: PluginContext) -> Result<()>;
    async fn ready(&mut self) -> Result<()>;
    async fn shutdown(&mut self) -> Result<()>;
    fn request_handlers(&self) -> Vec<Arc<dyn RequestHandler>>;
    fn protocol_handlers(&self) -> Vec<(AlpnProtocol, Arc<dyn ProtocolHandler>)>;
}

pub struct PluginContext {
    pub kv: NamespacedKvStore,        // Auto-namespaced KV access
    pub blobs: Option<Arc<dyn BlobStore>>,
    pub cluster: Arc<dyn ClusterInfo>,
    pub events: Arc<dyn EventBus>,
    pub tasks: Arc<dyn TaskSpawner>,
}
```

**Lunatic Plugin Implementation**:

```rust
pub struct LunaticPlugin {
    runtime: LunaticRuntime,
    ctx: PluginContext,
}

impl Plugin for LunaticPlugin {
    fn manifest(&self) -> &PluginManifest {
        PluginManifest {
            name: "lunatic",
            version: "0.13.2",
            job_types: vec!["wasm-actor", "lunatic-process"],
            alpn_protocols: vec![b"lunatic-actor"],
            kv_prefixes: vec!["lunatic/registry/", "lunatic/state/"],
        }
    }

    async fn init(&mut self, ctx: PluginContext) -> Result<()> {
        self.runtime = LunaticRuntime::with_aspen_backend(
            ctx.kv.clone(),
            ctx.blobs.clone(),
        )?;
        self.ctx = ctx;
        Ok(())
    }

    fn protocol_handlers(&self) -> Vec<(AlpnProtocol, Arc<dyn ProtocolHandler>)> {
        vec![(b"lunatic-actor", Arc::new(LunaticProtocolHandler::new(&self.runtime)))]
    }
}
```

**Pros**: Clean separation, standardized lifecycle, multiple plugin tiers
**Cons**: Plugin system not yet implemented in Aspen

---

## Integration Summary Table

| Level | Integration Depth | Changes Required | Benefits | Complexity |
|-------|------------------|------------------|----------|------------|
| **1** | External Client | New Lunatic API crate | Minimal coupling | Low |
| **2** | Worker Plugin | New Aspen worker crate | Job scheduling, direct KV | Medium |
| **3** | Distribution Replace | Modify Lunatic internals | Full P2P, fault-tolerant registry | High |
| **4** | Plugin System | Wait for Aspen plugin API | Clean architecture, multiple tiers | Future |

---

## Recommended Path

**Start with Level 2 (Worker Plugin)**:

1. Create `aspen-lunatic-worker` crate implementing `Worker` trait
2. Add `lunatic-aspen-api` crate with host functions for KV, coordination
3. WASM modules stored in iroh-blobs, executed by LunaticWorker
4. Actors access Aspen primitives via host function calls

**Evolve to Level 3/4** as needs grow:

- Replace Lunatic's distribution with Aspen's Iroh when cross-node actors needed
- Migrate to plugin architecture when Aspen plugin system ships

---

## What Aspen Should Learn from Lunatic

Based on analysis of both codebases, Aspen has **excellent static safety** (Tiger Style, Verus) and **operation-level fault tolerance** (DLQ, retries, CAS), but lacks **process-level resilience**. Lunatic's architecture reveals critical patterns Aspen should adopt.

### Current Gaps in Aspen

| Feature | Lunatic | Aspen Current State |
|---------|---------|---------------------|
| **Task Supervision** | Hierarchical supervision trees | Single Raft supervisor only |
| **Crash Propagation** | Links and monitors | Manual handle tracking |
| **Process Isolation** | WebAssembly sandboxing | Shared-memory tokio tasks |
| **Restart Strategies** | one_for_one, one_for_all, rest_for_one | Exponential backoff only |
| **Process Groups** | Dynamic registry | Static worker pools |
| **CPU Fairness** | Fuel-based preemption | No limits on tasks |
| **Message Routing** | Tag-based selective receive | ALPN only |

**Evidence of gaps:**

- 127 `tokio::spawn()` calls across 74 files, only 8 files use `TaskTracker`
- Most spawned tasks are orphaned (no supervision)
- Worker crashes marked Failed but never restarted
- No way to propagate failure from child to parent

---

### Recommendation 1: Supervision Trees

**Lunatic Pattern** (`/crates/lunatic-process/src/lib.rs:124-169`):

```rust
// Bidirectional links with optional tags
Signal::Link(Option<i64>, Arc<dyn Process>)
Signal::LinkDied(u64, Option<i64>, DeathReason)

// Death reasons for supervisors
enum DeathReason { Normal, Failure, NoProcess }
```

**Proposed for Aspen** (`crates/aspen-supervisor/`):

```rust
pub enum RestartStrategy {
    /// Restart only the failed child
    OneForOne { max_restarts: u32, window: Duration },
    /// Restart all children if one fails
    OneForAll { max_restarts: u32, window: Duration },
    /// Restart failed child and all children started after it
    RestForOne { max_restarts: u32, window: Duration },
}

pub struct Supervisor<S: KeyValueStore> {
    children: Vec<ChildSpec>,
    strategy: RestartStrategy,
    links: HashMap<TaskId, Vec<TaskId>>,  // Failure propagation
}

pub struct ChildSpec {
    id: String,
    start: Box<dyn Fn() -> BoxFuture<'static, Result<()>>>,
    restart: ChildRestart,  // Permanent, Transient, Temporary
    shutdown: Duration,
}

impl<S: KeyValueStore> Supervisor<S> {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                biased;  // Always check signals first (Lunatic pattern)

                signal = self.signal_rx.recv() => {
                    match signal {
                        Signal::ChildDied(id, reason) => {
                            self.handle_child_death(id, reason).await?;
                        }
                        Signal::Shutdown => break,
                    }
                }

                // Poll children only after signals processed
                _ = self.poll_children() => {}
            }
        }
    }
}
```

**Benefits:**

- CI pipeline stages as supervision tree (restart failed stage, not whole pipeline)
- Worker pools self-heal without manual intervention
- Distributed job execution with proper crash recovery

---

### Recommendation 2: Process Links and Monitors

**Lunatic Pattern** (`/crates/lunatic-process/src/lib.rs:296-457`):

```rust
// Configurable link behavior
Signal::DieWhenLinkDies(bool)

// Link: bidirectional death notification with cascade option
// Monitor: unidirectional observation without cascade
```

**Proposed for Aspen:**

```rust
pub enum LinkMode {
    /// Die immediately when linked process dies (Erlang default)
    Cascade,
    /// Receive message about death, handle manually
    Trap,
}

pub trait Linkable: Send + Sync {
    fn link(&self, other: &dyn Linkable, mode: LinkMode);
    fn monitor(&self, other: &dyn Linkable);
    fn unlink(&self, other: &dyn Linkable);
}

// Usage in job workers
impl Linkable for JobWorker {
    fn link(&self, other: &dyn Linkable, mode: LinkMode) {
        self.links.insert(other.id(), (other.clone(), mode));
        other.links.insert(self.id(), (self.clone(), mode));
    }
}
```

**Key files to modify:**

- `/crates/aspen-jobs/src/worker.rs` - Add link support to workers
- `/crates/aspen-raft/src/supervisor.rs` - Generalize to support links

---

### Recommendation 3: Fuel-Based Preemption for Jobs

**Lunatic Pattern** (`/crates/lunatic-process/src/runtimes/wasmtime.rs:47-58`):

```rust
const UNIT_OF_COMPUTE_IN_INSTRUCTIONS: u64 = 100_000;

store.set_fuel(fuel)?;
store.fuel_async_yield_interval(Some(UNIT_OF_COMPUTE_IN_INSTRUCTIONS))?;
```

**Proposed for Aspen VM Executor:**

```rust
// crates/aspen-jobs/src/vm_executor/mod.rs
pub struct VmJobConfig {
    /// Maximum WASM instructions before job is terminated
    pub max_fuel: Option<u64>,
    /// Instructions per yield for fairness (default: 100k)
    pub yield_interval: u64,
    /// Maximum memory in bytes
    pub max_memory: usize,
}

impl VmExecutor {
    async fn execute_with_limits(&self, wasm: &[u8], config: VmJobConfig) -> JobResult {
        let mut store = Store::new(&self.engine, state);

        if let Some(fuel) = config.max_fuel {
            store.set_fuel(fuel)?;
        }
        store.fuel_async_yield_interval(Some(config.yield_interval))?;

        // Execution automatically yields every 100k instructions
        // Enables: job cost accounting, fair scheduling, timeout detection
    }
}
```

**Benefits:**

- CI jobs can't monopolize executor threads
- Cost accounting per job (fuel consumed = compute cost)
- Fair scheduling without cooperative yielding bugs

---

### Recommendation 4: Tag-Based Selective Receive

**Lunatic Pattern** (`/crates/lunatic-process/src/mailbox.rs:32-74`):

```rust
// Search mailbox for matching tag
pub async fn pop(&self, tags: Option<&[i64]>) -> Message {
    if let Some(tags) = tags {
        let index = mailbox.iter().position(|msg| tags.contains(&msg.tag()));
        if let Some(index) = index {
            return mailbox.remove(index);
        }
    }
    // Await new message if no match
}
```

**Proposed for Aspen RPC:**

```rust
// crates/aspen-client-api/src/messages/mod.rs
pub struct TaggedRequest {
    pub correlation_id: u64,  // For response matching
    pub request: ClientRpcRequest,
}

// crates/aspen-transport/src/client.rs
impl RpcClient {
    /// Send request and receive response with matching correlation ID
    pub async fn call(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        let correlation_id = self.next_correlation_id();
        self.send(TaggedRequest { correlation_id, request }).await?;

        // Selective receive - only match our correlation ID
        self.receive_with_tag(correlation_id).await
    }

    /// Concurrent requests without head-of-line blocking
    pub async fn call_many(
        &self,
        requests: Vec<ClientRpcRequest>,
    ) -> Vec<Result<ClientRpcResponse>> {
        let futures = requests.into_iter().map(|r| self.call(r));
        futures::future::join_all(futures).await
    }
}
```

**Benefits:**

- Multiple concurrent RPC calls without blocking
- Request/response correlation without separate channels
- Cleaner timeout handling (drop future, message preserved)

---

### Recommendation 5: Fine-Grained Job Permissions

**Lunatic Pattern** (`/src/config.rs`):

```rust
struct ProcessConfig {
    max_memory: usize,
    max_fuel: Option<u64>,
    can_compile_modules: bool,
    can_spawn_processes: bool,
    preopened_dirs: Vec<(String, String)>,
}
```

**Proposed for Aspen Jobs:**

```rust
// crates/aspen-jobs/src/permissions.rs
pub struct JobPermissions {
    /// Maximum memory usage
    pub max_memory: usize,
    /// Maximum CPU fuel (WASM instructions)
    pub max_fuel: Option<u64>,
    /// Network access level
    pub network: NetworkPermission,
    /// KV namespace access (empty = no access)
    pub kv_namespaces: Vec<String>,
    /// Blob prefix access (empty = no access)
    pub blob_prefixes: Vec<String>,
    /// Can access secrets store
    pub secrets_access: SecretsPermission,
    /// Filesystem paths (for shell workers)
    pub filesystem_paths: Vec<PathBuf>,
}

pub enum NetworkPermission {
    None,
    LocalhostOnly,
    AllowList(Vec<String>),  // DNS names or CIDRs
    Unrestricted,
}

pub enum SecretsPermission {
    None,
    ReadOnly(Vec<String>),   // Secret key prefixes
    ReadWrite(Vec<String>),
}
```

**Usage:**

```rust
// CI job from untrusted fork - minimal permissions
let untrusted_job = JobRequest {
    permissions: JobPermissions {
        max_memory: 512 * 1024 * 1024,  // 512MB
        max_fuel: Some(10_000_000_000),  // ~100 seconds
        network: NetworkPermission::None,
        kv_namespaces: vec![format!("ci/jobs/{}", job_id)],
        secrets_access: SecretsPermission::None,
        ..Default::default()
    },
    ..
};

// Trusted deployment job - elevated permissions
let deploy_job = JobRequest {
    permissions: JobPermissions {
        network: NetworkPermission::Unrestricted,
        secrets_access: SecretsPermission::ReadOnly(vec!["deploy/".into()]),
        ..Default::default()
    },
    ..
};
```

---

### Recommendation 6: Resource Transfer Between Jobs

**Lunatic Pattern** (`/crates/lunatic-process/src/message.rs`):

```rust
pub struct DataMessage {
    buffer: Vec<u8>,                       // Serialized data
    resources: Vec<Option<Arc<Resource>>>, // Host resources
}

// Add resource, get index for serialization
pub fn add_resource(&mut self, resource: Arc<Resource>) -> usize
```

**Proposed for Aspen:**

```rust
// crates/aspen-jobs/src/resource_transfer.rs
pub struct TransferableMessage {
    pub data: Vec<u8>,
    pub resources: Vec<TransferableResource>,
}

pub enum TransferableResource {
    /// QUIC stream from Iroh
    QuicStream(iroh::Connection),
    /// File descriptor (Unix only)
    FileDescriptor(RawFd),
    /// Blob reference
    BlobRef(iroh_blobs::Hash),
    /// KV transaction handle
    Transaction(TransactionId),
}

impl TransferableMessage {
    /// Create from struct + extract resources
    pub fn serialize<T: Serialize + ExtractResources>(value: T) -> Self {
        let (data, resources) = value.serialize_with_resources();
        Self { data, resources }
    }

    /// Reconstruct struct + inject resources
    pub fn deserialize<T: Deserialize + InjectResources>(self) -> T {
        T::deserialize_with_resources(self.data, self.resources)
    }
}
```

**Use Cases:**

- Transfer WebSocket connection from dying worker to fresh one
- Hand off SSH connection during Git operation
- Pass database transaction between job stages

---

### Implementation Priority

| Priority | Feature | Effort | Impact | Key Files |
|----------|---------|--------|--------|-----------|
| **P0** | Supervision Trees | Medium | High | New `aspen-supervisor` crate |
| **P0** | Process Links | Medium | High | `aspen-jobs/src/worker.rs` |
| **P1** | Job Permissions | Low | High | `aspen-jobs/src/permissions.rs` |
| **P1** | Fuel-Based Preemption | Low | Medium | `aspen-jobs/src/vm_executor/` |
| **P2** | Tagged Messages | Medium | Medium | `aspen-client-api/`, `aspen-transport/` |
| **P3** | Resource Transfer | High | Medium | New serialization system |

---

### Summary: Lunatic Patterns for Aspen

**Adopt from Lunatic:**

1. **Supervision trees** with restart strategies (one_for_one, one_for_all)
2. **Process links** for crash propagation with trap/cascade modes
3. **Fuel-based preemption** for fair CPU scheduling in WASM jobs
4. **Tag-based selective receive** for concurrent RPC handling
5. **Fine-grained permissions** per job (memory, CPU, network, KV, secrets)
6. **Resource transfer** serialization for zero-downtime worker handoff

**Keep from Aspen:**

- Raft consensus for strong consistency
- Verus formal verification for pure functions
- Tiger Style resource bounds
- Single-fsync storage architecture
- Iroh P2P networking

**The result**: Aspen with Erlang-style process supervision, maintaining its distributed systems strengths while gaining process-level fault tolerance.

---

## Conclusion

**Aspen and Lunatic solve different problems:**

- **Aspen** is a **complete distributed infrastructure platform** with strong consistency, formal verification, and production readiness. It's designed to be the foundation layer on which distributed applications are built.

- **Lunatic** is an **actor runtime** bringing Erlang's process model to WebAssembly. It excels at isolation and polyglot support but lacks distributed consensus and isn't production-ready.

**They are complementary, not competing:**

- Lunatic could run *on top of* Aspen at multiple integration levels
- Aspen provides infrastructure primitives (Raft, KV, locks, queues, P2P)
- Lunatic provides actor runtime (WebAssembly sandboxing, supervision trees)

**For a Rust-based distributed system today**: Choose **Aspen** for production deployments requiring strong consistency and comprehensive infrastructure primitives.

**For WebAssembly actor experimentation**: Choose **Lunatic** for its elegant Erlang-inspired model and language portability, understanding it's not production-ready.

**For the best of both worlds**: Integrate Lunatic as an Aspen worker plugin (Level 2), gaining Aspen's distributed primitives while keeping Lunatic's actor model and WASM isolation.
