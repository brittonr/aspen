//! Actor-based Raft RPC server with supervision.
//!
//! This module provides an actor wrapper around the RaftRpcServer that:
//! - Manages the server lifecycle with supervision
//! - Provides message-based APIs for control and monitoring
//! - Enforces Tiger Style bounded resources
//! - Integrates with the Raft supervision tree
//!
//! # Architecture
//!
//! The actor follows ractor v0.15.9's pattern of immutable actor struct with mutable state:
//! - `RaftRpcServerActor`: Empty shell (zero-sized type) implementing the `Actor` trait
//! - `RaftRpcServerActorState`: All mutable runtime state managed by the actor
//! - `RaftRpcServerMessage`: Message enum (no Clone due to RpcReplyPort one-time use)
//!
//! ## Key Design Patterns
//!
//! 1. **Zero-Sized Actor Type**: The actor struct itself holds no data, following ractor's
//!    immutability pattern. All state lives in the separate State struct.
//!
//! 2. **Message-Based Communication**: All interactions happen through messages, never through
//!    direct method calls on the actor.
//!
//! 3. **Hybrid Architecture**: The actor manages a long-lived tokio task (`RaftRpcServer`) that
//!    handles the actual RPC connections. This provides supervision while maintaining efficiency.
//!
//! 4. **Tiger Style Bounds**: All resources are explicitly bounded (e.g., max_connections).
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen::raft::server_actor::{RaftRpcServerActor, RaftRpcServerMessage};
//! use ractor::Actor;
//!
//! let (actor_ref, handle) = Actor::spawn(
//!     Some("raft-rpc-server".to_string()),
//!     RaftRpcServerActor,
//!     RaftRpcServerActorArgs { endpoint_manager, raft_core },
//! ).await?;
//!
//! // Query stats
//! let stats = ractor::call_t!(
//!     actor_ref,
//!     RaftRpcServerMessage::GetStats,
//!     1000
//! )?;
//!
//! // Graceful shutdown
//! actor_ref.send_message(RaftRpcServerMessage::Shutdown)?;
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use openraft::Raft;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::cluster::IrohEndpointManager;
use crate::raft::constants::MAX_CONCURRENT_CONNECTIONS;
use crate::raft::server::RaftRpcServer;
use crate::raft::types::AppTypeConfig;

// ============================================================================
// Messages
// ============================================================================

/// Messages for the Raft RPC server actor.
///
/// Note: Does NOT derive Clone because RpcReplyPort is not cloneable.
#[derive(Debug)]
pub enum RaftRpcServerMessage {
    /// Get server statistics.
    GetStats(RpcReplyPort<ServerStats>),
    /// Set maximum concurrent connections (Tiger Style: bounded).
    SetMaxConnections(u32),
    /// Check if server is healthy.
    HealthCheck(RpcReplyPort<bool>),
    /// Graceful shutdown request.
    Shutdown,
}

impl ractor::Message for RaftRpcServerMessage {}

// ============================================================================
// Stats and Arguments
// ============================================================================

/// Server statistics exposed via GetStats message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// Maximum allowed concurrent connections.
    pub max_connections: u32,
    /// Currently active connections (approximation).
    pub active_connections: u32,
    /// Total connections handled since startup.
    pub total_connections_handled: u64,
    /// Connection errors encountered.
    pub connection_errors: u64,
    /// Whether the server is currently running.
    pub is_running: bool,
}

/// Arguments passed to the actor on startup.
#[derive(Clone)]
pub struct RaftRpcServerActorArgs {
    /// Iroh endpoint manager for accepting connections.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Raft core to forward RPCs to.
    pub raft_core: Raft<AppTypeConfig>,
    /// If true, the actor will NOT spawn its own RaftRpcServer.
    /// This is used when the Iroh Router handles ALPN dispatching externally.
    pub use_router: bool,
}

// ============================================================================
// Actor and State
// ============================================================================

/// Actor shell for the Raft RPC server.
///
/// This is an empty struct following ractor's pattern where the actor
/// is stateless and all mutable state lives in `Self::State`.
pub struct RaftRpcServerActor;

/// Mutable state for the Raft RPC server actor.
///
/// Tiger Style: All counters are bounded by their type (u32/u64).
pub struct RaftRpcServerActorState {
    /// The underlying server handle (None if using Router).
    server_handle: Option<RaftRpcServer>,
    /// Maximum concurrent connections allowed.
    max_connections: u32,
    /// Current active connections (tracked externally, approximation here).
    active_connections: u32,
    /// Total connections handled since startup.
    total_connections: u64,
    /// Connection errors encountered.
    connection_errors: u64,
    /// Arguments cached for potential restart.
    args: RaftRpcServerActorArgs,
    /// If true, don't spawn internal server (Router handles ALPN dispatching).
    use_router: bool,
}

impl RaftRpcServerActorState {
    /// Start the underlying server.
    fn start_server(&mut self) {
        // Skip starting internal server if using Router for ALPN dispatching
        if self.use_router {
            info!("using Iroh Router for ALPN dispatching, not starting internal RaftRpcServer");
            return;
        }

        if self.server_handle.is_some() {
            warn!("server already running");
            return;
        }

        info!(
            max_connections = self.max_connections,
            "starting Raft RPC server"
        );

        self.server_handle = Some(RaftRpcServer::spawn(
            self.args.endpoint_manager.clone(),
            self.args.raft_core.clone(),
        ));
    }

    /// Stop the underlying server gracefully.
    async fn stop_server(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            info!("stopping Raft RPC server");
            if let Err(e) = handle.shutdown().await {
                error!(error = %e, "failed to shutdown RPC server gracefully");
                self.connection_errors = self.connection_errors.saturating_add(1);
            }
        }
    }

    /// Get current server statistics.
    fn get_stats(&self) -> ServerStats {
        ServerStats {
            max_connections: self.max_connections,
            active_connections: self.active_connections,
            total_connections_handled: self.total_connections,
            connection_errors: self.connection_errors,
            is_running: self.server_handle.is_some(),
        }
    }
}

// ============================================================================
// Actor Implementation
// ============================================================================

#[async_trait]
impl Actor for RaftRpcServerActor {
    type Msg = RaftRpcServerMessage;
    type State = RaftRpcServerActorState;
    type Arguments = RaftRpcServerActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("RaftRpcServerActor starting");

        let use_router = args.use_router;
        let mut state = RaftRpcServerActorState {
            server_handle: None,
            max_connections: MAX_CONCURRENT_CONNECTIONS,
            active_connections: 0,
            total_connections: 0,
            connection_errors: 0,
            args,
            use_router,
        };

        // Start the server immediately
        state.start_server();

        Ok(state)
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("RaftRpcServerActor stopping");
        state.stop_server().await;
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            RaftRpcServerMessage::GetStats(reply) => {
                let stats = state.get_stats();
                if reply.send(stats).is_err() {
                    warn!("failed to send stats reply - caller dropped");
                }
            }
            RaftRpcServerMessage::SetMaxConnections(max) => {
                // Tiger Style: Enforce reasonable bounds [1, 10000]
                let bounded_max = max.clamp(1, 10000);
                info!(
                    old_max = state.max_connections,
                    new_max = bounded_max,
                    "updating max connections"
                );
                state.max_connections = bounded_max;
                // Note: The actual RaftRpcServer uses a semaphore initialized at spawn time.
                // To change limits at runtime, we'd need to restart the server or modify
                // RaftRpcServer to accept dynamic limits. For now, this only affects stats.
            }
            RaftRpcServerMessage::HealthCheck(reply) => {
                let is_healthy = state.server_handle.is_some();
                if reply.send(is_healthy).is_err() {
                    warn!("failed to send health check reply - caller dropped");
                }
            }
            RaftRpcServerMessage::Shutdown => {
                info!("received shutdown request");
                state.stop_server().await;
                myself.stop(None);
            }
        }
        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        event: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match event {
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                error!(
                    actor = ?actor_cell.get_id(),
                    error = %error,
                    "supervised actor failed"
                );
                state.connection_errors = state.connection_errors.saturating_add(1);

                // Restart the server if it's not running
                if state.server_handle.is_none() {
                    warn!("restarting server after supervised failure");
                    state.start_server();
                }
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, reason) => {
                info!(
                    actor = ?actor_cell.get_id(),
                    reason = ?reason,
                    "supervised actor terminated"
                );
            }
            _ => {}
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_stats_creation() {
        let stats = ServerStats {
            max_connections: 100,
            active_connections: 5,
            total_connections_handled: 1000,
            connection_errors: 2,
            is_running: true,
        };

        assert_eq!(stats.max_connections, 100);
        assert_eq!(stats.active_connections, 5);
        assert_eq!(stats.total_connections_handled, 1000);
        assert_eq!(stats.connection_errors, 2);
        assert!(stats.is_running);
    }

    #[test]
    fn test_server_stats_serialization() {
        let stats = ServerStats {
            max_connections: 500,
            active_connections: 10,
            total_connections_handled: 5000,
            connection_errors: 0,
            is_running: true,
        };

        // Should serialize to JSON without panic
        let json = serde_json::to_string(&stats).expect("serialization failed");
        assert!(json.contains("500"));
        assert!(json.contains("5000"));

        // Should deserialize back
        let deserialized: ServerStats =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized.max_connections, 500);
    }

    #[test]
    fn test_max_connections_bounds() {
        // Test the clamping logic that will be used in SetMaxConnections
        let test_cases = [
            (0u32, 1u32),   // Below minimum -> clamp to 1
            (1, 1),         // At minimum -> unchanged
            (500, 500),     // Normal value -> unchanged
            (10000, 10000), // At maximum -> unchanged
            (20000, 10000), // Above maximum -> clamp to 10000
        ];

        for (input, expected) in test_cases {
            let bounded = input.clamp(1, 10000);
            assert_eq!(bounded, expected, "clamp({}) should be {}", input, expected);
        }
    }
}
