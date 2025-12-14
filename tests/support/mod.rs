//! Test support utilities for Aspen.
//!
//! This module provides testing infrastructure including:
//! - Mock gossip for fast, deterministic peer discovery tests
//! - Test helpers for cluster bootstrap and configuration
//! - Proptest generators for property-based testing

pub mod mock_gossip;
pub mod proptest_generators;
