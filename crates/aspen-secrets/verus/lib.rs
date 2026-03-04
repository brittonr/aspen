//! Verus Formal Specifications for Aspen Secrets
//!
//! This crate contains standalone Verus specifications for verifying the
//! correctness of SOPS MAC computation in Aspen.
//!
//! # Verification
//!
//! Run Verus verification with:
//! ```bash
//! verus --crate-type=lib crates/aspen-secrets/verus/mac_spec.rs
//! ```
//!
//! # Invariants Verified
//!
//! ## SOPS MAC (HMAC-SHA256)
//!
//! 1. **MAC-1: Determinism** — Same inputs always produce the same output. `compute_sops_mac(v, k)
//!    == compute_sops_mac(v, k)` for all v, k.
//!
//! 2. **MAC-2: Key Sensitivity** — Different data keys produce different MACs (modeled via trusted
//!    HMAC axiom, not proven).
//!
//! 3. **MAC-3: Value Sensitivity** — Different values produce different MACs (modeled via trusted
//!    HMAC axiom, not proven).
//!
//! 4. **MAC-4: Path Sensitivity** — Different paths produce different MACs (modeled via trusted
//!    HMAC axiom, not proven).
//!
//! 5. **MAC-5: Sort Stability** — `collect_value_paths` produces a sorted output, ensuring MAC
//!    input order is deterministic regardless of insertion order.
//!
//! # Trusted Axioms
//!
//! The following properties of HMAC-SHA256 are assumed (not proven):
//!
//! 1. **HMAC Determinism** — HMAC-SHA256 is a deterministic function.
//! 2. **HMAC Key Separation** — Different keys produce different MACs for the same message (with
//!    overwhelming probability).
//! 3. **HMAC Collision Resistance** — Different messages produce different MACs for the same key
//!    (with overwhelming probability, bounded by SHA-256 collision resistance ~2^128).
//!
//! These are standard cryptographic assumptions. Proving them would require
//! modeling SHA-256 internals, which is out of scope.

mod mac_spec;
