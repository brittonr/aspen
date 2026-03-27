//! Federation token storage in KV.
//!
//! Stores issued and received federation credentials in the cluster's
//! KV store so they persist across restarts and get presented automatically
//! on outbound connections.
//!
//! Key layout:
//! - `_sys:fed:token:issued:<audience_key>` — tokens we issued to remote clusters
//! - `_sys:fed:token:received:<issuer_key>` — tokens we received from remote clusters

use aspen_auth::Credential;
use iroh::PublicKey;
use tracing::debug;
use tracing::warn;

/// KV prefix for issued federation tokens.
pub const ISSUED_PREFIX: &str = "_sys:fed:token:issued:";

/// KV prefix for received federation tokens.
const RECEIVED_PREFIX: &str = "_sys:fed:token:received:";

/// Build the KV key for an issued token.
fn issued_key(audience: &PublicKey) -> String {
    format!("{}{}", ISSUED_PREFIX, audience)
}

/// Build the KV key for a received token.
fn received_key(issuer: &PublicKey) -> String {
    format!("{}{}", RECEIVED_PREFIX, issuer)
}

/// Store a credential we issued to a remote cluster.
///
/// The credential is serialized to JSON and written at
/// `_sys:fed:token:issued:<audience_key>`.
pub async fn store_issued_token<S: aspen_traits::KeyValueStore>(
    store: &S,
    audience: &PublicKey,
    credential: &Credential,
) -> Result<(), anyhow::Error> {
    let key = issued_key(audience);
    let value = serde_json::to_string(credential)?;
    store.write(aspen_traits::WriteRequest::set(key, value)).await?;
    debug!(audience = %audience, "stored issued federation token");
    Ok(())
}

/// Store a credential we received from a remote cluster.
///
/// The credential is serialized to JSON and written at
/// `_sys:fed:token:received:<issuer_key>`.
pub async fn store_received_token<S: aspen_traits::KeyValueStore>(
    store: &S,
    issuer: &PublicKey,
    credential: &Credential,
) -> Result<(), anyhow::Error> {
    let key = received_key(issuer);
    let value = serde_json::to_string(credential)?;
    store.write(aspen_traits::WriteRequest::set(key, value)).await?;
    debug!(issuer = %issuer, "stored received federation token");
    Ok(())
}

/// Load a stored credential for a remote peer.
///
/// Checks `_sys:fed:token:received:<peer_key>` and returns the credential
/// if it exists and has not expired. Returns `None` if no credential is
/// stored or the stored credential has expired.
pub async fn load_credential_for_peer<S: aspen_traits::KeyValueStore>(
    store: &S,
    peer_key: &PublicKey,
) -> Option<Credential> {
    let key = received_key(peer_key);

    let result = match store.read(aspen_traits::ReadRequest::new(key)).await {
        Ok(r) => r,
        Err(e) => {
            debug!(peer = %peer_key, error = %e, "failed to read federation token from KV");
            return None;
        }
    };

    let kv = result.kv?;

    let credential: Credential = match serde_json::from_str(&kv.value) {
        Ok(c) => c,
        Err(e) => {
            warn!(peer = %peer_key, error = %e, "failed to deserialize stored federation credential");
            return None;
        }
    };

    // Check expiry
    let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

    if credential.token.expires_at < now_secs {
        debug!(
            peer = %peer_key,
            expires_at = credential.token.expires_at,
            now = now_secs,
            "stored federation credential has expired, skipping"
        );
        return None;
    }

    debug!(peer = %peer_key, "loaded federation credential from KV");
    Some(credential)
}
