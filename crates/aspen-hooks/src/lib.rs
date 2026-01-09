//! Event-driven hook system for Aspen distributed systems.
//!
//! This crate provides an event-driven hook system that leverages the existing
//! `aspen-pubsub` and `aspen-jobs` infrastructure. The system uses a hybrid approach:
//!
//! 1. **Fast path (Direct)**: In-process handlers for low-latency event processing
//! 2. **Reliable path (Jobs)**: Submit to job queue for handlers needing retry, DLQ, persistence
//!
//! # Architecture
//!
//! ```text
//! Aspen Events                           Pub/Sub Layer (aspen-pubsub)
//!      |                                          |
//!      v                                          v
//! KV Operations --> HookPublisher --> Topic: "hooks.kv.write_committed"
//! Cluster Events -> HookPublisher --> Topic: "hooks.cluster.leader_elected"
//!                                           |
//!                                           v
//!                                    Raft Consensus
//!                                           |
//!                                           v
//!                                    HookService
//!                           (subscribes to hook topics)
//!                                           |
//!                +--------------------------+-------------------------+
//!                |                          |                         |
//!                v                          v                         v
//!         Direct Handler            Job-Based Handler           Shell Handler
//!       (InProcessHandler)          (HookJobWorker)           (ShellHandler)
//!         [fast path]              [reliable path]             [local exec]
//! ```
//!
//! # Quick Start
//!
//! ```ignore
//! use aspen_hooks::{HooksConfig, HookService, HookEvent, HookEventType};
//! use std::sync::Arc;
//!
//! // Create configuration
//! let config = HooksConfig::default();
//!
//! // Create the service
//! let service = HookService::new(config, kv_store);
//!
//! // Register in-process handlers
//! service.register_handler("cache_invalidator", |event| async {
//!     println!("Received event: {:?}", event.event_type);
//!     Ok(())
//! }).await;
//!
//! // Start the service
//! let handle = service.run().await?;
//!
//! // Publish events
//! let publisher = HookPublisher::new(kv_store);
//! let event = HookEvent::new(HookEventType::WriteCommitted, 1, serde_json::json!({"key": "test"}));
//! publisher.publish_event(&event).await?;
//!
//! // Shutdown
//! handle.shutdown().await;
//! ```
//!
//! # Handler Types
//!
//! ## In-Process Handlers
//!
//! The fastest option with zero serialization overhead. Register Rust async closures:
//!
//! ```ignore
//! service.register_handler("my_handler", |event| async {
//!     // Handle the event
//!     Ok(())
//! }).await;
//! ```
//!
//! ## Shell Handlers
//!
//! Execute shell commands with event data passed via environment variables:
//!
//! - `ASPEN_HOOK_EVENT`: JSON-serialized event
//! - `ASPEN_HOOK_TOPIC`: Event topic
//! - `ASPEN_HOOK_EVENT_TYPE`: Event type name
//!
//! ## Forward Handlers
//!
//! Forward events to remote Aspen clusters for cross-cluster event propagation.
//!
//! # Execution Modes
//!
//! ## Direct Mode (Default)
//!
//! Events are handled immediately with semaphore-controlled concurrency:
//! - Low latency (microseconds)
//! - No persistence (fire-and-forget)
//! - Best for: cache invalidation, logging, real-time reactions
//!
//! ## Job Mode
//!
//! Events are submitted to the job queue:
//! - Higher latency (milliseconds)
//! - Automatic retry with exponential backoff
//! - Dead Letter Queue for failed handlers
//! - Persistent (survives restarts)
//! - Best for: external notifications, critical operations
//!
//! # Topic Hierarchy
//!
//! ```text
//! hooks.
//!   kv.
//!     write_committed      - KV write operations
//!     delete_committed     - KV delete operations
//!     ttl_expired          - TTL expirations
//!   cluster.
//!     leader_elected       - New leader elected
//!     membership_changed   - Voters/learners changed
//!     node_added           - Node joined cluster
//!     node_removed         - Node left cluster
//!   system.
//!     snapshot_created     - Snapshot taken
//!     snapshot_installed   - Snapshot restored
//!     health_changed       - Node health status change
//!   blob.
//!     blob_added           - Blob added to store
//!     blob_deleted         - Blob removed (GC protection removed)
//!     blob_downloaded      - Blob downloaded from peer
//!     blob_protected       - Blob protected from GC
//!     blob_unprotected     - Blob unprotected from GC
//!   docs.
//!     sync_started         - Docs sync session started
//!     sync_completed       - Docs sync session completed
//!     entry_imported       - Remote entry imported to local KV
//!     entry_exported       - Batch exported to docs namespace
//! ```
//!
//! # Tiger Style Compliance
//!
//! All operations are bounded by explicit limits defined in `constants.rs`:
//! - `MAX_HANDLERS`: 64
//! - `MAX_CONCURRENT_HANDLER_EXECUTIONS`: 32
//! - `MAX_HANDLER_TIMEOUT_MS`: 30,000
//! - `MAX_SHELL_COMMAND_SIZE`: 4,096
//! - `MAX_SHELL_OUTPUT_SIZE`: 64KB

pub mod config;
pub mod constants;
pub mod error;
pub mod event;
pub mod handlers;
pub mod metrics;
pub mod publisher;
pub mod service;
pub mod ticket;
pub mod worker;

// Re-export main types for convenience
pub use config::ExecutionMode;
pub use config::HookHandlerConfig;
pub use config::HookHandlerType;
pub use config::HooksConfig;
pub use error::HookError;
pub use error::Result;
// Blob event payloads
pub use event::BlobAddedPayload;
pub use event::BlobDeletedPayload;
pub use event::BlobDownloadedPayload;
pub use event::BlobProtectedPayload;
pub use event::BlobUnprotectedPayload;
// Docs event payloads
pub use event::DocsEntryExportedPayload;
pub use event::DocsEntryImportedPayload;
pub use event::DocsSyncCompletedPayload;
pub use event::DocsSyncStartedPayload;
pub use event::HealthChangedPayload;
pub use event::HookEvent;
pub use event::HookEventType;
pub use event::KvDeletePayload;
pub use event::KvWritePayload;
pub use event::LeaderElectedPayload;
pub use event::MembershipChangedPayload;
pub use event::SnapshotPayload;
pub use event::TtlExpiredPayload;
pub use handlers::HookHandler;
pub use handlers::InProcessHandler;
pub use handlers::InProcessHandlerRegistry;
pub use handlers::ShellHandler;
pub use handlers::create_handler_from_config;
pub use metrics::ExecutionMetrics;
pub use metrics::GlobalMetricsSnapshot;
pub use metrics::HandlerMetricsSnapshot;
pub use metrics::MetricsSnapshot;
pub use publisher::HookPublisher;
pub use service::HookService;
pub use service::HookServiceHandle;
// Ticket types
pub use ticket::AspenHookTicket;
pub use ticket::HOOK_TICKET_PREFIX;
pub use worker::HookJobPayload;
pub use worker::create_hook_job_spec;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_api_exports() {
        // Verify main types are accessible
        let _: fn() -> HooksConfig = HooksConfig::default;
        let _event_type = HookEventType::WriteCommitted;

        // Verify constants are accessible
        assert!(constants::MAX_HANDLERS > 0);
        assert!(constants::MAX_CONCURRENT_HANDLER_EXECUTIONS > 0);
    }

    #[test]
    fn test_default_config() {
        let config = HooksConfig::default();
        assert!(config.enabled);
        assert!(config.validate().is_ok());
    }
}
