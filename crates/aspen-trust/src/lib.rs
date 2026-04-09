//! Shamir secret sharing and cluster trust primitives for Aspen.
//!
//! This crate implements GF(2^8) Shamir secret sharing for distributing a
//! 32-byte cluster root secret across N nodes with a K-of-N reconstruction
//! threshold. Inspired by Oxide's trust-quorum and `gfss` crate.
//!
//! ## Modules
//!
//! - **`gf256`**: GF(2^8) finite field arithmetic (multiplication, polynomial evaluation)
//! - **`shamir`**: Secret splitting and reconstruction over 32 independent polynomials
//! - **`secret`**: `ClusterSecret` type with zeroize-on-drop and constant-time equality
//! - **`kdf`**: HKDF-SHA3-256 key derivation with scoped context strings
//!
//! ## Design
//!
//! Each byte of the 32-byte secret is treated as an independent GF(2^8) polynomial.
//! Splitting evaluates 32 random polynomials at N distinct nonzero x-coordinates.
//! Reconstruction uses Lagrange interpolation at x=0 to recover the constant terms.
//! Shares are 33 bytes: 1-byte x-coordinate + 32-byte y-values.
//!
//! Memory safety: all secret material implements `Zeroize` and `ZeroizeOnDrop`.
//! Comparisons use `subtle::ConstantTimeEq` to prevent timing side channels.

pub mod chain;
pub mod encryption;
pub mod envelope;
pub mod gf256;
pub mod kdf;
pub mod nonce;
pub mod protocol;
pub mod reconfig;
pub mod secret;
pub mod shamir;
pub mod verified;
