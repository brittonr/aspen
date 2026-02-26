//! Application registry for federation discovery.
//!
//! This module re-exports the app registry from `aspen_core::app_registry`.
//! The canonical implementation lives in `aspen-core` to allow shared access
//! from both the federation layer and the RPC dispatch layer without circular
//! dependencies.

pub use aspen_core::app_registry::*;
