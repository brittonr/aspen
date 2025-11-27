# Blixard Circular Dependencies and Module Coupling Analysis

## Executive Summary

The codebase exhibits **moderate coupling** with **no detected circular dependencies** at the module level. However, there are several problematic coupling patterns that create brittleness and reduce testability:

1. **God Object (AppState)** - Central coupling point
2. **Concrete Type Coupling** - VmManager and HiqliteService hardwired into infrastructure
3. **Bidirectional Trait Dependencies** - Some services depend on both repositories and events
4. **Infrastructure-Domain Entanglement** - VmManager leaks into domain layer
5. **Hidden Event System Coupling** - Event publishing creates implicit dependencies

## Dependency Graph Overview

### Simplified Dependency Flow (layers)

```
HTTP Handlers (router, queue, worker, dashboard)
    ↓
AppState (god object)
    ↓ (both infrastructure AND services)
InfrastructureState + DomainServices
    ↓
HiqliteService, IrohService, WorkQueue, VmManager (CONCRETE)
```

## Critical Issues

### 1. GOD OBJECT: AppState

**Severity**: CRITICAL
**Location**: `src/state/mod.rs`

```rust
pub struct AppState {
    infrastructure: Arc<InfrastructureState>,
    services: Arc<DomainServices>,
}
```

**Problems**:
- Every HTTP handler depends on AppState
- AppState depends on ALL infrastructure services
- Adding/removing any service requires updating AppState
- Difficult to test handlers in isolation
- Tight coupling across entire application

**Dependency Scope**:
```
AppState
├── InfrastructureState
│   ├── HiqliteService (CONCRETE - not trait)
│   ├── IrohService (CONCRETE - not trait)
│   ├── WorkQueue
│   ├── VmManager (CONCRETE - not trait)
│   └── ExecutionRegistry
└── DomainServices
    ├── JobLifecycleService
    ├── ClusterStatusService
    ├── HealthService
    ├── VmService
    ├── TofuService
    └── WorkerManagementService
```

**Why it matters**: Every handler file imports:
- `src/handlers/worker.rs:use crate::state::AppState;`
- `src/handlers/queue.rs:use crate::state::AppState;`
- `src/handlers/dashboard.rs:use crate::state::AppState;`

A single handler change potentially affects all tests that use AppState.

---

### 2. CONCRETE TYPE COUPLING: HiqliteService

**Severity**: HIGH
**Location**: `src/state/infrastructure.rs:39`

```rust
pub struct InfrastructureState {
    pub(crate) hiqlite: HiqliteService,  // ← CONCRETE, not a trait
}
```

**Problem**: HiqliteService is stored as concrete type, not trait object.

**Why it's coupled**:
- Repositories wrap HiqliteService directly
- `src/repositories/hiqlite_repository.rs` creates impl from HiqliteService
- Cannot swap for mock/test implementation
- HiqliteService's generic methods force concrete storage

**Affected modules**:
```
HiqliteService (CONCRETE)
├── vm_manager/mod.rs:16 (creates Arc<HiqliteService>)
├── vm_manager/vm_registry.rs (takes Arc<HiqliteService>)
├── repositories/hiqlite_repository.rs (requires concrete)
├── domain/vm_service.rs (indirect via vm_manager)
└── state/services.rs:49 (HiqliteStateRepository::new)
```

---

### 3. CONCRETE TYPE COUPLING: VmManager

**Severity**: HIGH
**Location**: `src/state/infrastructure.rs:43`, `src/state/services.rs:67-79`

```rust
// Infrastructure holds concrete VmManager
pub vm_manager: Option<Arc<VmManager>>,

// State creation uses unsafe transmute when unavailable
unsafe {
    std::mem::transmute::<Arc<()>, Arc<crate::vm_manager::VmManager>>(
        Arc::new(()),
    )
}
```

**Problems**:
1. **Unsafe code due to missing abstraction** - VmManager has no trait boundary
2. **Hard dependency leak** - Domain services couple to concrete VmManager
3. **No behavior substitution** - Cannot test with mock VM backends
4. **Option<Arc<>> pattern** - Inconsistent abstraction (some backends optional, VmManager required by domain)

**Coupling Chain**:
```
src/domain/vm_service.rs
├── uses VmManager (CONCRETE)
├── uses vm_manager::VmAssignment
├── uses vm_manager::VmRegistry
├── uses vm_manager::VmStats
└── uses vm_manager::JobResult

src/adapters/vm_adapter.rs
├── uses VmManager (CONCRETE)
├── uses VmAssignment
└── imports from crate::vm_manager
```

**Test Impact**: Cannot isolate VmService tests from VmManager implementation details.

---

### 4. BIDIRECTIONAL DEPENDENCIES: Repository <-> Event System

**Severity**: MEDIUM
**Location**: `src/domain/job_commands.rs`

```rust
pub struct JobCommandService {
    work_repo: Arc<dyn WorkRepository>,  // ← Depends on repo
    event_publisher: Arc<dyn EventPublisher>,  // ← And events
}
```

**Why it matters**:
- Command service depends on both repository AND event system
- Event handlers may depend on repositories
- Creates potential for circular event/state updates

**Dependency chain**:
```
JobCommandService
├── WorkRepository (publish_work, update_status)
├── EventPublisher (publish events after mutations)
└── JobStateMachine (state transitions)
    └── Can be informed by domain events
```

**Problem**: If event handlers access repositories that trigger more events, you have hidden circular coupling.

---

### 5. HIDDEN SERVICE COUPLING: ExecutionRegistry (Weak Point)

**Severity**: MEDIUM
**Location**: `src/adapters/registry.rs`

The ExecutionRegistry manages multiple adapters:
```rust
pub struct ExecutionRegistry {
    backends: Arc<RwLock<HashMap<String, Arc<dyn ExecutionBackend>>>>,
    placement_strategy: Arc<dyn PlacementStrategy>,
}
```

**Current Status**: Good (trait-based)

**But Problem Areas**:
1. **Adapter implementations couple back to domain**:
   - `src/adapters/vm_adapter.rs` imports VmManager (concrete)
   - `src/adapters/flawless_adapter.rs` imports FlawlessWorker trait
   - Each adapter has different abstraction levels

2. **Job type coupling**: All adapters take `Job` type
   - Job type changes break all 5 adapters
   - No job type versioning/compatibility layer

---

### 6. DOMAIN LAYER ENTANGLEMENT: Too Many Responsibilities

**Severity**: MEDIUM
**Location**: `src/domain/` (24 files)

**Problematic modules**:
```
job_commands.rs (mutations) + job_queries.rs (reads)
    - Could be split into separate service tiers

job_lifecycle.rs (combines commands + queries)
    - Wraps both, creating wrapper coupling

cluster_status.rs + health_service.rs
    - Both aggregate repository data
    - Similar responsibilities could be consolidated

vm_service.rs
    - Bridges domain → vm_manager
    - Shouldn't exist if vm_manager stayed in infrastructure

tofu_service.rs (optional feature)
    - Couples domain to infrastructure (execution_registry)
    - Feature-flag creates code path fragmentation
```

**Coupling analysis**:
```
JobLifecycleService
├── JobCommandService
├── JobQueryService
└── Makes both part of same "lifecycle" even if they're separate concerns

ClusterStatusService
├── StateRepository
├── WorkRepository
└── Both read different sources, no natural relationship
```

---

## Module Dependency Matrix

### Critical Dependencies (Direction: Top → Bottom)

```
HTTP Handlers
  └─→ AppState (REQUIRED)
      ├─→ InfrastructureState (REQUIRED)
      │   ├─→ HiqliteService (CONCRETE)
      │   ├─→ IrohService (CONCRETE)
      │   ├─→ WorkQueue (CONCRETE)
      │   ├─→ VmManager (CONCRETE) ⚠️
      │   └─→ ExecutionRegistry (TRAIT - GOOD)
      │
      └─→ DomainServices (REQUIRED)
          ├─→ JobLifecycleService
          │   ├─→ JobCommandService
          │   │   ├─→ WorkRepository (TRAIT - GOOD)
          │   │   └─→ EventPublisher (TRAIT - GOOD)
          │   └─→ JobQueryService
          │       └─→ WorkRepository (TRAIT - GOOD)
          │
          ├─→ ClusterStatusService
          │   ├─→ StateRepository (TRAIT - GOOD)
          │   └─→ WorkRepository (TRAIT - GOOD)
          │
          ├─→ VmService ⚠️
          │   └─→ VmManager (CONCRETE)
          │       ├─→ HiqliteService (CONCRETE)
          │       ├─→ VmRegistry
          │       └─→ VmController
          │
          ├─→ TofuService (optional)
          │   └─→ ExecutionRegistry (TRAIT)
          │
          └─→ WorkerManagementService
              └─→ Repositories
```

### Cross-module Couplings (Hidden Dependencies)

```
domain/vm_service.rs
  └─→ vm_manager/mod.rs (CONCRETE)
      └─→ hiqlite_service.rs (CONCRETE)

adapters/vm_adapter.rs
  └─→ vm_manager/mod.rs (CONCRETE)
      └─→ hiqlite_service.rs (CONCRETE)

[Both point to VmManager - implicit dependency ordering]
```

---

## Shared Mutable State Issues

### 1. AppState (Shared via Axum)

```rust
// In main.rs
let state_builder = factory.build_state(...).await;
let (domain, infra, config, features) = state_builder.build();

// In handlers - shared read-only via Arc
pub async fn handle_queue(State(domain): State<DomainState>) { }
```

**Assessment**: SAFE
- AppState and sub-states are read-only from handlers
- Mutations go through service methods
- Services use Arc<RwLock<>> where needed

### 2. WorkQueue (Work Item Cache)

**Location**: `src/work_queue.rs` and `src/work_item_cache.rs`

```rust
pub struct WorkQueue {
    pub tickets: Arc<RwLock<Vec<TicketInfo>>>,
    pub cache: Arc<DashMap<String, Job>>,
}
```

**Assessment**: SAFE (mostly)
- Uses DashMap (concurrent HashMap)
- Uses RwLock for tickets
- Job mutations go through work_repo interface

**Potential race**: Job status changes via two paths:
1. `work_repo.update_status()` → database
2. Domain events may update cache independently

### 3. VmManager (Shared State)

**Location**: `src/vm_manager/mod.rs`

```rust
pub struct VmManager {
    pub registry: Arc<VmRegistry>,  // Shared
    pub controller: Arc<VmController>,  // Shared
    pub monitor: Arc<ResourceMonitor>,  // Shared
}
```

**Assessment**: POTENTIALLY UNSAFE
- VmManager shared across multiple request handlers
- VmRegistry holds VM lifecycle state
- VmController mutates VM state
- Monitor modifies resource state
- **No documented synchronization guarantees**

### 4. ExecutionRegistry (Backend Health Cache)

```rust
pub struct ExecutionRegistry {
    backends: Arc<RwLock<HashMap<...>>>,
    health_cache: Arc<RwLock<HashMap<String, BackendHealth>>>,
    handle_to_backend: Arc<RwLock<HashMap<String, String>>>,
}
```

**Assessment**: SAFE
- Proper use of RwLock for concurrent access
- Health cache is secondary (can be invalidated)
- Handle tracking is read-heavy

---

## Test Coupling Analysis

### Unit Test Coupling Issues

**Problem 1: AppState required for all handler tests**
```rust
// Can't test handler without entire AppState
#[test]
async fn test_queue_handler() {
    let state = setup_full_app_state().await;  // ← heavyweight
    // Now test one handler
}
```

**Result**: Test setup cost high, tests break on infrastructure changes

**Problem 2: VmManager couples domain tests to infrastructure**
```rust
// VmService tests must handle:
#[test]
async fn test_vm_service() {
    let vm_manager = VmManager::new(config, hiqlite).await;  // ← real hiqlite
    let vm_service = VmService::new(vm_manager);
    // Cannot mock hiqlite
}
```

**Result**: Cannot unit test VmService in isolation

**Problem 3: Domain events create implicit test dependencies**
```rust
// If event handlers access repositories:
pub async fn on_job_completed(event: DomainEvent) {
    let job = repo.find_by_id(&event.job_id).await;  // ← implicit repo coupling
    update_metrics(job);
}

// Test of job_commands must verify event handlers ran correctly
// But event handlers aren't explicitly tested
```

---

## Recommendations (Priority Order)

### IMMEDIATE (Blocking)

1. **Trait-ify VmManager**
   - Create `ExecutionBackend` trait wrapper
   - Remove concrete VmManager from domain layer
   - Move VmService to adapters/

2. **Extract AppState into smaller components**
   - Create handler-specific state types
   - Use dependency injection instead of full AppState
   - Reduces handler test coupling

### SHORT-TERM (1-2 weeks)

3. **Make HiqliteService trait-based or injectable**
   - Create DatabaseBackend trait
   - Allow swapping for test implementations
   - Remove from concrete storage

4. **Decouple event publishing from command services**
   - Move EventPublisher to separate layer
   - Allow event handlers to NOT access repositories (command-only pattern)
   - Prevents event → repo → event cycles

### MEDIUM-TERM (2-4 weeks)

5. **Separate CQRS services**
   - JobCommandService stays (mutations only)
   - JobQueryService extracted from JobLifecycleService
   - Less coupling between read/write concerns

6. **Move VM-related code to `vm_backend` feature**
   - Currently mixed in core domain
   - Feature-gate reduces coupling in non-VM builds

---

## Circular Dependency Check Results

### No Circular Dependencies Found

The module import graph is **acyclic**:
```
✓ Handlers → AppState → InfrastructureState → Services
✓ Services → Repositories → (no reverse dependency)
✓ Adapters → Domain types (one-way)
✓ Domain → event publishers (one-way)
```

**Why**: Proper layering architecture prevents circles.

### But: Hidden circular patterns exist in runtime behavior

```
Domain Events
├─ Triggered by: JobCommandService
├─ Handled by: EventHandlers  
└─ May read: Repositories
    └─ Repositories return: Job entities
        └─ Job state changes trigger: EventPublisher
            └─ Back to: EventHandlers
```

This isn't a compile-time circle but can cause:
- Infinite event loops if not careful
- Test assertion races
- Undefined event ordering

---

## Module Cohesion Issues (God Objects)

### High-responsibility modules:

```
src/domain/job_lifecycle.rs
├─ Commands wrapper
├─ Queries wrapper
├─ Status conversion
├─ Event publication
└─ Job formatting
Total Lines: ~200+
Reasons to change: 5+

src/state/infrastructure.rs
├─ HiqliteService holder
├─ IrohService holder
├─ WorkQueue holder
├─ VmManager holder
├─ ExecutionRegistry holder
Total Lines: ~150+
Reasons to change: 5+ (any infrastructure change)

src/vm_manager/mod.rs
├─ Registry management
├─ Controller management
├─ Router management
├─ Monitor management
├─ Health checking
Total Lines: ~150+
Reasons to change: 5+
```

---

## External Dependency Graph

### Cargo dependencies with potential circular patterns:

```
mvm-ci (root crate)
├─ hiqlite (database)
│  ├─ openraft (consensus)
│  └─ rusqlite (SQL)
├─ iroh (P2P)
│  ├─ iroh-quinn (QUIC)
│  └─ iroh-relay (NAT traversal)
├─ flawless (WASM runtime)
├─ module1 (workspace member) ← imported as dependency
└─ axum (HTTP server)
```

**Audit Result**: No circular external dependencies detected.

**But**: `module1` is a workspace member imported as external dependency:
```toml
module1 = { path = "workflows/module1" }
```

This creates implicit workspace coupling.

---

## Summary Statistics

| Metric | Value | Assessment |
|--------|-------|------------|
| Circular dependencies (compile-time) | 0 | PASS |
| Trait-based abstractions | 6/12 services | 50% - NEEDS IMPROVEMENT |
| Concrete type couplings | 3 major | BLOCKER |
| God objects | 2 (AppState, InfrastructureState) | MEDIUM PRIORITY |
| Modules with >1 reason to change | 5 | COHESION ISSUE |
| Feature-gated coupling points | 2 (vm-backend, tofu-support) | FRAGMENTATION |
| Shared mutable state containers | 4 | MOSTLY SAFE |
| Hidden event system cycles | 1 possible pattern | MONITOR |

