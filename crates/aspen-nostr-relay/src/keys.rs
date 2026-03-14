//! Nostr key management — secp256k1 keypair for event signing.
//!
//! Uses the `nostr` crate's built-in key types which wrap `secp256k1`.

use nostr::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Cluster-level Nostr identity wrapping a secp256k1 keypair.
///
/// Independent of the Ed25519 cluster identity — no derivation relationship.
#[derive(Clone)]
pub struct NostrIdentity {
    keys: Keys,
}

/// Serializable form of the Nostr secret key for persistence.
#[derive(Serialize, Deserialize)]
pub struct NostrIdentityPersist {
    /// Hex-encoded secret key bytes.
    pub secret_key_hex: String,
}

impl NostrIdentity {
    /// Generate a new random secp256k1 keypair.
    pub fn generate() -> Self {
        Self { keys: Keys::generate() }
    }

    /// Restore from raw secret key bytes (32 bytes).
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Result<Self, nostr::key::Error> {
        let secret = SecretKey::from_slice(bytes)?;
        Ok(Self {
            keys: Keys::new(secret),
        })
    }

    /// Export secret key as 32 raw bytes.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.keys.secret_key().secret_bytes()
    }

    /// Hex-encoded x-only public key (64 chars) for NIP-11 and event `pubkey` fields.
    pub fn public_key_hex(&self) -> String {
        self.keys.public_key().to_hex()
    }

    /// Access the underlying `Keys` for signing operations.
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    /// Sign a Nostr event builder, producing a fully signed event.
    ///
    /// Computes the event ID (SHA-256 of serialized `[0, pubkey, created_at, kind, tags,
    /// content]`), signs with Schnorr per BIP-340, and sets `id`/`pubkey`/`sig` fields.
    pub fn sign_event(&self, builder: EventBuilder) -> Result<Event, nostr::event::builder::Error> {
        builder.sign_with_keys(&self.keys)
    }

    /// Serialize for persistent storage.
    pub fn to_persist(&self) -> NostrIdentityPersist {
        NostrIdentityPersist {
            secret_key_hex: self.keys.secret_key().to_secret_hex(),
        }
    }

    /// Restore from persisted form.
    pub fn from_persist(persist: &NostrIdentityPersist) -> Result<Self, nostr::key::Error> {
        let secret = SecretKey::from_hex(&persist.secret_key_hex)?;
        Ok(Self {
            keys: Keys::new(secret),
        })
    }
}
