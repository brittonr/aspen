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
use aspen_traits::KvRead;
use aspen_traits::KvWrite;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_traits::KeyValueStore;

    use super::*;

    fn make_store() -> Arc<aspen_testing::DeterministicKeyValueStore> {
        aspen_testing::DeterministicKeyValueStore::new()
    }

    fn make_credential(lifetime_secs: u64) -> Credential {
        let secret = iroh::SecretKey::generate(&mut rand::rng());
        let token = aspen_auth::TokenBuilder::new(secret)
            .with_lifetime(std::time::Duration::from_secs(lifetime_secs))
            .build()
            .expect("build token");
        Credential::from_root(token)
    }

    #[tokio::test]
    async fn test_load_returns_none_when_key_missing() {
        let store = make_store();
        let peer = iroh::SecretKey::generate(&mut rand::rng()).public();
        let result = load_credential_for_peer(store.as_ref(), &peer).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_store_and_load_credential() {
        let store = make_store();
        let peer = iroh::SecretKey::generate(&mut rand::rng()).public();
        // 1 hour lifetime — well in the future
        let cred = make_credential(3600);
        let issued = cred.token.issued_at;
        store_received_token(store.as_ref(), &peer, &cred).await.unwrap();

        let loaded = load_credential_for_peer(store.as_ref(), &peer).await;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().token.issued_at, issued);
    }

    #[tokio::test]
    async fn test_expired_credential_returns_none() {
        let store = make_store();
        let peer = iroh::SecretKey::generate(&mut rand::rng()).public();
        // Build a valid credential, then manually set expires_at to the past
        let mut cred = make_credential(3600);
        cred.token.expires_at = 1; // 1970-01-01T00:00:01 — clearly expired
        store_received_token(store.as_ref(), &peer, &cred).await.unwrap();

        let loaded = load_credential_for_peer(store.as_ref(), &peer).await;
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_corrupt_json_returns_none() {
        let store = make_store();
        let peer = iroh::SecretKey::generate(&mut rand::rng()).public();
        let key = received_key(&peer);
        // Write garbage JSON
        store.write(aspen_traits::WriteRequest::set(key, "not-valid-json".to_string())).await.unwrap();

        let loaded = load_credential_for_peer(store.as_ref(), &peer).await;
        assert!(loaded.is_none());
    }
}
