//! Client overlay system for multi-cluster subscriptions.
//!
//! This module provides client-side support for subscribing to multiple
//! Aspen clusters with priority-based routing, similar to Nix binary caches.
//!
//! ## Architecture
//!
//! ```text
//! Application
//!     |
//!     v
//! ClientOverlay
//!     |
//!     +-- Priority 0: Production cluster (read-write)
//!     |       |
//!     |       +-- LocalCache (LRU with TTL)
//!     |
//!     +-- Priority 1: Staging cluster (read-only)
//!     |       |
//!     |       +-- LocalCache
//!     |
//!     +-- Priority 2: Dev cluster (read-only)
//!             |
//!             +-- LocalCache
//! ```
//!
//! ## Read Path
//!
//! 1. Check local cache for each subscription in priority order
//! 2. Query cluster if not cached
//! 3. Return first match (no merge between clusters)
//!
//! ## Write Path
//!
//! 1. Find first subscription with write access
//! 2. Route write to that cluster's Raft leader
//! 3. Invalidate cache for affected keys
//!
//! ## Usage
//!
//! ```ignore
//! use aspen::client::{ClientOverlay, ClusterSubscription, AccessLevel};
//!
//! // Create overlay
//! let overlay = ClientOverlay::new();
//!
//! // Add subscriptions
//! overlay.add_subscription(
//!     ClusterSubscription::new("prod", "Production", "cluster-id")
//!         .with_priority(0)
//!         .with_access(AccessLevel::ReadWrite)
//! ).await?;
//!
//! overlay.add_subscription(
//!     ClusterSubscription::new("staging", "Staging", "cluster-id-2")
//!         .with_priority(1)
//! ).await?;
//!
//! // Read (checks prod first, then staging)
//! let result = overlay.read("config:app").await?;
//!
//! // Write (goes to prod - first read-write subscription)
//! overlay.write("config:app", "new-value").await?;
//! ```

pub mod cache;
pub mod constants;
pub mod coordination;
pub mod overlay;
pub mod subscription;
pub mod ticket;
pub mod transaction;
pub mod watch;

pub use cache::LocalCache;
pub use constants::{
    CONNECTION_TIMEOUT, DEFAULT_CACHE_TTL, HEARTBEAT_INTERVAL, MAX_CACHE_TTL, MAX_RECONNECT_DELAY,
    MAX_SUBSCRIPTIONS, MAX_TOTAL_CACHE_ENTRIES, RECONNECT_DELAY,
};
pub use coordination::{
    BarrierClient, BarrierEnterResult, BarrierLeaveResult, BarrierStatusResult, CoordinationRpc,
    CounterClient, LeaseClient, LeaseGrantResult, LeaseInfoLocal, LeaseKeepaliveHandle,
    LeaseKeepaliveResult, LeaseRevokeResult, LeaseTimeToLiveResult, LockClient, QueueClient,
    QueueCreateConfig, QueueDLQItemInfo, QueueDequeuedItem, QueueEnqueueBatchItem,
    QueueEnqueueOptions, QueuePeekedItem, QueueStatusInfo, RWLockClient, RWLockReadResult,
    RWLockStatusResult, RWLockWriteResult, RateLimitResult, RateLimiterClient, RemoteLockGuard,
    SemaphoreAcquireResult, SemaphoreClient, SemaphoreStatusResult, SequenceClient, ServiceClient,
    ServiceDiscoveryFilter, ServiceHeartbeatHandle, ServiceInstanceInfo, ServiceMetadataUpdate,
    ServiceRegisterOptions, ServiceRegistration, SignedCounterClient,
};
pub use overlay::{ClientOverlay, OverlayError, ReadResult, WriteResult};
pub use subscription::{AccessLevel, CacheConfig, ClusterSubscription, SubscriptionFilter};
pub use ticket::{AspenClientTicket, CLIENT_TICKET_PREFIX};
pub use transaction::{TransactionBuilder, TransactionResult};
pub use watch::{WatchEvent, WatchSession, WatchSubscription};
