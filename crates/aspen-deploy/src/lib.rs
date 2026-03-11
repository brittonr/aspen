//! Deployment orchestration for Aspen rolling upgrades.
//!
//! This crate provides the deployment state machine, quorum safety computations,
//! and type definitions for coordinating rolling upgrades across an Aspen cluster.
//!
//! # Architecture
//!
//! Follows the Functional Core, Imperative Shell pattern:
//! - **`src/verified/`**: Pure functions for quorum safety, state transitions. Formally verified
//!   via Verus specs in `verus/`.
//! - **Shell layer** (future): Async coordinator that calls verified functions, manages KV state,
//!   and orchestrates upgrades.
//!
//! # Types
//!
//! - [`DeploymentStatus`]: Overall deployment lifecycle state
//! - [`NodeDeployStatus`]: Per-node upgrade state
//! - [`DeploymentRecord`]: Full deployment record stored in KV
//! - [`DeployArtifact`]: What binary to deploy (Nix store path or blob hash)
//! - [`DeployStrategy`]: How to roll out (currently just rolling)

pub mod types;
pub mod verified;

// Re-export types for convenience
pub use types::*;
