//! Test support utilities for Aspen.
//!
//! This module provides testing infrastructure including:
//! - Mock Iroh infrastructure for fast, deterministic networking tests
//! - Mock gossip for fast, deterministic peer discovery tests
//! - Test helpers for cluster bootstrap and configuration
//! - Bolero generators for unified property-based testing (libFuzzer, AFL, Honggfuzz, Kani, Miri)
//! - Proptest generators for madsim compatibility (proptest used inside madsim tests)
//! - RPC handler generators for coordination and KV operation testing
//! - Pijul multi-node tester for P2P synchronization tests

pub mod bolero_generators;
pub mod mock_gossip;
pub mod mock_iroh;

pub mod proptest_generators;
#[cfg(all(feature = "jobs", feature = "docs", feature = "hooks", feature = "federation"))]
pub mod real_cluster;
pub mod rpc_handler_generators;
