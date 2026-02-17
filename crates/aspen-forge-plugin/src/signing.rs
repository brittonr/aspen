//! Object signing using host-provided Ed25519 crypto.

use serde::Serialize;

use crate::kv;
use crate::types::SignedObject;

/// Error type for signing operations.
#[derive(Debug)]
pub enum SignError {
    /// Failed to serialize payload for signing.
    SerializationFailed,
}

/// Sign a payload using the host node's Ed25519 key.
///
/// Constructs a `SignedObject<T>` with the node's public key, an HLC timestamp,
/// and a signature over the JSON-serialized `(payload, author, timestamp_ms)` tuple.
///
/// Returns `SignError::SerializationFailed` if the payload cannot be serialized.
/// Tiger Style: No silent failures - serialization errors are propagated.
pub fn sign_object<T: Serialize + Clone>(payload: &T) -> Result<SignedObject<T>, SignError> {
    let author = kv::public_key();
    let timestamp_ms = kv::hlc_now();

    // Tiger Style: Fail explicitly if serialization fails rather than signing empty bytes
    let signable = serde_json::to_vec(&(payload, &author, timestamp_ms)).map_err(|_| SignError::SerializationFailed)?;
    let sig_bytes = kv::sign(&signable);
    let signature = hex::encode(&sig_bytes);

    Ok(SignedObject {
        payload: payload.clone(),
        author,
        timestamp_ms,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_error_debug() {
        // SignError should implement Debug for error reporting
        let err = SignError::SerializationFailed;
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("SerializationFailed"));
    }
}
