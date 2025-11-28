//! Work Module - Clean Architecture Implementation
//!
//! This module provides a clean separation of concerns for the work queue:
//!
//! ## Architecture Layers
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                  Presentation Layer                      │
//! │              (HTTP Handlers, API Routes)                 │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │                   Domain Layer                           │
//! │  ┌─────────────────────┐  ┌─────────────────────────┐  │
//! │  │ WorkCommandService  │  │  WorkQueryService       │  │
//! │  │ (Write Operations)  │  │  (Read Operations)      │  │
//! │  └─────────────────────┘  └─────────────────────────┘  │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Cache Layer                           │
//! │            ┌─────────────────────────────┐               │
//! │            │ CachedWorkQueryService      │               │
//! │            │ (Decorates WorkQueryService) │               │
//! │            └─────────────────────────────┘               │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │                 Repository Layer                         │
//! │              ┌─────────────────────┐                     │
//! │              │ WorkRepositoryImpl  │                     │
//! │              └─────────────────────┘                     │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!                            ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │               Infrastructure Layer                       │
//! │               ┌──────────────────┐                       │
//! │               │ PersistentStore  │                       │
//! │               │  (Hiqlite DB)    │                       │
//! │               └──────────────────┘                       │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Component Responsibilities
//!
//! ### WorkRepositoryImpl (Repository Layer)
//! - Direct interaction with PersistentStore
//! - Data conversion (if needed)
//! - NO business logic, NO caching
//!
//! ### WorkCommandService (Domain Layer)
//! - Business logic for write operations (publish, claim, update)
//! - Coordinates with JobClaimingService and WorkStateMachine
//! - Emits cache invalidation events
//!
//! ### WorkQueryService (Domain Layer)
//! - Query operations (list, find, filter, stats)
//! - NO caching - just delegates to repository
//!
//! ### CachedWorkQueryService (Cache Layer)
//! - Decorates WorkQueryService with caching
//! - Cache invalidation based on version tracking
//! - Automatic refresh on cache miss
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use blixard::work::{WorkCommandService, WorkQueryService, CachedWorkQueryService};
//! use blixard::work::repository::WorkRepositoryImpl;
//! use blixard::persistent_store::PersistentStore;
//!
//! async fn setup(store: Arc<dyn PersistentStore>) -> (WorkCommandService, CachedWorkQueryService) {
//!     // Create repository
//!     let repository = Arc::new(WorkRepositoryImpl::new(store.clone()));
//!
//!     // Create command service (for writes)
//!     let command_service = WorkCommandService::new("node-1".to_string(), store);
//!
//!     // Create query service (for reads)
//!     let query_service = WorkQueryService::new(repository);
//!
//!     // Wrap query service with caching
//!     let cached_query_service = CachedWorkQueryService::new(query_service)
//!         .await
//!         .expect("Failed to create cached query service");
//!
//!     (command_service, cached_query_service)
//! }
//! ```
//!
//! ## Benefits
//!
//! - **Separation of Concerns**: Each component has a single responsibility
//! - **Testability**: Can test repository, commands, queries independently
//! - **Swappable Implementations**: Can swap cache, repository via traits
//! - **Clear Boundaries**: Know exactly where to add new features
//! - **Reduced Complexity**: No single file >250 lines
//!

pub mod repository;
pub mod command_service;
pub mod query_service;
pub mod cached_query_service;

pub use repository::WorkRepositoryImpl;
pub use command_service::{WorkCommandService, CacheVersion};
pub use query_service::WorkQueryService;
pub use cached_query_service::CachedWorkQueryService;
