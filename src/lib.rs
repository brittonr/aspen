// Library exports for mvm-ci
//
// This allows other binaries (like worker) to use the core functionality
// Refactor handlers into modular architecture: 1732376490

pub mod config;
pub mod work_queue_client;
pub mod worker_trait;
pub mod worker_flawless;

// Re-export common types from work_queue module
pub use work_queue::{WorkItem, WorkStatus, WorkQueueStats};

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
mod hiqlite_service;
mod iroh_service;
mod persistent_store;
mod hiqlite_persistent_store;
mod services;
