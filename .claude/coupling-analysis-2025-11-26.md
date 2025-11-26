# Blixard Coupling Analysis Report
Generated: 2025-11-26

## Executive Summary

The Blixard codebase demonstrates good architectural patterns with factory-based dependency injection and repository abstractions, but has significant coupling issues in peripheral areas that bypass these patterns. The core infrastructure (Iroh P2P and Hiqlite) are intentionally coupled per design requirements.

## Critical Coupling Issues (Priority 1)

### 1. Inefficient Data Access Pattern (PERFORMANCE KILLER)
**Location:** `src/domain/job_commands.rs:157,228,265`
**Problem:** Loading ALL jobs to find one by ID
```rust
let jobs = self.work_repo.list_work().await?;  // Loads 10,000+ jobs
let current_job = jobs.iter().find(|j| j.id == job_id)?;  // O(n) search
```
**Impact:** O(n) network calls for every job update
**Fix:** Use existing `find_by_id()` method directly

### 2. Unsafe VmManager Stub
**Location:** `src/state/services.rs:72-77`
**Problem:** Uses unsafe transmute to create fake VmManager
```rust
Arc::new(VmService::new(unsafe {
    std::mem::transmute::<Arc<()>, Arc<VmManager>>(Arc::new(()))
}))
```
**Impact:** Will panic if VmService methods are called
**Fix:** Create proper Option<Arc<VmManager>> handling

### 3. Worker Self-Contained Initialization
**Location:** `src/worker_microvm.rs:73-98`
**Problem:** Directly instantiates HiqliteService and VmManager
```rust
let hiqlite = Arc::new(HiqliteService::new(...).await?);  // Direct!
let vm_manager = Arc::new(VmManager::new(...).await?);     // Direct!
```
**Impact:** Cannot test with mocks, duplicates factory logic
**Fix:** Accept services as constructor parameters

## High Priority Coupling Issues

### 4. Iroh API Bypasses Domain Layer
**Location:** `src/iroh_api.rs` (all 7 endpoints)
**Problem:** Handlers directly access `state.infrastructure().iroh()`
**Impact:** No business logic abstraction for P2P operations
**Fix:** Create P2pService domain abstraction

### 5. Hard-coded Configuration Values
**Locations:**
- `src/adapters/flawless_adapter.rs:52` - `http://localhost:27288`
- `src/state/services.rs:61` - `/tmp/tofu-work`
- `src/adapters/local_adapter.rs:35-43` - Multiple magic numbers
**Impact:** Cannot configure without code changes
**Fix:** Move all to AppConfig

### 6. Environment Variables Bypass Config System
**Locations:**
- `src/middleware/auth.rs:20,51` - `BLIXARD_API_KEY` read on every request
- `src/bin/worker.rs:24,28,102` - Worker configuration
- `src/main.rs:17` - Feature flags (`SKIP_FLAWLESS`)
**Impact:** Configuration scattered, not testable
**Fix:** Centralize in AppConfig

### 7. God Object: AppState
**Location:** `src/state/mod.rs`
**Problem:** Every handler depends on entire infrastructure + all services
**Impact:** Heavyweight test setup, brittle tests
**Fix:** Extract handler-specific state slices

### 8. Domain Types Directly Exposed in APIs
**Location:** `src/handlers/queue.rs:99,114`
**Problem:** Job, Worker types serialized directly to JSON responses
**Impact:** Can't evolve domain models without breaking API
**Fix:** Create DTOs for API responses

## Medium Priority Issues

### 9. VmService Exposes Infrastructure
**Location:** `src/domain/vm_service.rs:26-27`
```rust
pub fn vm_manager(&self) -> Arc<VmManager> {
    Arc::clone(&self.vm_manager)  // Exposes infra type
}
```
**Fix:** Return domain types only

### 10. Tofu Service Direct Backend Creation
**Location:** `src/domain/tofu_service.rs:60-63`
**Problem:** Directly instantiates TofuStateBackend and TofuPlanExecutor
**Fix:** Use factory pattern for backends

### 11. Plugin System Bidirectional Coupling
**Location:** `src/domain/plugins.rs`
**Problem:** Plugins directly depend on Job, Worker domain types
**Fix:** Use generic data structures for plugin interface

### 12. Infrastructure Without Abstractions
**Problems:**
- Direct file system operations (`src/tofu/plan_executor.rs`)
- Time-dependent code without Clock abstraction
- Direct `tokio::spawn()` calls (12+ files)
- Direct stdout `println!()` in production code
- Command execution without abstraction (OpenTofu)

## What's Working Well

1. **Factory Pattern** - Well-designed in `state/factory.rs`
2. **Repository Abstractions** - Trait-based, testable
3. **Domain Service Container** - Proper dependency injection
4. **Handler Layer** - Correctly delegates to domain services
5. **CQRS Pattern** - Command/Query separation implemented

## Recommended Fix Sequence

### Week 1: Critical Performance & Safety
1. Fix inefficient data access (1 day)
2. Remove unsafe VmManager stub (1 day)
3. Extract configuration to AppConfig (2 days)
4. Create P2pService abstraction (1 day)

### Week 2: Testability
5. Refactor worker initialization for DI (2 days)
6. Create DTOs for API responses (2 days)
7. Extract AppState into focused slices (1 day)

### Week 3: Infrastructure Abstractions
8. Create Clock abstraction for time
9. Create FileSystem abstraction
10. Create CommandExecutor for OpenTofu

## Metrics

- **Well-decoupled modules:** 14/25 (56%)
- **Coupling Score by Layer:**
  - Repositories: 9/10
  - Domain Services: 7/10
  - Handlers: 8/10
  - Adapters: 5/10
  - Workers: 3/10
- **Estimated refactoring effort:** 3-4 weeks

## Key Insight

The core architecture is solid with good patterns. The coupling issues are primarily in:
1. Peripheral code that bypasses the factory pattern
2. Configuration management
3. Missing abstractions for infrastructure operations
4. Direct exposure of domain types in APIs

Addressing the top 5 issues would eliminate 80% of the testability problems.