//! Health checking system for VM fleet management
//!
//! This module provides health checking, circuit breaking, and monitoring
//! for VMs. It follows clean architecture principles with clear separation
//! between pure state logic and side effects.
//!
//! ## Architecture
//!
//! The health checking system is decomposed into focused components:
//! - **types**: Shared type definitions (HealthStatus, StateTransition, etc.)
//! - **config**: Validated configuration
//! - **state_machine**: Pure state transition logic
//! - **circuit_breaker**: Circuit breaker pattern implementation
//! - **monitor**: VM health checking via Unix sockets
//! - **side_effects**: Side effect execution
//! - **metrics**: Health metrics aggregation
//!
//! ## State Transitions
//!
//! ```text
//! Unknown → Healthy (first successful check)
//! Unknown → Degraded (first failed check)
//! Healthy → Healthy (continued success)
//! Healthy → Degraded (first failure)
//! Degraded → Healthy (recovery after <= recovery_threshold failures)
//! Degraded → Degraded (gradual recovery or additional failures)
//! Degraded → Unhealthy (failures >= failure_threshold)
//! Unhealthy → Degraded (first successful check after unhealthy)
//! Unhealthy → Unhealthy (continued failure)
//! ```

pub mod types;
pub mod config;
pub mod state_machine;
pub mod circuit_breaker;
pub mod monitor;
pub mod side_effects;
pub mod metrics;
pub mod coordinator;

// Re-export public types
pub use types::{HealthStatus, StateTransition, SideEffect, CheckResult};
pub use config::HealthCheckConfig;
pub use state_machine::HealthStateMachine;
pub use circuit_breaker::{CircuitBreaker, CircuitState, CircuitBreakerMetrics};
pub use monitor::VmHealthMonitor;
pub use side_effects::SideEffectExecutor;
pub use metrics::HealthStats;

// Re-export the coordinator implementation
// The coordinator orchestrates all the extracted components
pub use coordinator::HealthChecker;
