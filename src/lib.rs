// Library exports for mvm-ci
//
// This allows other binaries (like worker) to use the core functionality
// Refactor handlers into modular architecture: 1732376490

pub mod config;
pub mod work_queue_client;
pub mod worker_trait;
pub mod worker_flawless;
pub mod worker_microvm;

// Re-export domain types (WorkQueue now uses these internally)
pub use domain::types::{Job, JobStatus, QueueStats, Worker, WorkerStatus, WorkerType, WorkerRegistration, WorkerHeartbeat, WorkerStats};

// Re-export client for convenience
pub use work_queue_client::WorkQueueClient;

// Re-export trait for convenience
pub use worker_trait::{WorkerBackend, WorkResult};

// Re-export config for convenience
pub use config::AppConfig;

// Internal modules (used by main binary)
mod work_queue;
mod work_state_machine;
mod work_item_cache;
mod persistent_store;
mod hiqlite_persistent_store;
mod work_queue_tests;
mod work_queue_proptest;
mod work_queue_distributed_tests;
mod services;
mod iroh_service;
mod iroh_api;
mod handlers;
mod views;
pub mod domain;
pub mod middleware;

// Export modules needed for testing
pub mod hiqlite;
pub mod repositories;
pub mod state;
pub mod server;

// Execution backend adapters
pub mod adapters;

// Infrastructure layer
pub mod infrastructure;

// Deprecated: vm_manager moved to infrastructure::vm
#[deprecated(since = "0.2.0", note = "Use infrastructure::vm instead")]
pub use infrastructure::vm as vm_manager;

// API handlers
pub mod api;

// Storage abstractions
pub mod storage;

// OpenTofu integration
#[cfg(feature = "tofu-support")]
pub mod tofu;
