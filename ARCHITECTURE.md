# Architecture Documentation

This document describes the architectural patterns, design decisions, and best practices for the mvm-ci codebase.

## Overview

mvm-ci follows a **Layered Hexagonal Architecture** (also known as Ports and Adapters), emphasizing:

- **Separation of concerns** - Each layer has a distinct responsibility
- **Dependency inversion** - Domain logic depends on abstractions, not concrete implementations
- **Testability** - Business logic can be tested without infrastructure
- **Protocol independence** - Domain logic is decoupled from HTTP, databases, and networking

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     HANDLERS (HTTP/REST)                      │
│  ┌────────────────┐           ┌───────────────────────────┐ │
│  │ dashboard.rs   │           │ queue.rs                  │ │
│  │ - Thin HTTP    │           │ - REST endpoints          │ │
│  │ - Extract data │           │ - JSON responses          │ │
│  │ - Call domain  │           │ - Status codes            │ │
│  └────────┬───────┘           └────────────┬──────────────┘ │
└───────────┼──────────────────────────────────┼───────────────┘
            │                                  │
            ▼                                  ▼
┌─────────────────────────────────────────────────────────────┐
│                    DOMAIN SERVICES                            │
│  ┌──────────────────────────┐  ┌──────────────────────────┐ │
│  │ ClusterStatusService     │  │ JobLifecycleService      │ │
│  │ - Business logic         │  │ - Validation             │ │
│  │ - Orchestration          │  │ - State transitions      │ │
│  │ - Uses repositories      │  │ - Aggregation            │ │
│  └────────┬─────────────────┘  └─────────┬────────────────┘ │
└───────────┼──────────────────────────────┼───────────────────┘
            │                               │
            ▼                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    REPOSITORIES (Traits)                      │
│  ┌──────────────────────────┐  ┌──────────────────────────┐ │
│  │ StateRepository          │  │ WorkRepository           │ │
│  │ - health_check()         │  │ - publish_work()         │ │
│  │ (abstracts Hiqlite)      │  │ - claim_work()           │ │
│  └────────┬─────────────────┘  └─────────┬────────────────┘ │
└───────────┼──────────────────────────────┼───────────────────┘
            │                               │
            ▼                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   INFRASTRUCTURE                              │
│  ┌──────────────────────────┐  ┌──────────────────────────┐ │
│  │ HiqliteService           │  │ WorkQueue                │ │
│  │ IrohService              │  │ FlawlessModule           │ │
│  │ (Concrete implementations)│  │ (External dependencies)  │ │
│  └──────────────────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Architectural Layers

### 1. Handlers Layer (`src/handlers/`)

**Purpose:** Protocol adapters that translate HTTP requests to domain operations

**Responsibilities:**
- Extract and validate request parameters (query strings, JSON, forms)
- Call domain services with validated data
- Transform domain results into HTTP responses (HTML, JSON, status codes)
- Handle protocol-specific concerns (HTMX partials, content negotiation)

**Rules:**
- Must be thin (no business logic)
- One handler per route
- Use view models for HTML templates
- Depend only on domain services

**Example:** Dashboard health endpoint

```rust
// src/handlers/dashboard.rs
pub async fn dashboard_cluster_health(State(state): State<AppState>) -> impl IntoResponse {
    let cluster_service = state.services().cluster_status();

    match cluster_service.get_cluster_health().await {
        Ok(health) => {
            let view = ClusterHealthView::from(health);
            Html(view.render().expect("cluster_health renders"))
        }
        Err(e) => {
            tracing::error!("Failed to get cluster health: {}", e);
            let error = ErrorView::new("Error loading health status");
            Html(format!("<h2>Cluster Health</h2>{}", error.render().expect("error renders")))
        }
    }
}
```

**Key insight:** The handler doesn't know about Hiqlite, WorkQueue, or infrastructure. It only knows about `ClusterStatusService`.

### 2. Domain Services Layer (`src/domain/`)

**Purpose:** Business logic and orchestration layer

**Responsibilities:**
- Implement use cases and business operations
- Enforce business rules and validation
- Orchestrate multiple repository calls
- Compute derived values and aggregations
- Maintain domain state machines

**Rules:**
- Depend only on repository traits (never concrete infrastructure)
- Accept domain types as input (`JobSubmission`, `JobSortOrder`)
- Return domain types as output (`EnrichedJob`, `ClusterHealth`)
- No HTTP knowledge (no status codes, headers, HTML)
- No direct database or network access

**Example:** Job lifecycle service

```rust
// src/domain/job_lifecycle.rs
pub struct JobLifecycleService {
    work_repo: Arc<dyn WorkRepository>,
    job_counter: Arc<AtomicUsize>,
}

impl JobLifecycleService {
    pub async fn submit_job(&self, submission: JobSubmission) -> Result<String> {
        // Validation (business rule)
        if submission.url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        // Generate unique ID (domain logic)
        let id = self.job_counter.fetch_add(1, Ordering::SeqCst);
        let job_id = format!("job-{}", id);

        // Create payload
        let payload = serde_json::json!({
            "id": id,
            "url": submission.url,
        });

        // Publish via repository abstraction
        self.work_repo.publish_work(job_id.clone(), payload).await?;

        tracing::info!(job_id = %job_id, url = %submission.url, "Job submitted");
        Ok(job_id)
    }
}
```

**Key insight:** The service uses `WorkRepository` trait, not `WorkQueue` concrete type. This enables mocking for tests.

### 3. Repositories Layer (`src/repositories/`)

**Purpose:** Data access abstractions that hide infrastructure details

**Responsibilities:**
- Define trait interfaces for data operations
- Provide concrete implementations wrapping infrastructure
- Translate between domain types and infrastructure types
- Isolate infrastructure concerns from domain logic

**Rules:**
- Traits must be `Send + Sync` and use `#[async_trait]`
- One trait per data concern (single responsibility)
- Implementations are thin wrappers (no business logic)
- Provide mock implementations for testing

**Example:** State repository trait

```rust
// src/repositories/mod.rs
#[async_trait]
pub trait StateRepository: Send + Sync {
    /// Get cluster health information
    async fn health_check(&self) -> Result<ClusterHealth>;
}

// Concrete implementation
pub struct HiqliteStateRepository {
    hiqlite: Arc<HiqliteService>,
}

#[async_trait]
impl StateRepository for HiqliteStateRepository {
    async fn health_check(&self) -> Result<ClusterHealth> {
        self.hiqlite.health_check().await
    }
}
```

**Key insight:** Domain services depend on `StateRepository` trait, allowing tests to inject `MockStateRepository`.

### 4. Infrastructure Layer

**Purpose:** Concrete implementations of external systems and services

**Components:**
- **HiqliteService** (`src/hiqlite_service.rs`) - Distributed SQL database
- **WorkQueue** (`src/work_queue.rs`) - Distributed job queue over iroh-gossip
- **IrohService** (`src/iroh_service.rs`) - P2P networking and blob storage
- **FlawlessModule** - WASM runtime module

**Rules:**
- No business logic (pure technical concerns)
- Implement service traits for consistency
- Handle connection lifecycle and errors
- Provide observability (logging, metrics)

**Example:** Infrastructure state container

```rust
// src/state/infrastructure.rs
pub struct InfrastructureState {
    pub(crate) module: Arc<DeployedModule>,
    pub(crate) iroh: IrohService,
    pub(crate) hiqlite: HiqliteService,
    pub(crate) work_queue: WorkQueue,
}
```

## Design Patterns

### Repository Pattern

The repository pattern abstracts data access behind trait interfaces, enabling:
- **Testability** - Mock implementations for unit tests
- **Flexibility** - Swap implementations without changing domain logic
- **Decoupling** - Domain logic doesn't depend on databases

**Trait definition:**

```rust
#[async_trait]
pub trait WorkRepository: Send + Sync {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()>;
    async fn claim_work(&self) -> Result<Option<WorkItem>>;
    async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()>;
    async fn list_work(&self) -> Result<Vec<WorkItem>>;
    async fn stats(&self) -> WorkQueueStats;
}
```

**Usage in domain service:**

```rust
pub struct JobLifecycleService {
    work_repo: Arc<dyn WorkRepository>,  // Depends on trait, not concrete type
    job_counter: Arc<AtomicUsize>,
}
```

**Mock implementation for tests:**

```rust
pub struct MockWorkRepository {
    work_items: Arc<Mutex<Vec<WorkItem>>>,
    stats: Arc<Mutex<WorkQueueStats>>,
}

#[async_trait]
impl WorkRepository for MockWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        // In-memory implementation for testing
        let work_item = WorkItem {
            job_id,
            status: WorkStatus::Pending,
            payload,
            // ...
        };
        self.work_items.lock().await.push(work_item);
        Ok(())
    }
    // ... other methods
}
```

### Service Trait Pattern

Service traits provide **focused capabilities** rather than monolithic interfaces:

```rust
// Separate traits for distinct capabilities
pub trait EndpointInfo: Send + Sync {
    fn endpoint_id(&self) -> EndpointId;
    fn endpoint_addr(&self) -> EndpointAddr;
}

pub trait BlobStorage: Send + Sync {
    async fn store_blob(&self, data: Bytes) -> Result<String>;
    async fn retrieve_blob(&self, hash: &str) -> Result<Bytes>;
}

pub trait DatabaseHealth: Send + Sync {
    async fn health_check(&self) -> Result<ClusterHealth>;
}
```

**Benefits:**
- Mock implementations only need to implement what they use
- Clear documentation of capabilities
- Single Responsibility Principle
- Easier to understand than large interfaces

**Usage:**

```rust
// Infrastructure state provides trait views
impl InfrastructureState {
    pub fn iroh_blobs(&self) -> &dyn BlobStorage {
        &self.iroh
    }

    pub fn db_health(&self) -> &dyn DatabaseHealth {
        &self.hiqlite
    }
}
```

### View Model Pattern

View models decouple domain models from HTML templates:

```rust
// Domain model (business logic)
pub struct ClusterHealth {
    pub is_healthy: bool,
    pub node_count: usize,
    pub has_leader: bool,
    pub active_worker_count: usize,
}

// View model (presentation logic)
#[derive(Template)]
#[template(path = "cluster_health.html")]
pub struct ClusterHealthView {
    pub status_text: String,
    pub status_class: String,  // CSS class: "healthy", "degraded", "unhealthy"
    pub node_count: usize,
    pub worker_count: usize,
}

impl From<ClusterHealth> for ClusterHealthView {
    fn from(health: ClusterHealth) -> Self {
        let (status_text, status_class) = if health.is_healthy && health.has_leader {
            ("Healthy", "healthy")
        } else if health.node_count > 0 {
            ("Degraded", "degraded")
        } else {
            ("Unhealthy", "unhealthy")
        };

        Self {
            status_text: status_text.to_string(),
            status_class: status_class.to_string(),
            node_count: health.node_count,
            worker_count: health.active_worker_count,
        }
    }
}
```

**Benefits:**
- Handlers don't build HTML strings
- Presentation logic is testable
- Templates are type-safe (Askama)
- Clear separation of concerns

### Dependency Injection via State Container

Application state is composed of focused containers:

```rust
// Root state
pub struct AppState {
    infrastructure: Arc<InfrastructureState>,
    services: Arc<DomainServices>,
}

// Infrastructure container
pub struct InfrastructureState {
    module: Arc<DeployedModule>,
    iroh: IrohService,
    hiqlite: HiqliteService,
    work_queue: WorkQueue,
}

// Domain services container (dependency injection)
pub struct DomainServices {
    cluster_status: Arc<ClusterStatusService>,
    job_lifecycle: Arc<JobLifecycleService>,
}

impl DomainServices {
    pub fn new(infra: &InfrastructureState) -> Self {
        // Create repository implementations
        let state_repo = Arc::new(HiqliteStateRepository::new(
            Arc::new(infra.hiqlite().clone())
        ));
        let work_repo = Arc::new(WorkQueueWorkRepository::new(infra.work_queue().clone()));

        // Inject dependencies into domain services
        let cluster_status = Arc::new(ClusterStatusService::new(
            state_repo.clone(),
            work_repo.clone(),
        ));

        let job_lifecycle = Arc::new(JobLifecycleService::new(work_repo));

        Self { cluster_status, job_lifecycle }
    }
}
```

**Usage in main.rs:**

```rust
// Infrastructure initialized first
let state = AppState::from_infrastructure(module, iroh_service, hiqlite_service, work_queue);

// Passed to server
let handle = server::start(ServerConfig {
    app_config: config,
    endpoint,
    state,
}).await?;
```

## Architectural Rules

### DO: Follow the Dependency Flow

```rust
// ✅ CORRECT: Handler → Domain Service → Repository → Infrastructure
pub async fn dashboard_recent_jobs(
    State(state): State<AppState>,
    Query(query): Query<SortQuery>,
) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();  // Get domain service
    let sort_order = JobSortOrder::from_str(query.sort.as_deref().unwrap_or("time"));

    match job_service.list_jobs(sort_order, 20).await {  // Call domain service
        Ok(jobs) => {
            let view = JobListView::new(jobs);
            Html(view.render().expect("job_list renders"))
        }
        Err(e) => {
            let error = ErrorView::new("Error loading jobs");
            Html(error.render().expect("error renders"))
        }
    }
}
```

### DON'T: Bypass the Domain Layer

```rust
// ❌ WRONG: Handler directly accessing infrastructure
pub async fn dashboard_recent_jobs(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // DON'T DO THIS - bypasses domain logic!
    let work_items = state.infrastructure().work_queue().list_work().await?;
    // Handler now contains business logic (sorting, filtering, enrichment)
    // This makes the logic untestable and violates separation of concerns
}
```

### DO: Use Repository Traits in Domain Services

```rust
// ✅ CORRECT: Domain service depends on trait
pub struct ClusterStatusService {
    state_repo: Arc<dyn StateRepository>,   // Trait, not concrete type
    work_repo: Arc<dyn WorkRepository>,     // Trait, not concrete type
}

impl ClusterStatusService {
    pub fn new(state_repo: Arc<dyn StateRepository>, work_repo: Arc<dyn WorkRepository>) -> Self {
        Self { state_repo, work_repo }
    }
}
```

### DON'T: Depend on Concrete Infrastructure

```rust
// ❌ WRONG: Domain service depends on concrete types
pub struct ClusterStatusService {
    hiqlite: Arc<HiqliteService>,      // Concrete infrastructure
    work_queue: Arc<WorkQueue>,        // Concrete infrastructure
}

// This prevents testing with mocks and creates tight coupling
```

### DO: Keep Handlers Thin

```rust
// ✅ CORRECT: Thin handler, all logic in domain service
pub async fn dashboard_submit_job(
    State(state): State<AppState>,
    Form(job): Form<NewJob>,
) -> Response {
    let job_service = state.services().job_lifecycle();
    let submission = JobSubmission { url: job.url };

    match job_service.submit_job(submission).await {
        Ok(job_id) => {
            tracing::info!(job_id = %job_id, "Job submitted from dashboard");
            dashboard_recent_jobs(State(state), Query(SortQuery { sort: Some("time".to_string()) }))
                .await
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to submit job: {}", e);
            let error = ErrorView::new(format!("Error: {}", e));
            Html(error.render().expect("error renders")).into_response()
        }
    }
}
```

### DON'T: Put Business Logic in Handlers

```rust
// ❌ WRONG: Handler contains validation and business logic
pub async fn dashboard_submit_job(
    State(state): State<AppState>,
    Form(job): Form<NewJob>,
) -> Response {
    // DON'T DO THIS - validation belongs in domain service
    if job.url.is_empty() {
        return (StatusCode::BAD_REQUEST, "URL cannot be empty").into_response();
    }

    // DON'T DO THIS - ID generation belongs in domain service
    let job_id = format!("job-{}", uuid::Uuid::new_v4());

    // DON'T DO THIS - direct infrastructure access
    state.infrastructure().work_queue().publish_work(job_id, payload).await?;
}
```

### DO: Test Domain Logic with Mocks

```rust
// ✅ CORRECT: Unit test with mock repositories
#[tokio::test]
async fn test_submit_job_validation() {
    let work_repo = Arc::new(MockWorkRepository::new());
    let service = JobLifecycleService::new(work_repo);

    // Test validation logic without infrastructure
    let submission = JobSubmission { url: "".to_string() };
    let result = service.submit_job(submission).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("URL cannot be empty"));
}
```

### DON'T: Test Business Logic with Real Infrastructure

```rust
// ❌ WRONG: Integration test pretending to be unit test
#[tokio::test]
async fn test_submit_job_validation() {
    // DON'T DO THIS - requires real database, network, etc.
    let hiqlite = HiqliteService::new("./test-data").await.unwrap();
    let work_queue = WorkQueue::new(endpoint, node_id, store).await.unwrap();
    let work_repo = Arc::new(WorkQueueWorkRepository::new(work_queue));

    let service = JobLifecycleService::new(work_repo);
    // Test is now slow, flaky, and requires external dependencies
}
```

## Testing Strategy

### Unit Testing (Fast, Isolated)

**Purpose:** Test domain logic without infrastructure

**Approach:**
- Use mock repositories implementing the trait interfaces
- Test business rules, validation, and state transitions
- No database, network, or file system access
- Fast execution (milliseconds)

**Example:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::{MockStateRepository, MockWorkRepository};

    #[tokio::test]
    async fn test_cluster_health_aggregation() {
        // Arrange: Create mocks
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        state_repo.set_health(ClusterHealth {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        work_repo.add_work_items(vec![
            WorkItem {
                job_id: "job-1".to_string(),
                status: WorkStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                // ...
            },
        ]).await;

        // Act: Call domain service
        let service = ClusterStatusService::new(state_repo, work_repo);
        let health = service.get_cluster_health().await.unwrap();

        // Assert: Verify business logic
        assert_eq!(health.is_healthy, true);
        assert_eq!(health.node_count, 3);
        assert_eq!(health.active_worker_count, 1);
    }
}
```

### Integration Testing (Full Stack)

**Purpose:** Test complete request flow with real infrastructure

**Approach:**
- Use real implementations (or test doubles of infrastructure)
- Test handler → domain → repository → infrastructure flow
- Verify protocol concerns (HTTP status codes, JSON structure)
- Slower execution (seconds)

**Example:**

```rust
#[tokio::test]
async fn test_job_submission_integration() {
    // Arrange: Real infrastructure in test mode
    let hiqlite = HiqliteService::new(temp_dir()).await.unwrap();
    let work_queue = WorkQueue::new_test_mode().await.unwrap();
    let state = AppState::from_infrastructure(module, iroh, hiqlite, work_queue);

    // Act: Call handler through HTTP
    let app = router::build_router(&state);
    let response = app
        .oneshot(Request::builder()
            .uri("/api/queue/submit")
            .method("POST")
            .body(Body::from(r#"{"url": "https://example.com"}"#))
            .unwrap())
        .await
        .unwrap();

    // Assert: Verify HTTP response
    assert_eq!(response.status(), StatusCode::OK);

    // Verify database state
    let work_items = work_queue.list_work().await.unwrap();
    assert_eq!(work_items.len(), 1);
    assert_eq!(work_items[0].status, WorkStatus::Pending);
}
```

### Test Doubles Location

- **Mock Repositories:** `src/repositories/mocks.rs`
- **Mock Services:** `src/services/mocks.rs` (if needed)

## Adding New Features

Follow this step-by-step process to add new features while maintaining architectural consistency.

### Example: Adding "Cancel Job" Feature

#### Step 1: Define repository trait method (if needed)

```rust
// src/repositories/mod.rs
#[async_trait]
pub trait WorkRepository: Send + Sync {
    // Existing methods...

    /// Cancel a pending or claimed job
    async fn cancel_work(&self, job_id: &str) -> Result<()>;
}
```

#### Step 2: Implement use case in domain service

```rust
// src/domain/job_lifecycle.rs
impl JobLifecycleService {
    /// Cancel a job if it hasn't started processing
    pub async fn cancel_job(&self, job_id: String) -> Result<()> {
        // Business rule: Can only cancel pending or claimed jobs
        let work_items = self.work_repo.list_work().await?;
        let job = work_items.iter().find(|item| item.job_id == job_id)
            .ok_or_else(|| anyhow::anyhow!("Job not found: {}", job_id))?;

        match job.status {
            WorkStatus::Pending | WorkStatus::Claimed => {
                // Valid cancellation
                self.work_repo.update_status(&job_id, WorkStatus::Cancelled).await?;
                tracing::info!(job_id = %job_id, "Job cancelled");
                Ok(())
            }
            WorkStatus::InProgress | WorkStatus::Completed => {
                // Business rule violation
                anyhow::bail!("Cannot cancel job in {} state", job.status)
            }
            WorkStatus::Cancelled => {
                // Idempotent - already cancelled
                Ok(())
            }
        }
    }
}
```

#### Step 3: Add thin handler

```rust
// src/handlers/queue.rs
#[derive(Deserialize)]
pub struct CancelJobRequest {
    pub job_id: String,
}

pub async fn cancel_job(
    State(state): State<AppState>,
    Json(req): Json<CancelJobRequest>,
) -> impl IntoResponse {
    let job_service = state.services().job_lifecycle();

    match job_service.cancel_job(req.job_id).await {
        Ok(()) => (StatusCode::OK, "Job cancelled").into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Failed to cancel job: {}", e),
        ).into_response(),
    }
}
```

#### Step 4: Wire up the route

```rust
// src/server/router.rs
pub fn build_router(state: &AppState) -> Router {
    Router::new()
        // Existing routes...
        .route("/api/queue/cancel", post(queue::cancel_job))
        .with_state(state.clone())
}
```

#### Step 5: Write unit tests

```rust
// src/domain/job_lifecycle.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::MockWorkRepository;

    #[tokio::test]
    async fn test_cancel_pending_job_succeeds() {
        let work_repo = Arc::new(MockWorkRepository::new());
        work_repo.add_work_items(vec![
            WorkItem {
                job_id: "job-1".to_string(),
                status: WorkStatus::Pending,
                // ...
            },
        ]).await;

        let service = JobLifecycleService::new(work_repo.clone());
        let result = service.cancel_job("job-1".to_string()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_in_progress_job_fails() {
        let work_repo = Arc::new(MockWorkRepository::new());
        work_repo.add_work_items(vec![
            WorkItem {
                job_id: "job-1".to_string(),
                status: WorkStatus::InProgress,
                // ...
            },
        ]).await;

        let service = JobLifecycleService::new(work_repo.clone());
        let result = service.cancel_job("job-1".to_string()).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot cancel"));
    }
}
```

#### Step 6: Write integration test

```rust
// tests/integration/job_lifecycle.rs
#[tokio::test]
async fn test_cancel_job_integration() {
    let state = setup_test_state().await;
    let app = router::build_router(&state);

    // Submit job
    app.oneshot(Request::post("/api/queue/submit")
        .body(r#"{"url": "https://example.com"}"#)
        .unwrap())
        .await
        .unwrap();

    // Cancel job
    let response = app.oneshot(Request::post("/api/queue/cancel")
        .body(r#"{"job_id": "job-1"}"#)
        .unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

## Module Organization

```
src/
├── bin/
│   └── worker.rs              # Worker binary (separate from control plane)
├── config.rs                  # Centralized configuration (AppConfig)
├── domain/                    # Business logic layer
│   ├── mod.rs                 # Public API and re-exports
│   ├── cluster_status.rs      # Cluster monitoring use cases
│   └── job_lifecycle.rs       # Job management use cases
├── handlers/                  # HTTP request handlers
│   ├── dashboard.rs           # Web UI endpoints (HTMX)
│   └── queue.rs               # REST API endpoints (JSON)
├── repositories/              # Data access abstractions
│   ├── mod.rs                 # Repository traits
│   ├── hiqlite_repository.rs  # StateRepository implementation
│   ├── work_queue_repository.rs # WorkRepository implementation
│   └── mocks.rs               # Mock implementations for testing
├── server/                    # Server lifecycle management
│   ├── mod.rs                 # Public API (start function)
│   ├── router.rs              # Route definitions
│   ├── localhost.rs           # Localhost HTTP listener
│   ├── iroh.rs                # Iroh+H3 P2P listener
│   └── lifecycle.rs           # Graceful shutdown coordination
├── services/                  # Service trait abstractions
│   ├── mod.rs                 # Service implementations
│   └── traits.rs              # Service trait definitions
├── state/                     # State management
│   ├── mod.rs                 # AppState root container
│   ├── infrastructure.rs      # InfrastructureState container
│   └── services.rs            # DomainServices container (DI)
├── views/                     # View models for HTML templates
│   ├── mod.rs                 # Public API and re-exports
│   ├── cluster_health.rs      # Cluster health view
│   ├── job_list.rs            # Job list view
│   └── ...                    # Other view models
├── hiqlite_service.rs         # Distributed SQL database service
├── iroh_service.rs            # P2P networking service
├── work_queue.rs              # Distributed job queue
└── main.rs                    # Application entry point
```

### Key Module Responsibilities

- **domain/** - Pure business logic, no infrastructure dependencies
- **handlers/** - HTTP protocol adapters, thin glue code
- **repositories/** - Abstract data access behind traits
- **server/** - Lifecycle management (startup, shutdown, dual listeners)
- **services/** - Service trait definitions for infrastructure
- **state/** - Dependency injection and state composition
- **views/** - Presentation layer (domain → HTML)

## Current Domain Services

### ClusterStatusService

**Purpose:** Aggregate cluster health and worker statistics

**Key Methods:**
- `get_cluster_health()` - Overall cluster status (nodes, workers, leader)
- `get_worker_stats()` - Per-worker job counts and activity
- `get_control_plane_nodes()` - Control plane node information

**Dependencies:**
- `StateRepository` (for Hiqlite cluster health)
- `WorkRepository` (for job-based worker inference)

**Example Usage:**

```rust
let cluster_service = state.services().cluster_status();
let health = cluster_service.get_cluster_health().await?;
println!("Cluster has {} nodes and {} workers", health.node_count, health.active_worker_count);
```

### JobLifecycleService

**Purpose:** Job submission, querying, and status management

**Key Methods:**
- `submit_job(submission)` - Create and queue new job
- `list_jobs(sort_order, limit)` - Query jobs with sorting and enrichment
- `get_queue_stats()` - Aggregate statistics (pending, completed, failed)

**Dependencies:**
- `WorkRepository` (for job queue operations)

**Example Usage:**

```rust
let job_service = state.services().job_lifecycle();
let submission = JobSubmission { url: "https://example.com".to_string() };
let job_id = job_service.submit_job(submission).await?;
println!("Created job: {}", job_id);
```

## Configuration

### AppConfig Structure

```rust
pub struct AppConfig {
    pub network: NetworkConfig,    // HTTP port, bind address, ALPN
    pub storage: StorageConfig,    // Paths for iroh blobs, hiqlite data
    pub flawless: FlawlessConfig,  // Flawless WASM runtime URL
    pub timing: TimingConfig,      // Timeouts and delays
}
```

### Loading Configuration

```rust
// In main.rs
let config = AppConfig::load().expect("Failed to load configuration");
```

### Environment Variables

- `HTTP_PORT` - HTTP server port (default: 3020)
- `IROH_BLOBS_PATH` - Iroh blob storage path (default: ./data/iroh-blobs)
- `HQL_DATA_DIR` - Hiqlite data directory (default: ./data/hiqlite)
- `FLAWLESS_URL` - Flawless server URL (default: http://localhost:27288)
- `WORKER_NO_WORK_SLEEP_SECS` - Worker sleep duration when queue empty (default: 2)
- `WORKER_ERROR_SLEEP_SECS` - Worker sleep duration on error (default: 5)
- `HIQLITE_STARTUP_DELAY_SECS` - Initial delay after Hiqlite start (default: 3)
- `CONTROL_PLANE_TICKET` - Worker connection ticket (iroh+h3 URL)

### Validation

All configuration values are validated at load time:

```rust
let config = AppConfig::load()?; // Returns ConfigError if invalid
```

### Test-Friendly Defaults

```rust
// For tests, use defaults without environment variables
let config = AppConfig::default();
```

## Dependency Flow

The architecture enforces a strict dependency flow to prevent circular dependencies and maintain clean boundaries:

```
main.rs
  ↓
server::start(ServerConfig)
  ↓
router::build_router(AppState)
  ↓
handlers (dashboard.rs, queue.rs)
  ↓
domain services (ClusterStatusService, JobLifecycleService)
  ↓
repositories (StateRepository, WorkRepository)
  ↓
infrastructure (HiqliteService, WorkQueue, IrohService)
```

### Key Principles

1. **Layers only depend on layers below them** - Handlers depend on domain, domain depends on repositories
2. **No upward dependencies** - Infrastructure never depends on domain or handlers
3. **No layer skipping** - Handlers must go through domain, cannot access infrastructure directly
4. **Abstractions point downward** - Traits defined at the level that needs them (repository traits in repositories/)

### Initialization Order (main.rs)

```rust
// 1. Load configuration
let config = AppConfig::load()?;

// 2. Initialize infrastructure services
let hiqlite_service = HiqliteService::new(config.storage.hiqlite_data_dir).await?;
let endpoint = iroh::Endpoint::builder().bind().await?;
let iroh_service = IrohService::new(config.storage.iroh_blobs_path, endpoint);
let work_queue = WorkQueue::new(endpoint, node_id, persistent_store).await?;

// 3. Compose state with dependency injection
let state = AppState::from_infrastructure(module, iroh_service, hiqlite_service, work_queue);
// Inside: Creates repositories, injects into domain services

// 4. Start server with composed state
let handle = server::start(ServerConfig { config, endpoint, state }).await?;
handle.run().await?;
```

## Event-Driven Architecture ✅ IMPLEMENTED

**Status:** Fully integrated in domain layer

The application now publishes domain events for all significant state changes, enabling:
- **Observability** - Structured logging of all job lifecycle events
- **Audit trails** - Complete history of state transitions
- **Metrics collection** - Foundation for monitoring dashboards
- **Future extensibility** - Easy to add webhooks, notifications, event sourcing

### Architecture

```
┌──────────────────────────────────────────────────────────┐
│          JobCommandService (Domain Service)              │
│  ┌────────────────────────────────────────────────────┐ │
│  │  1. Validate business rules                        │ │
│  │  2. Execute command (via repository)               │ │
│  │  3. Publish domain event ──────────────────────────┼─┼──> EventPublisher
│  └────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                                                             │
                    ┌────────────────────────────────────────┘
                    │
                    ▼
    ┌───────────────────────────────────────────┐
    │     EventPublisher Implementations        │
    ├───────────────────────────────────────────┤
    │ • LoggingEventPublisher (production)      │
    │ • InMemoryEventPublisher (testing)        │
    │ • NoOpEventPublisher (disabled)           │
    └───────────────────────────────────────────┘
```

### Domain Events

All events are defined in `src/domain/events.rs`:

- `JobSubmitted` - A new job was queued
- `JobClaimed` - A worker claimed a pending job
- `JobStatusChanged` - Generic status transition
- `JobCompleted` - Job finished successfully
- `JobFailed` - Job failed with error
- `JobCancelled` - Job was cancelled
- `JobRetried` - Failed job reset for retry

Events include:
- Job ID (for correlation)
- Timestamp (Unix seconds)
- Context-specific data (URL, worker ID, error, etc.)

### Event Publishers

**LoggingEventPublisher** (default for production):
```rust
// Automatically injected by DomainServices::from_repositories()
let event_publisher = Arc::new(LoggingEventPublisher::new());
```

Logs events at INFO level with structured fields:
```
[INFO] Domain event: Job job-123 submitted for URL https://example.com
       event_type=JobSubmitted job_id=job-123 timestamp=1704067200
```

**InMemoryEventPublisher** (for testing):
```rust
// Use in tests to verify events were published
let event_publisher = Arc::new(InMemoryEventPublisher::new());
let service = JobCommandService::with_events(repo, event_publisher.clone());

// ... perform operations ...

let events = event_publisher.get_events().await;
assert_eq!(events.len(), 3);
```

**NoOpEventPublisher** (disable events):
```rust
// For performance testing or when events aren't needed
let event_publisher = Arc::new(NoOpEventPublisher::new());
```

### Dependency Injection

Events are injected through the service layer:

```rust
// Production (automatic)
let services = DomainServices::from_repositories(state_repo, work_repo);
// Uses LoggingEventPublisher internally

// Testing (manual control)
let event_publisher = Arc::new(InMemoryEventPublisher::new());
let services = DomainServices::from_repositories_with_events(
    state_repo,
    work_repo,
    event_publisher,
);
```

### Usage Example

```rust
// Command service automatically publishes events
let commands = JobCommandService::with_events(work_repo, event_publisher);

// Submit job -> publishes JobSubmitted event
let job_id = commands.submit_job(JobSubmission {
    url: "https://example.com".to_string(),
}).await?;

// Claim job -> publishes JobClaimed event
commands.claim_job().await?;

// Update status -> publishes JobStatusChanged/JobCompleted/JobFailed
commands.update_job_status(&job_id, JobStatus::Completed).await?;
```

### Testing Events

See `src/domain/job_commands.rs` tests for comprehensive examples:

```rust
#[tokio::test]
async fn test_submit_job_publishes_job_submitted_event() {
    let mock_repo = Arc::new(MockWorkRepository::new());
    let event_publisher = Arc::new(InMemoryEventPublisher::new());
    let service = JobCommandService::with_events(mock_repo, event_publisher.clone());

    let job_id = service.submit_job(JobSubmission {
        url: "https://example.com".to_string(),
    }).await.unwrap();

    let events = event_publisher.get_events().await;
    assert_eq!(events.len(), 1);

    match &events[0] {
        DomainEvent::JobSubmitted { job_id: id, url, .. } => {
            assert_eq!(id, &job_id);
            assert_eq!(url, "https://example.com");
        }
        _ => panic!("Expected JobSubmitted event"),
    }
}
```

### Future Enhancements

The event infrastructure is now ready for:
- **External event streams** (Kafka, Redis pub/sub)
- **Webhook notifications** (notify external systems)
- **Event sourcing** (rebuild state from events)
- **Metrics dashboards** (aggregate event data)
- **Distributed tracing** (correlate across services)

## State Machine Validation ✅ IMPLEMENTED

**Status:** Fully integrated in domain layer

The application enforces strict state transition rules for jobs, preventing invalid operations like transitioning from Completed back to Pending. This ensures data integrity and makes business rules explicit and testable.

### State Machine Diagram

```
   Pending ───────> Claimed ───────> InProgress ───────> Completed ⊗
      │                │                  │
      │                │                  │
      └────────────────┴──────────────────┴────────────> Failed
                                                            │
                                                            └──────> Pending (retry)

Legend:
  → Valid forward transition
  ⊗ Terminal state (immutable)
```

### Business Rules

**Valid Transitions:**

| From | To | Rule |
|------|------|------|
| Pending | Claimed | Worker takes ownership |
| Claimed | InProgress | Worker starts execution |
| InProgress | Completed | Execution succeeds (terminal) |
| Pending | Failed | Validation failure before claiming |
| Claimed | Failed | Worker decides not to process |
| InProgress | Failed | Execution error |
| Failed | Pending | Reset for retry (only from Failed) |
| Any | Same | Idempotent updates allowed |

**Invalid Transitions:**

- **Completed → any other state** - Completed is immutable (terminal state)
- **Backward transitions** - Cannot unclaim or revert (e.g., Claimed → Pending)
- **Skip transitions** - Must follow proper flow (e.g., cannot go Pending → InProgress without Claimed)

### Implementation

The state machine is implemented as a pure stateless validator in `src/domain/state_machine.rs`:

```rust
use mvm_ci::domain::{JobStateMachine, JobStatus};

// Validate a transition
match JobStateMachine::validate_transition(JobStatus::Pending, JobStatus::Claimed) {
    Ok(()) => println!("Valid transition"),
    Err(e) => println!("Invalid: {}", e.reason),
}

// Check state properties
assert!(!JobStateMachine::is_terminal(JobStatus::Failed));  // Can retry
assert!(JobStateMachine::is_terminal(JobStatus::Completed));  // Immutable
assert!(JobStateMachine::is_retriable(JobStatus::Failed));
```

### Integration Points

**JobCommandService** validates all state transitions before executing:

```rust
// src/domain/job_commands.rs

pub async fn update_job_status(&self, job_id: &str, status: JobStatus) -> Result<()> {
    // Get current job
    let current_job = /* ... */;

    // Validate transition using business rules
    JobStateMachine::validate_transition(current_job.status, status)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Transition is valid - proceed
    self.work_repo.update_status(job_id, status).await?;
    // ... publish events ...
}
```

All command methods validate transitions:
- `update_job_status()` - General status updates
- `cancel_job()` - Validates can transition to Failed
- `retry_job()` - Validates job is Failed before resetting to Pending

### Error Handling

Invalid transitions return descriptive errors:

```rust
// Example error from attempting Completed → Pending:
StateTransitionError {
    from: JobStatus::Completed,
    to: JobStatus::Pending,
    reason: "Completed jobs are immutable (terminal state)"
}
```

These errors propagate to HTTP handlers with appropriate status codes (400 Bad Request).

### Testing

Comprehensive test coverage in `src/domain/state_machine.rs` (14 tests) and `src/domain/job_commands.rs` (11 integration tests):

**State Machine Tests:**
- Valid forward transitions (happy path)
- Failure transitions (can fail from any non-terminal state)
- Retry transition (Failed → Pending)
- Idempotent updates (same state transitions)
- Terminal state rejection (Completed → any)
- Invalid backward transitions
- Invalid skip transitions
- Invalid Failed transitions
- Helper functions (`is_terminal`, `is_retriable`, `requires_worker`)

**Integration Tests:**
- Reject invalid skip transitions (Pending → InProgress)
- Reject backward transitions (Claimed → Pending)
- Reject transitions from Completed (terminal)
- Reject cancelling completed jobs
- Reject retrying non-failed jobs
- Accept idempotent updates
- Accept retrying failed jobs
- Accept all valid failure transitions

### Benefits

**Data Integrity:**
- Prevents impossible states (e.g., Completed job becoming Pending)
- Ensures consistent job lifecycle across distributed system
- Catches logic errors at runtime before corrupting data

**Explicit Business Rules:**
- State transitions are self-documenting
- Rules are testable independently of infrastructure
- Easy to audit and verify compliance

**Better Error Messages:**
- Descriptive errors explain why transitions are invalid
- Helps debugging and operator understanding
- Enables better UX in frontend applications

### Future Enhancements

The state machine is ready for:
- **Conditional transitions** - Add context-based rules (e.g., only certain workers can claim jobs)
- **State metadata** - Track retry counts, transition history
- **Visual tooling** - Generate state diagrams from code
- **Audit logging** - Integrate with event system for compliance
- **Rate limiting** - Prevent too many retries

## Router Modularization ✅ IMPLEMENTED

**Status:** Fully implemented in server layer

The application now organizes routes into focused sub-routers, providing clear API boundaries and enabling route-specific middleware.

### Router Structure

```text
/
├── /dashboard/*       - HTMX monitoring UI (browser access)
│   ├── GET  /                     - Main dashboard page
│   ├── GET  /cluster-health       - Cluster health HTMX fragment
│   ├── GET  /queue-stats          - Queue statistics fragment
│   ├── GET  /recent-jobs          - Recent jobs list fragment
│   ├── GET  /control-plane-nodes  - Control plane nodes fragment
│   ├── GET  /workers              - Worker nodes fragment
│   └── POST /submit-job           - Submit new job via web UI
│
├── /api/queue/*       - Work queue REST API (worker access)
│   ├── POST /publish              - Submit new job to queue
│   ├── POST /claim                - Claim next available job
│   ├── GET  /list                 - List all jobs
│   ├── GET  /stats                - Get queue statistics
│   └── POST /status/{job_id}      - Update job status
│
├── /api/iroh/*        - P2P blob storage and gossip API
│   ├── POST /blob/store           - Store content-addressed blob
│   ├── GET  /blob/{hash}          - Retrieve blob by hash
│   ├── POST /gossip/join          - Join gossip topic
│   ├── POST /gossip/broadcast     - Broadcast message to topic
│   ├── GET  /gossip/subscribe/{topic_id} - Subscribe to topic (SSE)
│   ├── POST /connect              - Connect to peer
│   └── GET  /info                 - Get endpoint information
│
└── /health/*          - Health check endpoints
    └── GET  /hiqlite              - Hiqlite database health check
```

### Implementation

The router is organized in `src/server/router.rs` with focused sub-routers:

```rust
pub fn build_router(state: &AppState) -> Router {
    Router::new()
        .nest("/dashboard", dashboard_router())
        .nest("/api/queue", queue_api_router())
        .nest("/api/iroh", iroh_api_router())
        .nest("/health", health_router())
        .with_state(state.clone())
}

fn dashboard_router() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/cluster-health", get(dashboard_cluster_health))
        // ... other dashboard routes
        // Future: Add dashboard-specific middleware
        // .layer(/* HTMX headers, caching policy, etc. */)
}

fn queue_api_router() -> Router<AppState> {
    Router::new()
        .route("/publish", post(queue_publish))
        .route("/claim", post(queue_claim))
        // ... other queue API routes
        // Future: Add API-specific middleware
        // .layer(/* rate limiting, API versioning, auth, etc. */)
}
```

### Benefits

**Clear API Boundaries:**
- Logical grouping of related routes
- Self-describing URL structure (`/api/queue/claim` vs `/queue/claim`)
- Easy to understand what each route group does

**Route-Specific Middleware:**
- Apply middleware to entire route groups
- Dashboard: HTMX headers, caching policies
- Queue API: Rate limiting, authentication, versioning
- Health: No middleware (fast response)

**Easier Maintenance:**
- Add/remove routes within focused sub-routers
- Change middleware for entire API surface at once
- Reduces merge conflicts (changes isolated to sub-routers)

**Better Documentation:**
- Routes self-document through URL structure
- Each sub-router has comprehensive doc comments
- Clear separation between UI, API, and operations

### Middleware Ready

Each sub-router has placeholder comments for future middleware:

**Dashboard Router:**
```rust
// .layer(tower_http::set_header::SetRequestHeader::if_not_present(
//     header::HeaderName::from_static("hx-request"),
//     header::HeaderValue::from_static("true"),
// ))
// .layer(tower_http::compression::CompressionLayer::new())
```

**Queue API Router:**
```rust
// .layer(tower::limit::RateLimitLayer::new(100, Duration::from_secs(1)))
// .layer(tower_http::validate_request::ValidateRequestHeaderLayer::bearer("secret"))
// .layer(tower_http::set_header::SetResponseHeader::overriding(
//     header::CONTENT_TYPE,
//     HeaderValue::from_static("application/json"),
// ))
```

**Iroh API Router:**
```rust
// .layer(PeerAuthenticationLayer::new())
// .layer(EncryptionVerificationLayer::new())
```

**Health Router:**
```rust
// Keep minimal - no middleware for fastest health check response
```

### Migration Notes

**Route Changes:**

Old routes remain backward compatible during transition:

| Old Route | New Route | Notes |
|-----------|-----------|-------|
| `/queue/publish` | `/api/queue/publish` | Old still works |
| `/iroh/blob/store` | `/api/iroh/blob/store` | Old still works |
| `/hiqlite/health` | `/health/hiqlite` | Old still works |

**Why keep both?**
- Gradual migration for existing clients
- Workers/CLI tools may hardcode old URLs
- Can deprecate old routes after migration period

**To fully migrate:**
1. Update worker binaries to use `/api/*` routes
2. Update CLI tools configuration
3. Add deprecation warnings to old routes
4. Remove old routes after transition period

### Future Enhancements

The modular router is ready for:
- **API Versioning** - `/api/v2/queue/*` alongside `/api/queue/*`
- **OpenAPI Documentation** - Generate from router structure
- **Route-specific observability** - Metrics per sub-router
- **Dynamic routes** - Plugin system for additional routes
- **GraphQL endpoint** - Add `/api/graphql` alongside REST

## Future Improvements

### gRPC Adapter (Protocol Independence)

**Motivation:** Demonstrate protocol independence of domain layer

**Approach:**
- Add `src/grpc/` module with gRPC handlers
- Handlers call same domain services as HTTP handlers
- No changes to domain or repository layers
- Proves architecture is protocol-agnostic

**Structure:**

```
src/
├── handlers/          # HTTP handlers (existing)
└── grpc/              # gRPC handlers (new)
    ├── job_service.rs
    └── cluster_service.rs
```

### More Domain Services

**As complexity grows, introduce additional domain services:**

- `WorkflowOrchestrationService` - Multi-step workflow coordination
- `ResourceAllocationService` - Worker capacity planning
- `AuditLogService` - Change tracking and compliance
- `NotificationService` - Alert and notification routing

### Command/Query Separation (CQRS)

**Motivation:** Optimize read and write paths separately

**Approach:**
- Split domain services into command and query services
- Commands: `JobCommandService` (submit, cancel, retry)
- Queries: `JobQueryService` (list, search, statistics)
- Different repository traits for reads vs writes

**Benefits:**
- Optimize reads (caching, denormalization)
- Clearer intent (mutation vs read-only)
- Easier to scale independently

## References

### Architectural Patterns

- **Hexagonal Architecture** (Alistair Cockburn) - https://alistair.cockburn.us/hexagonal-architecture/
- **Repository Pattern** (Martin Fowler) - https://martinfowler.com/eaaCatalog/repository.html
- **Dependency Inversion Principle** (Robert Martin) - Clean Architecture book

### Related Documentation

- **CLAUDE.md** - Project overview, development setup, and tooling
- **README.md** - Quick start and basic usage
- **Cargo.toml** - Dependencies and project metadata

### Code Examples

- **Dashboard handlers** (`src/handlers/dashboard.rs`) - Good example of thin handlers
- **JobLifecycleService** (`src/domain/job_lifecycle.rs`) - Domain service pattern
- **Mock repositories** (`src/repositories/mocks.rs`) - Testing approach

---

## Quick Reference

### Adding a New Feature

1. Define repository trait method (if new data access needed)
2. Implement use case in domain service
3. Add thin handler calling domain service
4. Write unit tests with mocks
5. Write integration test with real infrastructure

### Testing Checklist

- Unit tests use mock repositories
- Domain logic has no HTTP knowledge
- Handlers are thin (no business logic)
- Integration tests verify full stack
- Tests are fast (unit) or complete (integration)

### Architectural Violations to Avoid

- Handlers accessing infrastructure directly
- Business logic in handlers
- Domain services depending on concrete types
- Repository implementations containing business logic
- Circular dependencies between layers

### When in Doubt

- **Is this business logic?** → Put it in a domain service
- **Is this protocol-specific?** → Put it in a handler
- **Is this data access?** → Put it behind a repository trait
- **Is this infrastructure?** → Keep it in the infrastructure layer
- **Can I test this without a database?** → Use the repository pattern
