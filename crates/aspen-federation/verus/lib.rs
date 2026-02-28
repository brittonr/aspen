//! Verus formal verification specifications for aspen-federation.
//!
//! These specs prove correctness properties of federation's pure functions.
//! Production code lives in `src/verified/` and compiles with standard cargo.
//! These specs are verified separately via `nix run .#verify-verus federation`.
//!
//! # Invariants Verified
//!
//! ## Seeder Quorum (QUORUM)
//!
//! 1. **QUORUM-1: Threshold Minimum**: Quorum threshold is always >= 1
//! 2. **QUORUM-2: Majority**: For N trusted seeders, quorum = (N/2) + 1
//! 3. **QUORUM-3: Monotonicity**: More seeders → same or higher quorum threshold
//! 4. **QUORUM-4: Untrusted Exclusion**: Untrusted seeders never count toward quorum
//!
//! ## Fork Detection (FORK)
//!
//! 1. **FORK-1: Agreement**: No fork reported when all seeders agree
//! 2. **FORK-2: Detection**: Fork detected when any two seeders disagree on a ref
//! 3. **FORK-3: Branch Completeness**: Every distinct hash value appears as a branch
//! 4. **FORK-4: Supporter Accuracy**: Each supporter appears in exactly one branch
//!
//! ## Content Verification (VERIFY)
//!
//! 1. **VERIFY-1: Hash Integrity**: `verify_content_hash(data, blake3(data))` always true
//! 2. **VERIFY-2: Tamper Detection**: Modifying any byte causes hash verification to fail
//!
//! # Trusted Axioms
//!
//! 1. **BLAKE3 Collision Resistance**: Two distinct inputs produce distinct BLAKE3 hashes
//! 2. **Ed25519 Unforgeability**: Signatures cannot be forged without the private key
//! 3. **Bounded Arithmetic**: u32 in [0, 2^32-1], u64 in [0, 2^64-1]

mod fork_detection_spec;
mod quorum_spec;

fn main() {}
