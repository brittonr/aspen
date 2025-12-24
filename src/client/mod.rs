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
pub use constants::CONNECTION_TIMEOUT;
pub use constants::DEFAULT_CACHE_TTL;
pub use constants::HEARTBEAT_INTERVAL;
pub use constants::MAX_CACHE_TTL;
pub use constants::MAX_RECONNECT_DELAY;
pub use constants::MAX_SUBSCRIPTIONS;
pub use constants::MAX_TOTAL_CACHE_ENTRIES;
pub use constants::RECONNECT_DELAY;
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
