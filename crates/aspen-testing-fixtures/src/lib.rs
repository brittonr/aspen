//! Reusable test fixtures and builders for Aspen testing.
//!
//! This crate provides builder patterns for creating test configurations,
//! cluster setups, and mock providers. It is designed to be lightweight
//! with minimal dependencies, suitable for unit tests that don't need
//! full simulation or network infrastructure.
//!
//! # Key Types
//!
//! - [`ClusterBuilder`]: Builder for configuring test cluster setups
//! - [`KvStoreBuilder`]: Builder for creating configured KV stores
//! - [`CoordinationTestHelper`]: Helper for coordination primitive tests
//! - [`MockEndpointProvider`]: Mock implementation for Iroh endpoint testing
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing_fixtures::{ClusterBuilder, KvStoreBuilder};
//!
//! // Create a 3-node test cluster configuration
//! let cluster = ClusterBuilder::new()
//!     .with_nodes(3)
//!     .with_timeout_ms(5000)
//!     .build();
//!
//! // Create an in-memory KV store for testing
//! let kv = KvStoreBuilder::new()
//!     .with_prefix("test/")
//!     .build();
//! ```

pub mod builders;
pub mod coordination_helper;
pub mod mock_endpoint;

pub use builders::ClusterBuilder;
pub use builders::KvStoreBuilder;
pub use coordination_helper::CoordinationTestHelper;
pub use mock_endpoint::MockEndpointProvider;
