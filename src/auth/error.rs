//! Error types for capability-based authorization.
//!
//! Uses thiserror for derive macro error implementation as per project conventions.

use thiserror::Error;

/// Errors that can occur during token operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Token signature verification failed.
    #[error("invalid token signature")]
    InvalidSignature,

    /// Token has expired.
    #[error("token expired at {expired_at}, current time is {now}")]
    TokenExpired {
        /// When the token expired (Unix seconds).
        expired_at: u64,
        /// Current time (Unix seconds).
        now: u64,
    },

    /// Token was issued in the future (clock skew).
    #[error("token issued in future at {issued_at}, current time is {now}")]
    TokenFromFuture {
        /// When the token claims to be issued (Unix seconds).
        issued_at: u64,
        /// Current time (Unix seconds).
        now: u64,
    },

    /// Token audience doesn't match presenter.
    #[error("token audience mismatch: expected {expected}, got {actual}")]
    WrongAudience {
        /// Expected audience.
        expected: String,
        /// Actual presenter.
        actual: String,
    },

    /// Token requires specific audience but none provided.
    #[error("token requires specific audience but none provided")]
    AudienceRequired,

    /// Token has been revoked.
    #[error("token has been revoked")]
    TokenRevoked,

    /// No capability authorizes the requested operation.
    #[error("unauthorized: no capability for operation {operation}")]
    Unauthorized {
        /// The operation that was attempted.
        operation: String,
    },

    /// Token has too many capabilities.
    #[error("too many capabilities: {count} exceeds max {max}")]
    TooManyCapabilities {
        /// Number of capabilities in token.
        count: usize,
        /// Maximum allowed.
        max: u32,
    },

    /// Delegation chain is too deep.
    #[error("delegation chain too deep: {depth} exceeds max {max}")]
    DelegationTooDeep {
        /// Current depth.
        depth: u8,
        /// Maximum allowed.
        max: u8,
    },

    /// Attempted to create a child token with more permissions than parent.
    #[error("capability escalation attempted: {requested}")]
    CapabilityEscalation {
        /// The capability that was requested but not allowed.
        requested: String,
    },

    /// Parent token doesn't have Delegate capability.
    #[error("parent token does not allow delegation")]
    DelegationNotAllowed,

    /// Token exceeds maximum size.
    #[error("token too large: {size} bytes exceeds max {max}")]
    TokenTooLarge {
        /// Actual size in bytes.
        size: usize,
        /// Maximum allowed.
        max: u32,
    },

    /// Error encoding token.
    #[error("token encoding error: {0}")]
    EncodingError(String),

    /// Error decoding token.
    #[error("token decoding error: {0}")]
    DecodingError(String),

    /// No token provided when one was required.
    #[error("no token provided")]
    NoToken,

    /// Internal error (e.g., lock poisoning).
    ///
    /// Tiger Style: Return error instead of panicking on lock poisoning.
    #[error("internal error: {reason}")]
    InternalError {
        /// Description of the internal error.
        reason: String,
    },
}

impl From<postcard::Error> for AuthError {
    fn from(e: postcard::Error) -> Self {
        AuthError::DecodingError(e.to_string())
    }
}

impl From<base64::DecodeError> for AuthError {
    fn from(e: base64::DecodeError) -> Self {
        AuthError::DecodingError(e.to_string())
    }
}
