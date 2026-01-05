//! Pure client library for Aspen distributed systems.
//!
//! This crate provides a lightweight client for connecting to Aspen clusters
//! without pulling in the heavy raft/cluster dependencies. It is designed for
//! external consumers who only need to interact with Aspen as a client.

// Allow dead code and unused imports as this is a public API crate with many optional features
#![allow(dead_code, unused_imports)]
//!
//! # Key Components
//!
//! - [`AspenClient`]: Main client for sending RPC requests to cluster nodes
//! - [`ClientRpcRequest`]: All available RPC request types
//! - [`ClientRpcResponse`]: All available RPC response types
//! - [`AspenClusterTicket`]: Ticket for bootstrapping connections (legacy)
//! - [`AspenClientTicket`]: Client ticket for connection access control
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
pub mod blob_client;
pub mod cache;
pub mod coordination;
pub mod job_client;
pub mod observability;
pub mod overlay;
pub mod subscription;
pub mod transaction;
pub mod watch;

// Re-export all public types at crate root
// Re-export RPC types from aspen-client-rpc
// Re-export RPC response types from aspen-client-rpc
// These must match the types used in ClientRpcResponse variants
pub use aspen_client_rpc::AddLearnerResultResponse;
pub use aspen_client_rpc::AuthenticatedRequest;
pub use aspen_client_rpc::ChangeMembershipResultResponse;
pub use aspen_client_rpc::CheckpointWalResultResponse;
pub use aspen_client_rpc::ClientRpcRequest;
pub use aspen_client_rpc::ClientRpcResponse;
pub use aspen_client_rpc::ClusterStateResponse;
pub use aspen_client_rpc::ClusterTicketResponse;
pub use aspen_client_rpc::DeleteResultResponse;
pub use aspen_client_rpc::ErrorResponse;
pub use aspen_client_rpc::HealthResponse;
pub use aspen_client_rpc::InitResultResponse;
pub use aspen_client_rpc::MetricsResponse;
pub use aspen_client_rpc::NodeDescriptor;
pub use aspen_client_rpc::NodeInfoResponse;
pub use aspen_client_rpc::PromoteLearnerResultResponse;
pub use aspen_client_rpc::RaftMetricsResponse;
pub use aspen_client_rpc::ReadResultResponse;
pub use aspen_client_rpc::ReplicationProgress;
pub use aspen_client_rpc::ScanEntry;
pub use aspen_client_rpc::ScanResultResponse;
pub use aspen_client_rpc::SnapshotResultResponse;
// Re-export SQL types from aspen-client-rpc
pub use aspen_client_rpc::SqlCellValue;
pub use aspen_client_rpc::SqlResultResponse;
pub use aspen_client_rpc::VaultInfo;
pub use aspen_client_rpc::VaultKeysResponse;
pub use aspen_client_rpc::VaultListResponse;
pub use aspen_client_rpc::WriteResultResponse;
// Re-export blob client types
pub use blob_client::AspenClientBlobExt;
pub use blob_client::BlobClient;
pub use blob_client::BlobDownloadResult;
pub use blob_client::BlobEntry;
pub use blob_client::BlobListOptions;
pub use blob_client::BlobListResult;
pub use blob_client::BlobStatus;
pub use blob_client::BlobUploadResult;
// Re-export overlay and coordination types
pub use cache::LocalCache;
pub use client::AspenClient;
pub use client::AspenClusterTicket;
pub use client::AuthToken;
pub use constants::CLIENT_ALPN;
pub use constants::MAX_CLIENT_MESSAGE_SIZE;
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
// Re-export iroh types that clients need
pub use iroh::EndpointAddr;
pub use iroh::EndpointId;
// Re-export job client types
pub use job_client::AspenClientJobExt;
pub use job_client::JobClient;
pub use job_client::JobListOptions;
pub use job_client::JobListResult;
pub use job_client::JobPriority;
pub use job_client::JobQueueStats;
pub use job_client::JobStatus;
pub use job_client::JobSubmitBuilder;
// Re-export observability types
pub use observability::AspenClientObservabilityExt;
pub use observability::HistogramStats;
pub use observability::MetricsCollector;
pub use observability::ObservabilityBuilder;
pub use observability::ObservabilityClient;
pub use observability::Span;
pub use observability::SpanEvent;
pub use observability::SpanStatus;
pub use observability::TraceContext;
pub use overlay::ClientOverlay;
pub use overlay::OverlayError;
pub use overlay::ReadResult;
pub use overlay::WriteResult;
pub use overlay_constants::CONNECTION_TIMEOUT;
pub use overlay_constants::DEFAULT_CACHE_TTL;
pub use overlay_constants::HEARTBEAT_INTERVAL;
pub use overlay_constants::MAX_CACHE_TTL;
pub use overlay_constants::MAX_RECONNECT_DELAY;
pub use overlay_constants::MAX_SUBSCRIPTIONS;
pub use overlay_constants::MAX_TOTAL_CACHE_ENTRIES;
pub use overlay_constants::RECONNECT_DELAY;
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
