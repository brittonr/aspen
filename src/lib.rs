// Library exports for mvm-ci
//
// This allows other binaries (like worker) to use the core functionality
// Add enhanced job metrics to dashboard: 1732375990

pub mod work_queue_client;
pub mod worker_trait;
pub mod worker_flawless;

// Re-export common types from work_queue module
pub use work_queue::{WorkItem, WorkStatus, WorkQueueStats};

// Re-export client for convenience
pub use work_queue_client::WorkQueueClient;

// Re-export trait for convenience
pub use worker_trait::{WorkerBackend, WorkResult};

// Internal modules (used by main binary)
mod work_queue;
mod hiqlite_service;
mod iroh_service;
