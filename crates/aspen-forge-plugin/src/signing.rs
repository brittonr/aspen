//! Object signing using host-provided Ed25519 crypto.

use serde::Serialize;

use crate::kv;
use crate::types::SignedObject;

/// Sign a payload using the host node's Ed25519 key.
///
/// Constructs a `SignedObject<T>` with the node's public key, an HLC timestamp,
/// and a signature over the JSON-serialized `(payload, author, timestamp_ms)` tuple.
pub fn sign_object<T: Serialize + Clone>(payload: &T) -> SignedObject<T> {
    let author = kv::public_key();
    let timestamp_ms = kv::hlc_now();

    let signable = serde_json::to_vec(&(payload, &author, timestamp_ms)).unwrap_or_default();
    let sig_bytes = kv::sign(&signable);
    let signature = hex::encode(&sig_bytes);

    SignedObject {
        payload: payload.clone(),
        author,
        timestamp_ms,
        signature,
    }
}
