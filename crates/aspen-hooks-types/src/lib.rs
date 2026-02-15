//! Lightweight type definitions for the Aspen hook system.
//!
//! This crate contains only the type definitions needed for hook configuration,
//! events, and tickets. It has minimal dependencies (no tokio, no async runtime)
//! and can be used by consumers who only need to work with hook types without
//! pulling in the full hook runtime.
//!
//! For the full hook system with services, handlers, and pub/sub machinery,
//! see the `aspen-hooks` crate which re-exports all types from this crate.
//!
//! # Type Categories
//!
//! - **Configuration**: `HooksConfig`, `HookHandlerConfig`, `ExecutionMode`, `HookHandlerType`
//! - **Events**: `HookEvent`, `HookEventType`, and payload types for each event category
//! - **Tickets**: `AspenHookTicket` for external program integration
//! - **Constants**: Tiger Style resource bounds
//!
//! # Example
//!
//! ```ignore
//! use aspen_hooks_types::{HooksConfig, HookEvent, HookEventType};
//!
//! // Create a configuration
//! let config = HooksConfig::default();
//! assert!(config.validate().is_ok());
//!
//! // Create an event
//! let event = HookEvent::new(
//!     HookEventType::WriteCommitted,
//!     1,
//!     serde_json::json!({"key": "test"}),
//! );
//! ```

pub mod config;
pub mod constants;
pub mod error;
pub mod event;
pub mod ticket;

// Re-export main types for convenience
pub use config::ExecutionMode;
pub use config::HookHandlerConfig;
pub use config::HookHandlerType;
pub use config::HooksConfig;
pub use constants::DEFAULT_HANDLER_RETRY_COUNT;
pub use constants::DEFAULT_HANDLER_TIMEOUT_MS;
pub use constants::DEFAULT_JOB_PRIORITY;
pub use constants::HOOK_EVENT_ENV_VAR;
pub use constants::HOOK_EVENT_TYPE_ENV_VAR;
pub use constants::HOOK_JOB_TYPE;
pub use constants::HOOK_TOPIC_ENV_VAR;
pub use constants::HOOK_TOPIC_PREFIX;
pub use constants::MAX_CONCURRENT_HANDLER_EXECUTIONS;
pub use constants::MAX_FORWARD_CONNECTIONS;
pub use constants::MAX_HANDLER_NAME_SIZE;
pub use constants::MAX_HANDLER_RETRY_COUNT;
pub use constants::MAX_HANDLER_TIMEOUT_MS;
pub use constants::MAX_HANDLERS;
pub use constants::MAX_METRICS_SAMPLES;
pub use constants::MAX_PATTERN_SIZE;
pub use constants::MAX_SHELL_COMMAND_SIZE;
pub use constants::MAX_SHELL_OUTPUT_SIZE;
pub use constants::METRICS_AGGREGATION_INTERVAL_MS;
pub use constants::RETRY_BACKOFF_MULTIPLIER;
pub use constants::RETRY_INITIAL_BACKOFF_MS;
pub use constants::RETRY_MAX_BACKOFF_MS;
pub use constants::SHELL_GRACE_PERIOD;
pub use constants::TOPIC_SEPARATOR;
pub use error::HookTypeError;
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
// Ticket types
pub use ticket::AspenHookTicket;
pub use ticket::DEFAULT_EXPIRY_HOURS;
pub use ticket::HOOK_TICKET_PREFIX;
pub use ticket::MAX_BOOTSTRAP_PEERS;
pub use ticket::MAX_CLUSTER_ID_SIZE;
pub use ticket::MAX_EVENT_TYPE_SIZE;
pub use ticket::MAX_PAYLOAD_SIZE;
pub use ticket::MAX_RELAY_URL_SIZE;

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
        assert!(config.is_enabled);
        assert!(config.validate().is_ok());
    }
}
