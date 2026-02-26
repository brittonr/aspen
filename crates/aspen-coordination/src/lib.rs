//! Distributed coordination primitives built on CAS operations.
//!
//! This module provides high-level coordination primitives for distributed systems:
//!
//! - `DistributedLock` - Mutual exclusion with fencing tokens
//! - `LeaderElection` - Leader election with automatic lease renewal
//! - `AtomicCounter` - Race-free increment/decrement
//! - `SequenceGenerator` - Monotonically increasing unique IDs
//! - `DistributedRateLimiter` - Token bucket rate limiting
//! - `QueueManager` - Distributed FIFO queue with visibility timeout
//! - `ServiceRegistry` - Service discovery with health checks
//!
//! All primitives are built on top of the [`aspen_core::KeyValueStore`] trait's CAS operations,
//! providing linearizable semantics through Raft consensus.
//!
//! ## Leader Election Example
//!
//! ```ignore
//! use aspen_coordination::{LeaderElection, ElectionConfig};
//!
//! let election = LeaderElection::new(
//!     store,
//!     "my-service-leader",
//!     "node-1",
//!     ElectionConfig::default(),
//! );
//!
//! let handle = election.start().await?;
//!
//! // Check leadership state
//! if handle.is_leader() {
//!     let token = handle.fencing_token().unwrap();
//!     // Perform leader-only operations with fencing token
//! }
//!
//! // Graceful stepdown
//! handle.stepdown();
//! ```
//!
//! ## Lock Example
//!
//! ```ignore
//! use aspen_coordination::{DistributedLock, LockConfig};
//!
//! let lock = DistributedLock::new(store, "my_lock", "client_1", LockConfig::default());
//! let guard = lock.acquire().await?;
//!
//! // Protected critical section
//! // Fencing token can be passed to external services
//! let token = guard.fencing_token();
//!
//! // Lock released on drop
//! ```

mod barrier;
mod counter;
mod election;
mod error;
mod lock;
mod queue;
mod rate_limiter;
mod registry;
mod rwlock;
mod semaphore;
mod sequence;
pub mod spec;
mod types;
pub mod verified;
mod worker_coordinator;
mod worker_strategies;

pub use barrier::BarrierManager;
pub use barrier::BarrierPhase;
pub use barrier::BarrierState;
pub use counter::AtomicCounter;
pub use counter::BufferedCounter;
pub use counter::CounterConfig;
pub use counter::SignedAtomicCounter;
pub use election::ElectionConfig;
pub use election::ElectionHandle;
pub use election::LeaderElection;
pub use election::LeadershipState;
pub use error::CoordinationError;
pub use error::FenceError;
pub use error::RateLimitError;
pub use lock::DistributedLock;
pub use lock::LockConfig;
pub use lock::LockGuard;
pub use queue::DLQItem;
pub use queue::DLQReason;
pub use queue::DequeuedItem;
pub use queue::EnqueueOptions;
pub use queue::PendingItem;
pub use queue::QueueConfig;
pub use queue::QueueItem;
pub use queue::QueueManager;
pub use queue::QueueState;
pub use queue::QueueStats;
pub use queue::QueueStatus;
pub use rate_limiter::DistributedRateLimiter;
pub use rate_limiter::RateLimiterConfig;
pub use registry::DiscoveryFilter;
pub use registry::HealthStatus;
pub use registry::RegisterOptions;
pub use registry::ServiceInstance;
pub use registry::ServiceInstanceMetadata;
pub use registry::ServiceMetadata;
pub use registry::ServiceRegistry;
pub use rwlock::RWLockManager;
pub use rwlock::RWLockMode;
pub use rwlock::RWLockState;
pub use rwlock::ReaderEntry;
pub use rwlock::WriterEntry;
pub use semaphore::SemaphoreHolder;
pub use semaphore::SemaphoreManager;
pub use semaphore::SemaphoreState;
pub use sequence::SequenceConfig;
pub use sequence::SequenceGenerator;
pub use types::BucketState;
pub use types::FencingToken;
pub use types::LockEntry;
pub use types::now_unix_ms;
pub use worker_coordinator::DistributedWorkerCoordinator;
pub use worker_coordinator::GroupState;
pub use worker_coordinator::LoadBalancingStrategy;
pub use worker_coordinator::StealHint;
pub use worker_coordinator::WorkerCoordinatorConfig;
pub use worker_coordinator::WorkerFilter;
pub use worker_coordinator::WorkerGroup;
pub use worker_coordinator::WorkerInfo;
pub use worker_coordinator::WorkerStats;
pub use worker_strategies::AffinityStrategy;
pub use worker_strategies::ConsistentHashStrategy;
pub use worker_strategies::LeastLoadedStrategy;
pub use worker_strategies::LoadBalancer;
pub use worker_strategies::Priority;
pub use worker_strategies::RoundRobinStrategy;
pub use worker_strategies::RoutingContext;
pub use worker_strategies::StrategyMetrics;
pub use worker_strategies::WorkStealingStrategy;
