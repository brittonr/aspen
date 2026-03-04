//! Lightweight cryptographic primitives for Aspen.
//!
//! This crate provides the foundational crypto operations needed by most Aspen
//! crates without pulling in heavy dependencies (age, rcgen, x509-cert, etc.).
//!
//! ## Modules
//!
//! - **`cookie`**: Cluster cookie validation, HMAC key derivation, gossip topic derivation
//! - **`identity`**: Node identity key lifecycle (generate, parse, load/save)
//!
//! ## Design
//!
//! `aspen-crypto` has zero aspen-* dependencies. It sits at the bottom of the
//! dependency tree so any crate can use it without pulling in the world.
//!
//! For higher-level secrets management (KV, Transit, PKI, SOPS), see `aspen-secrets`
//! which re-exports these modules and adds the full secrets engine.

pub mod cookie;
pub mod identity;
