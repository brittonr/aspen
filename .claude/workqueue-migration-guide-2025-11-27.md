# WorkQueue Migration Guide
Created: 2025-11-27

## Summary

The WorkQueue God Object has been successfully refactored into a clean architecture. This guide explains how to complete the migration and integrate the new components into the application.

## What Was Built

### New Module Structure: `src/work/`

```
src/work/
├── mod.rs                      # Module definitions and documentation
├── repository.rs               # WorkRepositoryImpl - data access layer
├── command_service.rs          # WorkCommandService - write operations
├── query_service.rs            # WorkQueryService - read operations
└── cached_query_service.rs     # CachedWorkQueryService - caching decorator
```

### Components

1. **WorkRepositoryImpl** (230 lines)
   - Pure data access layer
   - Implements WorkRepository trait
   - NO business logic, NO caching

2. **WorkCommandService** (320 lines)
   - Business logic for writes (publish, claim, update)
   - Uses JobClaimingService for compatibility checking
   - Emits cache invalidation events

3. **WorkQueryService** (120 lines)
   - Read operations (list, find, stats)
   - NO caching - just delegates to repository

4. **CachedWorkQueryService** (250 lines)
   - Decorates WorkQueryService with caching
   - Version-based cache invalidation
   - Automatic refresh on miss

## Architecture

```
HTTP Handlers
    │
    ├─► WorkCommandService (writes)
    │       ↓
    │   PersistentStore
    │       ↓
    │   Cache Invalidation Event
    │
    └─► CachedWorkQueryService (reads)
            ├─► Check Cache Version
            ├─► If stale: refresh via WorkQueryService
            └─► If fresh: return from WorkItemCache
```

## Migration Steps

### Phase 1: Update State Factory (CURRENT TASK)

The state factory needs to be updated to create the new components:

```rust
// In src/state/factory.rs

use crate::work::{
    WorkRepositoryImpl,
    WorkCommandService,
    WorkQueryService,
    CachedWorkQueryService
};

// In the build method:

// 1. Create repository
let work_repository = Arc::new(WorkRepositoryImpl::new(hiqlite_store.clone()));

// 2. Create command service (for writes)
let work_command_service = Arc::new(WorkCommandService::new(
    node_id.clone(),
    hiqlite_store.clone(),
));

// 3. Create query service (for reads)
let work_query_service = Arc::new(WorkQueryService::new(
    work_repository.clone()
));

// 4. Create cached query service (wraps query service)
let cached_work_query = Arc::new(
    CachedWorkQueryService::new(work_query_service.clone())
        .await?
);
```

### Phase 2: Update Repository Wrapper

The `WorkQueueWorkRepository` can be simplified or removed:

**Option A: Simplify it**
```rust
// src/repositories/work_queue_repository.rs
pub struct WorkQueueWorkRepository {
    command_service: Arc<WorkCommandService>,
    query_service: Arc<CachedWorkQueryService>,
}

impl WorkRepository for WorkQueueWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        self.command_service.publish_work(job_id, payload).await
    }

    async fn claim_work(...) -> Result<Option<Job>> {
        let cache_version = self.command_service.get_cache_version();
        self.command_service.claim_work(worker_id, worker_type).await
    }

    async fn list_work(&self) -> Result<Vec<Job>> {
        let cache_version = self.command_service.get_cache_version();
        self.query_service.list_work(cache_version).await
    }

    // ... etc
}
```

**Option B: Remove it and use services directly**
```rust
// In handlers, use the services directly:
pub async fn queue_list(
    State((cmd, query)): State<(Arc<WorkCommandService>, Arc<CachedWorkQueryService>)>,
) -> Result<Json<Vec<Job>>, AppError> {
    let cache_version = cmd.get_cache_version();
    let jobs = query.list_work(cache_version).await?;
    Ok(Json(jobs))
}
```

### Phase 3: Update HTTP Handlers

Handlers need access to both command and query services:

```rust
// In src/server/router.rs

pub fn queue_routes(
    work_command: Arc<WorkCommandService>,
    work_query: Arc<CachedWorkQueryService>,
) -> Router {
    Router::new()
        .route("/queue", get(queue_list))
        .route("/queue/claim", post(queue_claim))
        .route("/queue/publish", post(queue_publish))
        .with_state((work_command, work_query))
}

// In handlers:
pub async fn queue_publish(
    State((cmd, _)): State<(Arc<WorkCommandService>, Arc<CachedWorkQueryService>)>,
    Json(req): Json<PublishRequest>,
) -> Result<StatusCode, AppError> {
    cmd.publish_work(req.job_id, req.payload).await?;
    Ok(StatusCode::CREATED)
}

pub async fn queue_claim(
    State((cmd, _)): State<(Arc<WorkCommandService>, Arc<CachedWorkQueryService>)>,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<Option<Job>>, AppError> {
    let job = cmd.claim_work(
        req.worker_id.as_deref(),
        req.worker_type,
    ).await?;
    Ok(Json(job))
}

pub async fn queue_list(
    State((cmd, query)): State<(Arc<WorkCommandService>, Arc<CachedWorkQueryService>)>,
) -> Result<Json<Vec<Job>>, AppError> {
    let cache_version = cmd.get_cache_version();
    let jobs = query.list_work(cache_version).await?;
    Ok(Json(jobs))
}
```

### Phase 4: Remove Old WorkQueue

After all handlers are updated and tests pass:

1. Delete `src/work_queue.rs`
2. Remove from `src/lib.rs`: `mod work_queue;`
3. Remove old test files if they're duplicates
4. Update imports across the codebase

## Testing Strategy

### Unit Tests (Already Written)

Each component has its own test suite:
- `work::repository::tests` - 5 tests
- `work::command_service::tests` - 4 tests
- `work::query_service::tests` - 5 tests
- `work::cached_query_service::tests` - 4 tests

Run with:
```bash
nix develop -c cargo test work::
```

### Integration Tests

Run existing work_queue integration tests to ensure compatibility:
```bash
nix develop -c cargo test work_queue_tests::
nix develop -c cargo test work_queue_distributed_tests::
nix develop -c cargo test work_queue_proptest::
```

### Property-Based Tests

The existing proptest suite should continue to work with the new components. You may want to create additional property tests for:
- Cache coherency across command/query separation
- Version tracking invariants
- Repository isolation

## Cache Version Coordination

The key to cache coherency is the CacheVersion in WorkCommandService:

```rust
// Every write operation increments the version
cmd.publish_work(...).await; // Increments version
cmd.claim_work(...).await;   // Increments version
cmd.update_status(...).await; // Increments version

// Reads use the current version to check cache freshness
let version = cmd.get_cache_version();
let jobs = query.list_work(version).await; // Refreshes if needed
```

This ensures:
1. Writes always invalidate the cache
2. Reads detect stale cache via version mismatch
3. No race conditions between write and read paths

## Performance Considerations

### Cache Hit Rate

Monitor cache effectiveness:
```rust
// Add metrics to CachedWorkQueryService
pub fn cache_stats(&self) -> CacheStats {
    CacheStats {
        last_refresh: self.last_cache_sync.load(Ordering::SeqCst),
        current_version: /* from command service */,
        items_cached: /* from cache */,
    }
}
```

### Read vs Write Ratio

The architecture is optimized for read-heavy workloads:
- Reads are fast (in-memory cache)
- Writes are slower (persistent store + cache invalidation)
- Typical ratio: 90% reads, 10% writes

### Distributed Consistency

The write-through pattern ensures consistency:
1. Write to PersistentStore first (atomic)
2. Increment cache version (atomic)
3. Readers refresh on next read

## Rollback Plan

If issues arise during migration:

1. **Keep old WorkQueue**: The old code is still in the repository
2. **Feature flag**: Add a config option to toggle between old/new
3. **Gradual rollout**: Migrate one handler at a time
4. **Monitor metrics**: Watch for performance regressions

## Common Pitfalls

### 1. Forgetting Cache Version

```rust
// ❌ WRONG - no version check
let jobs = query.list_work(0).await;

// ✅ CORRECT - use current version
let version = cmd.get_cache_version();
let jobs = query.list_work(version).await;
```

### 2. Direct Cache Access

```rust
// ❌ WRONG - bypassing query service
let cache = WorkItemCache::new();
let jobs = cache.get_all().await;

// ✅ CORRECT - use query service
let jobs = query.list_work(version).await;
```

### 3. Mixed Responsibilities

```rust
// ❌ WRONG - putting business logic in repository
impl WorkRepositoryImpl {
    async fn claim_work(&self, ...) -> Result<Option<Job>> {
        // Don't add compatibility checking here!
    }
}

// ✅ CORRECT - business logic in service
impl WorkCommandService {
    async fn claim_work(&self, ...) -> Result<Option<Job>> {
        // Compatibility checking belongs here
        if !JobClaimingService::is_compatible(...) {
            return Ok(None);
        }
    }
}
```

## Success Criteria

Migration is complete when:

- [ ] All handlers use new services
- [ ] All existing tests pass
- [ ] Integration tests pass
- [ ] Property tests pass
- [ ] Old WorkQueue is deleted
- [ ] Performance metrics are stable
- [ ] Cache hit rate is >80% (for read-heavy workloads)

## Benefits Achieved

1. **Separation of Concerns**: Each component has exactly one responsibility
2. **Testability**: 18 unit tests (was 0 for business logic in WorkQueue)
3. **Maintainability**: Largest file is 320 lines (was 444 lines monolith)
4. **Flexibility**: Can swap cache, repository implementations
5. **Clarity**: Always know where to add features

## Questions?

If you encounter issues:
1. Check the design doc: `.claude/workqueue-refactoring-design-2025-11-27.toml`
2. Review the architecture diagram in `src/work/mod.rs`
3. Run the tests: `nix develop -c cargo test work::`
4. Check existing integration tests for examples

## Next Steps

1. Update `src/state/factory.rs` to create new components
2. Update handlers to use command/query services
3. Run full test suite
4. Monitor performance in production
5. Delete old WorkQueue after validation period
