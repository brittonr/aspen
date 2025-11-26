# Decoupling and Module Coupling Recommendations

## Quick Reference: Priority Matrix

```
IMPACT  │ HIGH                              │ MEDIUM                   │ LOW
────────┼──────────────────────────────────┼──────────────────────────┼─────────
HIGH    │ 1. Trait-ify VmManager          │ 4. Decouple EventPublisher│
EFFORT  │ 2. Extract AppState             │ 5. Feature-gate VM code  │
        │ 3. Trait-ify HiqliteService    │ 6. Separate CQRS services│
────────┼──────────────────────────────────┼──────────────────────────┼─────────
LOW     │ 7. Move vm_service to adapters │ 8. Consolidate services │
EFFORT  │ 9. Add sync guarantees for      │ 10. Module documentation │
        │    VmManager                    │                          │
```

---

## IMMEDIATE PRIORITIES (Week 1-2)

### 1. Trait-ify VmManager (BLOCKING)

**Current Problem**:
```rust
// src/domain/vm_service.rs
pub struct VmService {
    vm_manager: Arc<VmManager>,  // ← CONCRETE TYPE
}

// This forces domain to know about infrastructure implementation
```

**Why it's blocking**: VmService cannot be tested without real VmManager and HiqliteService.

**Proposed Solution**:

```rust
// src/adapters/mod.rs
pub trait VirtualMachineBackend: Send + Sync {
    async fn create_vm(&self, config: VmConfig) -> Result<VmInstance>;
    async fn execute_job(&self, vm_id: &str, job: Job) -> Result<JobResult>;
    async fn get_vm_state(&self, vm_id: &str) -> Result<VmState>;
    async fn terminate_vm(&self, vm_id: &str) -> Result<()>;
    // ... other VM operations
}

// Concrete implementation wraps VmManager
pub struct VmManagerAdapter {
    vm_manager: Arc<VmManager>,
}

#[async_trait]
impl VirtualMachineBackend for VmManagerAdapter {
    async fn create_vm(&self, config: VmConfig) -> Result<VmInstance> {
        self.vm_manager.create(config).await
    }
    // ...
}

// Domain now uses trait
pub struct VmService {
    backend: Arc<dyn VirtualMachineBackend>,
}
```

**Implementation Checklist**:
- [ ] Create `VirtualMachineBackend` trait in adapters module
- [ ] Create `VmManagerAdapter` wrapping VmManager
- [ ] Update `VmService` to use trait
- [ ] Create `MockVmBackend` for testing
- [ ] Update state initialization to use adapter
- [ ] Verify all tests still pass

**Impact**:
- Fixes unsafe code in `state/services.rs:72-78`
- Allows unit testing VmService in isolation
- Reduces coupling from domain to infrastructure

**Files affected**:
- `src/adapters/mod.rs` - add trait
- `src/adapters/vm_adapter.rs` - refactor to use trait
- `src/domain/vm_service.rs` - change dependency
- `src/state/services.rs` - create adapter instead of raw VmManager

---

### 2. Extract AppState into Smaller Components (HIGH IMPACT)

**Current Problem**:
```rust
// Every handler needs the full AppState
pub async fn handle_queue(
    State(state): State<AppState>,
) -> Result<Json<Job>> {
    let service = state.domain_services().job_lifecycle();
    // Can't test without entire AppState
}
```

**Why it's high priority**: Makes all handler tests heavyweight and fragile.

**Proposed Solution: Handler-specific state types**

```rust
// src/state/handler_states.rs
pub struct QueueHandlerState {
    pub job_lifecycle: Arc<JobLifecycleService>,
    pub execution_registry: Arc<ExecutionRegistry>,
}

pub struct WorkerHandlerState {
    pub worker_management: Arc<WorkerManagementService>,
    pub job_lifecycle: Arc<JobLifecycleService>,
}

pub struct DashboardHandlerState {
    pub cluster_status: Arc<ClusterStatusService>,
    pub health: Arc<HealthService>,
}

// Usage in handlers
pub async fn handle_queue(
    State(state): State<QueueHandlerState>,
) -> Result<Json<Job>> {
    // Only need job_lifecycle and registry
    let job = state.job_lifecycle.submit_job(submission).await?;
    Ok(Json(job))
}

// In main.rs router setup
let queue_state = QueueHandlerState {
    job_lifecycle: app_state.domain_services().job_lifecycle().clone(),
    execution_registry: app_state.infrastructure().execution_registry().clone(),
};

router.layer(axum::middleware::from_fn_with_state(
    queue_state.clone(),
    middleware::auth,
))
```

**Implementation Steps**:
1. Create `src/state/handler_states.rs`
2. Define state types for each handler group
3. Update router to inject handler-specific states
4. Update each handler to use specific state
5. Update tests to only set up needed services

**Benefits**:
- Tests only instantiate required services
- Clear dependency documentation in code
- Easier to identify service dependencies
- Reduces brittleness when infrastructure changes

**Files affected**:
- `src/state/handler_states.rs` - new file
- `src/server/router.rs` - update state creation
- `src/handlers/*.rs` - update handler signatures

---

### 3. Trait-ify HiqliteService (HIGH EFFORT, HIGH IMPACT)

**Current Problem**:
```rust
// src/state/infrastructure.rs:39
pub struct InfrastructureState {
    pub(crate) hiqlite: HiqliteService,  // ← CONCRETE TYPE
    // Cannot mock for testing
}
```

**Why it matters**: HiqliteService's generic methods prevent trait-based storage, forcing concrete type usage.

**Approach A: Wrapper/Adapter Pattern** (Recommended for now)
```rust
// src/services/database.rs
pub trait DatabaseService: Send + Sync {
    async fn health_check(&self) -> Result<HealthStatus>;
    async fn store_job(&self, job: Job) -> Result<()>;
    async fn retrieve_job(&self, id: &str) -> Result<Option<Job>>;
    // ... specific methods only, no generics
}

pub struct HiqliteAdapter {
    inner: HiqliteService,
}

#[async_trait]
impl DatabaseService for HiqliteAdapter {
    async fn health_check(&self) -> Result<HealthStatus> {
        self.inner.health_check().await
    }
    // ...
}
```

**Approach B: Feature-gate internal generics** (If Approach A not sufficient)
```rust
// Keep HiqliteService concrete for internal use only
// Create thin trait boundary at repository layer
```

**Implementation Checklist**:
- [ ] Create `DatabaseService` trait
- [ ] Create `HiqliteAdapter` wrapper
- [ ] Create `MockDatabase` implementation
- [ ] Update infrastructure to use trait
- [ ] Move HiqliteService to private module
- [ ] Verify tests can use mock database

**Impact**:
- Enables mock database for testing
- Reduces coupling between repositories and hiqlite
- Makes test databases swappable

---

## SHORT-TERM IMPROVEMENTS (Week 3-4)

### 4. Decouple Event Publishing from Command Services

**Current Problem**:
```rust
// src/domain/job_commands.rs
pub struct JobCommandService {
    work_repo: Arc<dyn WorkRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl JobCommandService {
    pub async fn submit_job(&self, ...) -> Result<String> {
        let job_id = self.work_repo.publish_work(...).await?;
        self.event_publisher.publish(JobSubmitted { job_id, ... }).await?;
        Ok(job_id)
    }
}
```

**Hidden Risk**: If event handlers access repositories, circular coupling emerges:
```
submit_job()
  ↓ publishes → JobSubmitted event
  ↓ event handler processes
  ↓ accesses → repository (find_by_id)
  ↓ retrieves → job state
  ↓ [may modify] → job state again
  ↓ publishes → another event
  ↓ [back to] → event handler
```

**Proposed Solution: Event-first architecture**

```rust
// src/domain/event_bus.rs
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<()>;
    // Decoupled from commands
}

// src/domain/job_commands.rs (MODIFIED)
pub struct JobCommandService {
    work_repo: Arc<dyn WorkRepository>,
    event_bus: Arc<dyn EventBus>,  // ← Just publish
}

impl JobCommandService {
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        let job_id = self.work_repo.publish_work(...).await?;
        
        // Fire-and-forget event (async, no wait)
        let event_bus = self.event_bus.clone();
        tokio::spawn(async move {
            let _ = event_bus.publish(DomainEvent::JobSubmitted { ... }).await;
        });
        
        Ok(job_id)  // ← Return immediately, don't wait for events
    }
}

// src/domain/event_handlers.rs
pub struct JobEventHandlers {
    // NO REPO ACCESS - only state modification
    // If need to read state, do so BEFORE command
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<()>;
}

// Audit event handler - no repo mutations
pub struct AuditEventHandler {
    // Logger only, no database access
}

#[async_trait]
impl EventHandler for AuditEventHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<()> {
        tracing::info!("Event: {:?}", event);
        Ok(())
    }
}
```

**Rules to enforce**:
1. Event handlers CANNOT access WriteRepositories
2. Event handlers MAY access ReadRepositories (queries only)
3. Event handlers CANNOT publish more events
4. Commands fire events asynchronously (no await)

**Implementation Checklist**:
- [ ] Create explicit EventBus trait
- [ ] Separate EventHandler trait
- [ ] Implement fire-and-forget pattern
- [ ] Add event handler tests that verify no repo writes
- [ ] Document event handler rules

---

### 5. Feature-gate VM-related code

**Current Problem**:
```rust
// src/state/services.rs:34
pub struct DomainServices {
    #[cfg(feature = "vm-backend")]
    vm_service: Arc<VmService>,
}
```

This creates two different code paths:
- With `vm-backend`: Includes VmService, uses unsafe transmute
- Without: VmService doesn't exist, handlers fail

**Proposed Solution: Complete feature isolation**

```rust
// src/domain/mod.rs
#[cfg(feature = "vm-backend")]
pub mod vm_service;

#[cfg(feature = "vm-backend")]
pub use vm_service::VmService;

// src/state/services.rs
pub struct DomainServices {
    #[cfg(feature = "vm-backend")]
    vm_service: Arc<VmService>,
}

impl DomainServices {
    #[cfg(feature = "vm-backend")]
    pub fn vm_service(&self) -> &Arc<VmService> {
        &self.vm_service
    }

    #[cfg(not(feature = "vm-backend"))]
    pub fn vm_service(&self) -> &Arc<dyn std::any::Any> {
        panic!("vm-backend feature not enabled")
    }
}

// In handlers, guard all VM service usage
#[cfg(feature = "vm-backend")]
pub async fn handle_vm_submit(...) -> Result<...> {
    state.vm_service().execute(...).await
}

#[cfg(not(feature = "vm-backend"))]
pub async fn handle_vm_submit(...) -> Result<...> {
    Err(anyhow!("VM backend not available"))
}
```

**Implementation Checklist**:
- [ ] Move all vm_service code behind `#[cfg(feature = "vm-backend")]`
- [ ] Remove unsafe transmute
- [ ] Update Cargo.toml default features
- [ ] Test builds without `vm-backend` feature
- [ ] Update handler tests for both feature variants

---

## MEDIUM-TERM IMPROVEMENTS (1 Month)

### 6. Separate CQRS Services

**Current Problem**:
```rust
pub struct JobLifecycleService {
    commands: Arc<JobCommandService>,
    queries: Arc<JobQueryService>,
}

// This couples reads and writes even though they're separate concerns
```

**Proposed Solution: Distinct service tiers**

```rust
// Layer 1: Command only
pub struct JobMutationService {
    repo: Arc<dyn WorkRepository>,
    event_bus: Arc<dyn EventBus>,
}

// Layer 2: Query only
pub struct JobReadService {
    repo: Arc<dyn WorkRepository>,
}

// Layer 3: Orchestration (optional, for complex workflows)
pub struct JobOrchestrationService {
    mutations: Arc<JobMutationService>,
    reads: Arc<JobReadService>,
}

// In handlers: use appropriate service
pub async fn handle_submit(State(state): State<QueueHandlerState>) {
    let job_id = state.mutations.submit_job(...).await?;  // Write
}

pub async fn handle_status(State(state): State<QueueHandlerState>) {
    let job = state.reads.find_job(&job_id).await?;  // Read
}
```

**Benefits**:
- Clear separation of read/write semantics
- Easier to implement caching on reads
- Simpler to test read and write paths separately
- Better for eventual consistency patterns

---

### 7. Consolidate Overlapping Services

**Current Structure**:
```
ClusterStatusService ─ reads state + work repos
HealthService ───────── reads state repo
JobQueryService ────── reads work repo
```

**Proposed Consolidation**:
```
ClusterDataService (single source for cluster data)
├─ query_cluster_health() → ClusterHealth
├─ query_worker_stats() → WorkerStats
├─ query_job_status() → JobStatus
└─ query_queue_depth() → QueueStats

HealthReportService (transforms raw data)
├─ assess_overall_health() → HealthReport
├─ detect_bottlenecks() → BottleneckAnalysis
└─ recommend_scaling() → ScalingRecommendation
```

**Implementation Steps**:
1. Create unified `ClusterDataService`
2. Refactor `ClusterStatusService` to use new service
3. Merge `HealthService` health check logic
4. Remove redundant repository accesses

---

## ARCHITECTURE IMPROVEMENTS (Ongoing)

### 8. Synchronization Guarantees for VmManager

**Current Issue**:
```rust
pub struct VmManager {
    pub registry: Arc<VmRegistry>,  // ← No documented sync guarantees
    pub controller: Arc<VmController>,
    pub monitor: Arc<ResourceMonitor>,
}
```

**Problem**: If multiple handlers access VmManager simultaneously:
- VmRegistry writes VM state
- Monitor reads resource state
- Controller updates VM lifecycle
- No clear ordering guarantee

**Solution: Add synchronization layer**

```rust
pub struct VmManagerLock {
    vm_manager: Arc<VmManager>,
    // Per-VM locks for fine-grained control
    vm_locks: Arc<DashMap<String, Arc<RwLock<()>>>>,
}

impl VmManagerLock {
    pub async fn execute_job_on_vm(
        &self,
        vm_id: &str,
        job: Job,
    ) -> Result<JobResult> {
        // Acquire VM-specific lock
        let lock = self.vm_locks
            .entry(vm_id.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(())))
            .clone();
        
        let _guard = lock.write().await;
        
        // Now execute with guarantee of exclusive access
        self.vm_manager.controller.execute(vm_id, job).await
    }
}
```

**Documentation Requirements**:
- [ ] Document VmManager thread-safety guarantees
- [ ] List which operations require external synchronization
- [ ] Provide example patterns for safe concurrent access
- [ ] Add lock-free analysis for perf-critical paths

---

### 9. Module Documentation Standard

**Create module-level docs** with dependency maps:

```rust
// src/domain/job_commands.rs

//! Job command service - handle mutations with CQRS pattern
//!
//! # Dependencies
//!
//! Depends on:
//! - [`WorkRepository`] - for persisting job state
//! - [`EventBus`] - for publishing domain events
//! - [`JobStateMachine`] - for state transition validation
//!
//! Depended on by:
//! - [`JobLifecycleService`] - orchestrator
//! - All HTTP handlers for job submission
//!
//! # Coupling Notes
//!
//! This service publishes events asynchronously (fire-and-forget).
//! Event handlers MUST NOT access WriteRepositories to prevent
//! circular coupling. See [`crate::domain::event_handlers`] for
//! handler rules.
//!
//! # Testing
//!
//! Use [`crate::repositories::mocks::MockWorkRepository`] to test
//! in isolation. Events are published to [`MockEventBus`].

pub struct JobCommandService { ... }
```

**Standard sections**:
- What this module does
- What it depends on (with links)
- What depends on it
- Known coupling issues
- Testing guidance

---

## Testing Strategy Improvements

### Unit Test Isolation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::*;
    
    #[tokio::test]
    async fn test_submit_job_isolation() {
        // NO AppState setup
        // NO HiqliteService startup
        // NO IrohService connection
        
        let repo = Arc::new(MockWorkRepository::new());
        let event_bus = Arc::new(MockEventBus::new());
        
        let service = JobCommandService::new(repo.clone(), event_bus.clone());
        let result = service.submit_job(submission).await;
        
        assert!(result.is_ok());
        assert_eq!(repo.published_count(), 1);
        assert_eq!(event_bus.events_published(), 1);
    }
}
```

### Integration Test Matrix

```rust
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    async fn test_job_lifecycle_with_real_services() {
        // Uses real HiqliteService, WorkQueue, etc.
        // Tests integration between services
    }
    
    #[tokio::test]
    #[cfg(feature = "vm-backend")]
    async fn test_vm_execution() {
        // Only runs if vm-backend enabled
    }
    
    #[tokio::test]
    #[cfg(not(feature = "vm-backend"))]
    async fn test_non_vm_execution() {
        // Different test for non-VM builds
    }
}
```

---

## Implementation Timeline

```
Week 1-2:
├─ Trait-ify VmManager
├─ Extract AppState components
└─ Begin Trait-ify HiqliteService

Week 3-4:
├─ Decouple EventPublisher
├─ Feature-gate VM code
└─ Separate CQRS services

Week 5-6:
├─ Consolidate services
├─ Add sync guarantees
└─ Module documentation

Ongoing:
├─ Testing improvements
├─ Perf monitoring
└─ Refactoring iterations
```

---

## Validation Checklist

After completing refactoring:

- [ ] `cargo check` passes with all feature combinations
- [ ] `cargo nextest run` passes (unit tests)
- [ ] `cargo test --test '*'` passes (integration tests)
- [ ] No circular dependency warnings
- [ ] Handlers can be tested without full AppState
- [ ] VmService can be tested without real VmManager
- [ ] Domain services don't import infrastructure modules
- [ ] All service dependencies documented
- [ ] Event handler rules enforced (no repo writes)
- [ ] Feature-flag code paths tested separately

