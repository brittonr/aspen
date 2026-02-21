//! Plugin signature type.

use serde::Deserialize;
use serde::Serialize;

/// Ed25519 signature over a WASM plugin binary.
///
/// The signing process:
/// 1. Compute `blake3::hash(wasm_bytes)` → 32-byte hash
/// 2. Sign the hash with Ed25519 → 64-byte signature
/// 3. Encode all binary fields as hex strings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginSignature {
    /// Ed25519 public key of the signer (64 hex chars = 32 bytes).
    pub author_pubkey: String,
    /// Ed25519 signature over the BLAKE3 hash (128 hex chars = 64 bytes).
    pub signature: String,
    /// BLAKE3 hash of the signed WASM binary (64 hex chars = 32 bytes).
    pub wasm_hash: String,
    /// Timestamp of signing (Unix milliseconds).
    pub signed_at_ms: u64,
}
