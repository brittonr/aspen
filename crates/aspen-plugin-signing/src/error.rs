//! Error types for plugin signing operations.

/// Errors from plugin signing and verification.
#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    /// The hex string could not be decoded.
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    /// The Ed25519 public key bytes are invalid.
    #[error("invalid public key: {0}")]
    InvalidPublicKey(#[from] ed25519_dalek::SignatureError),

    /// The signature bytes have wrong length (expected 64).
    #[error("invalid signature length: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),

    /// The public key bytes have wrong length (expected 32).
    #[error("invalid public key length: expected 32 bytes, got {0}")]
    InvalidPublicKeyLength(usize),

    /// The WASM hash does not match the binary.
    #[error("WASM hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// The signature did not verify against the public key.
    #[error("signature verification failed")]
    VerificationFailed,

    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
