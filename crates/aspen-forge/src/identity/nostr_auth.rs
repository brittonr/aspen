//! Nostr challenge-response authentication.
//!
//! Proves npub ownership via secp256k1 Schnorr signature over a random
//! challenge. On success, issues a capability token carrying the npub
//! and resolves the assigned ed25519 keypair from the identity store.
//!
//! ```text
//!   Client                        Server
//!   ──────                        ──────
//!   1. "I am npub X"        ───►
//!                            ◄───  2. challenge_id + 32 random bytes
//!   3. sign(challenge, nsec) ───►
//!                            ◄───  4. capability token with npub fact
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use iroh::SecretKey;
use tokio::sync::Mutex;
use tracing::debug;

use super::nostr_mapping::NostrIdentityStore;
use super::nostr_mapping::UserContext;
use crate::error::ForgeError;
use crate::error::ForgeResult;

/// Maximum pending challenges.
const MAX_PENDING_CHALLENGES: usize = 1000;

/// Challenge validity duration.
const CHALLENGE_TTL_SECS: u64 = 60;

/// Fact key for npub in capability tokens.
pub const TOKEN_FACT_NPUB: &str = "npub";

/// A pending authentication challenge.
struct PendingChallenge {
    npub_hex: String,
    challenge_bytes: [u8; 32],
    created: Instant,
}

/// Nostr authentication service.
///
/// Manages challenge-response flows and issues capability tokens
/// on successful verification.
pub struct NostrAuthService<K: ?Sized> {
    identity_store: Arc<NostrIdentityStore<K>>,
    /// Cluster secret key for signing capability tokens.
    cluster_key: SecretKey,
    /// Pending challenges indexed by challenge_id.
    pending: Mutex<HashMap<String, PendingChallenge>>,
}

impl<K: aspen_core::KeyValueStore + ?Sized + 'static> NostrAuthService<K> {
    /// Create a new auth service.
    pub fn new(identity_store: Arc<NostrIdentityStore<K>>, cluster_key: SecretKey) -> Self {
        Self {
            identity_store,
            cluster_key,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Step 1: Create a challenge for an npub.
    ///
    /// Returns (challenge_id, challenge_bytes).
    // r[impl identity.auth.challenge-unique]
    pub async fn create_challenge(&self, npub_hex: &str) -> ForgeResult<(String, [u8; 32])> {
        let mut pending = self.pending.lock().await;

        // Evict expired challenges
        let now = Instant::now();
        pending.retain(|_, c| now.duration_since(c.created).as_secs() < CHALLENGE_TTL_SECS);

        if pending.len() >= MAX_PENDING_CHALLENGES {
            return Err(ForgeError::InvalidCobChange {
                message: "too many pending auth challenges".to_string(),
            });
        }

        let mut challenge_bytes = [0u8; 32];
        getrandom::fill(&mut challenge_bytes).map_err(|e| ForgeError::KvStorage {
            message: format!("failed to generate challenge: {e}"),
        })?;

        // challenge_id is the hex of the first 16 bytes (unique enough)
        let challenge_id = hex::encode(&challenge_bytes[..16]);

        pending.insert(challenge_id.clone(), PendingChallenge {
            npub_hex: npub_hex.to_string(),
            challenge_bytes,
            created: Instant::now(),
        });

        debug!(npub = npub_hex, challenge_id = %challenge_id, "auth challenge created");

        Ok((challenge_id, challenge_bytes))
    }

    /// Step 2: Verify a signed challenge and return the user context.
    ///
    /// The signature must be a 64-byte Schnorr signature (BIP-340) over
    /// the challenge bytes, verifiable against the claimed npub.
    // r[impl identity.auth.verify-signature]
    // r[impl identity.auth.reject-invalid]
    // r[impl identity.auth.reject-unknown]
    // r[impl identity.auth.stable-key]
    pub async fn verify_challenge(
        &self,
        npub_hex: &str,
        challenge_id: &str,
        signature_hex: &str,
    ) -> ForgeResult<UserContext> {
        // Extract and remove the pending challenge
        let challenge = {
            let mut pending = self.pending.lock().await;
            pending.remove(challenge_id)
        };

        let challenge = challenge.ok_or_else(|| ForgeError::InvalidCobChange {
            message: "unknown or expired challenge".to_string(),
        })?;

        // Check expiry
        if Instant::now().duration_since(challenge.created).as_secs() >= CHALLENGE_TTL_SECS {
            return Err(ForgeError::InvalidCobChange {
                message: "challenge expired".to_string(),
            });
        }

        // Check npub matches
        if challenge.npub_hex != npub_hex {
            return Err(ForgeError::InvalidCobChange {
                message: "npub mismatch".to_string(),
            });
        }

        // Parse the public key and signature
        let sig_bytes = hex::decode(signature_hex).map_err(|e| ForgeError::InvalidCobChange {
            message: format!("invalid signature hex: {e}"),
        })?;

        // Verify secp256k1 Schnorr signature over the challenge bytes
        verify_schnorr(npub_hex, &challenge.challenge_bytes, &sig_bytes)?;

        debug!(npub = npub_hex, "auth challenge verified");

        // Resolve or create the ed25519 keypair for this npub
        self.identity_store.get_or_create(npub_hex).await
    }

    /// Build a capability token carrying the npub as a fact.
    // r[impl identity.auth.token-npub]
    pub fn issue_token(&self, user_ctx: &UserContext) -> ForgeResult<aspen_auth::CapabilityToken> {
        use std::time::Duration;

        let token = aspen_auth::TokenBuilder::new(self.cluster_key.clone())
            .for_key(user_ctx.public_key)
            .with_capability(aspen_auth::Capability::Full {
                prefix: "_forge:".to_string(),
            })
            .with_lifetime(Duration::from_secs(24 * 3600)) // 24 hours
            .with_fact(TOKEN_FACT_NPUB, user_ctx.npub.as_bytes().to_vec())
            .build()
            .map_err(|e| ForgeError::InvalidCobChange {
                message: format!("failed to build auth token: {e}"),
            })?;

        Ok(token)
    }

    /// Extract npub from a capability token's facts.
    pub fn npub_from_token(token: &aspen_auth::CapabilityToken) -> Option<String> {
        token
            .facts
            .iter()
            .find(|(k, _)| k == TOKEN_FACT_NPUB)
            .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
    }
}

/// Verify a BIP-340 Schnorr signature over a message.
/// Verify a BIP-340 Schnorr signature over SHA-256(message).
fn verify_schnorr(npub_hex: &str, message: &[u8], sig_bytes: &[u8]) -> ForgeResult<()> {
    use nostr::secp256k1::Message;
    use nostr::secp256k1::Secp256k1;

    if sig_bytes.len() != 64 {
        return Err(ForgeError::InvalidCobChange {
            message: format!("signature must be 64 bytes, got {}", sig_bytes.len()),
        });
    }

    let pubkey_bytes = hex::decode(npub_hex).map_err(|e| ForgeError::InvalidCobChange {
        message: format!("invalid npub hex: {e}"),
    })?;
    let xonly =
        nostr::secp256k1::XOnlyPublicKey::from_slice(&pubkey_bytes).map_err(|e| ForgeError::InvalidCobChange {
            message: format!("invalid secp256k1 public key: {e}"),
        })?;

    let msg_hash = bitcoin_hashes::sha256::Hash::hash(message);
    let msg = Message::from_digest(msg_hash.to_byte_array());

    let sig =
        nostr::secp256k1::schnorr::Signature::from_slice(sig_bytes).map_err(|e| ForgeError::InvalidCobChange {
            message: format!("invalid schnorr signature: {e}"),
        })?;

    let secp = Secp256k1::verification_only();
    secp.verify_schnorr(&sig, &msg, &xonly).map_err(|e| ForgeError::InvalidCobChange {
        message: format!("signature verification failed: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;

    async fn test_auth_service() -> (NostrAuthService<dyn aspen_core::KeyValueStore>, nostr::Keys) {
        let kv: Arc<dyn aspen_core::KeyValueStore> = DeterministicKeyValueStore::new();
        let cluster_key = SecretKey::generate(&mut ::rand::rng());
        let store = Arc::new(NostrIdentityStore::new(kv, &cluster_key));
        let service = NostrAuthService::new(store, cluster_key);
        let nostr_keys = nostr::Keys::generate();
        (service, nostr_keys)
    }

    fn sign_challenge(keys: &nostr::Keys, challenge: &[u8; 32]) -> String {
        use nostr::secp256k1::Message;
        use nostr::secp256k1::Secp256k1;

        let msg_hash = bitcoin_hashes::sha256::Hash::hash(challenge);
        let msg = Message::from_digest(msg_hash.to_byte_array());
        let secp = Secp256k1::new();
        let keypair = keys.secret_key().keypair(&secp);
        let sig = secp.sign_schnorr(&msg, &keypair);
        hex::encode(sig.serialize())
    }

    // r[verify identity.auth.challenge-unique]
    // r[verify identity.auth.verify-signature]
    // r[verify identity.auth.token-npub]
    #[tokio::test]
    async fn full_challenge_response_cycle() {
        let (service, keys) = test_auth_service().await;
        let npub_hex = keys.public_key().to_hex();

        let (challenge_id, challenge_bytes) = service.create_challenge(&npub_hex).await.unwrap();
        let sig = sign_challenge(&keys, &challenge_bytes);

        let ctx = service.verify_challenge(&npub_hex, &challenge_id, &sig).await.unwrap();
        assert_eq!(ctx.npub, npub_hex);

        // Token should carry the npub
        let token = service.issue_token(&ctx).unwrap();
        let extracted = NostrAuthService::<dyn aspen_core::KeyValueStore>::npub_from_token(&token);
        assert_eq!(extracted.as_deref(), Some(npub_hex.as_str()));
    }

    // r[verify identity.auth.reject-invalid]
    #[tokio::test]
    async fn invalid_signature_rejected() {
        let (service, keys) = test_auth_service().await;
        let npub_hex = keys.public_key().to_hex();

        let (challenge_id, _challenge_bytes) = service.create_challenge(&npub_hex).await.unwrap();

        // Sign wrong data
        let bad_sig = hex::encode([0u8; 64]);
        let result = service.verify_challenge(&npub_hex, &challenge_id, &bad_sig).await;
        assert!(result.is_err());
    }

    // r[verify identity.auth.reject-unknown]
    #[tokio::test]
    async fn unknown_challenge_rejected() {
        let (service, keys) = test_auth_service().await;
        let npub_hex = keys.public_key().to_hex();
        let sig = hex::encode([0u8; 64]);

        let result = service.verify_challenge(&npub_hex, "nonexistent", &sig).await;
        assert!(result.is_err());
    }

    // r[verify identity.auth.stable-key]
    #[tokio::test]
    async fn same_npub_gets_same_ed25519_key() {
        let (service, keys) = test_auth_service().await;
        let npub_hex = keys.public_key().to_hex();

        // First auth
        let (cid1, cb1) = service.create_challenge(&npub_hex).await.unwrap();
        let sig1 = sign_challenge(&keys, &cb1);
        let ctx1 = service.verify_challenge(&npub_hex, &cid1, &sig1).await.unwrap();

        // Second auth — same npub should get same ed25519 key
        let (cid2, cb2) = service.create_challenge(&npub_hex).await.unwrap();
        let sig2 = sign_challenge(&keys, &cb2);
        let ctx2 = service.verify_challenge(&npub_hex, &cid2, &sig2).await.unwrap();

        assert_eq!(ctx1.public_key, ctx2.public_key);
    }
}
