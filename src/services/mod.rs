//! Service layer abstractions and implementations
//!
//! This module provides trait-based abstractions for infrastructure services,
//! enabling testability, dependency injection, and flexible implementations.

pub mod traits;
pub mod mocks;

// Re-export main traits for convenience
pub use traits::{
    // Iroh networking
    EndpointInfo,
    BlobStorage,
    GossipNetwork,
    PeerConnection,
    // Hiqlite database
    DatabaseQueries,
    DatabaseHealth,
    DatabaseSchema,
    DatabaseLifecycle,
    // Common types
    EndpointId,
    EndpointAddr,
    Param,
};
