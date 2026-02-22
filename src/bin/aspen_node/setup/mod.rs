//! Setup orchestration for the aspen-node binary.
//!
//! This module contains the client protocol setup and router configuration
//! for initializing all subsystems of an Aspen node.

pub mod client;
pub mod router;

pub use client::setup_client_protocol;
pub use router::print_cluster_ticket;
pub use router::setup_router;
