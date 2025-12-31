//! Pure client library for Aspen distributed systems.
//!
//! This crate provides a lightweight client for connecting to Aspen clusters
//! without pulling in the heavy raft/cluster dependencies. It is designed for
//! external consumers who only need to interact with Aspen as a client.
//!
//! # Key Components
//!
//! - [`AspenClient`]: Main client for sending RPC requests to cluster nodes
//! - [`ClientRpcRequest`]: All available RPC request types
//! - [`ClientRpcResponse`]: All available RPC response types
//! - [`AspenClusterTicket`]: Ticket for bootstrapping connections
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen_client::{AspenClient, ClientRpcRequest};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Connect to a cluster using a ticket
//!     let client = AspenClient::connect(
//!         "aspen...",  // ticket string
//!         Duration::from_secs(5),
//!         None,  // no auth token
//!     ).await?;
//!
//!     // Send a request
//!     let response = client.send(ClientRpcRequest::Ping).await?;
//!     println!("Response: {:?}", response);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `pijul`: Include Pijul version control RPC variants
//! - `forge`: Include Forge decentralized Git RPC variants

mod client;
mod constants;
mod overlay_constants;
mod rpc;
mod ticket;

// Extended client functionality modules
pub mod cache;
pub mod coordination;
pub mod overlay;
pub mod subscription;
pub mod transaction;
pub mod watch;

// Re-export all public types at crate root
pub use client::AspenClient;
pub use client::AuthToken;
pub use client::AuthenticatedRequest;
pub use constants::CLIENT_ALPN;
pub use constants::MAX_CLIENT_MESSAGE_SIZE;
pub use rpc::*;
pub use ticket::AspenClusterTicket;
pub use ticket::SignedAspenClusterTicket;

// Re-export overlay and coordination types
pub use cache::LocalCache;
pub use overlay_constants::CONNECTION_TIMEOUT;
pub use overlay_constants::DEFAULT_CACHE_TTL;
pub use overlay_constants::HEARTBEAT_INTERVAL;
pub use overlay_constants::MAX_CACHE_TTL;
pub use overlay_constants::MAX_RECONNECT_DELAY;
pub use overlay_constants::MAX_SUBSCRIPTIONS;
pub use overlay_constants::MAX_TOTAL_CACHE_ENTRIES;
pub use overlay_constants::RECONNECT_DELAY;
pub use coordination::BarrierClient;
pub use coordination::BarrierEnterResult;
pub use coordination::BarrierLeaveResult;
pub use coordination::BarrierStatusResult;
pub use coordination::CoordinationRpc;
pub use coordination::CounterClient;
pub use coordination::LeaseClient;
pub use coordination::LeaseGrantResult;
pub use coordination::LeaseInfoLocal;
pub use coordination::LeaseKeepaliveHandle;
pub use coordination::LeaseKeepaliveResult;
pub use coordination::LeaseRevokeResult;
pub use coordination::LeaseTimeToLiveResult;
pub use coordination::LockClient;
pub use coordination::QueueClient;
pub use coordination::QueueCreateConfig;
pub use coordination::QueueDLQItemInfo;
pub use coordination::QueueDequeuedItem;
pub use coordination::QueueEnqueueBatchItem;
pub use coordination::QueueEnqueueOptions;
pub use coordination::QueuePeekedItem;
pub use coordination::QueueStatusInfo;
pub use coordination::RWLockClient;
pub use coordination::RWLockReadResult;
pub use coordination::RWLockStatusResult;
pub use coordination::RWLockWriteResult;
pub use coordination::RateLimitResult;
pub use coordination::RateLimiterClient;
pub use coordination::RemoteLockGuard;
pub use coordination::SemaphoreAcquireResult;
pub use coordination::SemaphoreClient;
pub use coordination::SemaphoreStatusResult;
pub use coordination::SequenceClient;
pub use coordination::ServiceClient;
pub use coordination::ServiceDiscoveryFilter;
pub use coordination::ServiceHeartbeatHandle;
pub use coordination::ServiceInstanceInfo;
pub use coordination::ServiceMetadataUpdate;
pub use coordination::ServiceRegisterOptions;
pub use coordination::ServiceRegistration;
pub use coordination::SignedCounterClient;
pub use overlay::ClientOverlay;
pub use overlay::OverlayError;
pub use overlay::ReadResult;
pub use overlay::WriteResult;
pub use subscription::AccessLevel;
pub use subscription::CacheConfig;
pub use subscription::ClusterSubscription;
pub use subscription::SubscriptionFilter;
pub use ticket::AspenClientTicket;
pub use ticket::CLIENT_TICKET_PREFIX;
pub use transaction::TransactionBuilder;
pub use transaction::TransactionResult;
pub use watch::WatchEvent;
pub use watch::WatchSession;
pub use watch::WatchSubscription;

// Re-export iroh types that clients need
pub use iroh::EndpointAddr;
pub use iroh::EndpointId;
