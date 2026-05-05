//! Signed cluster tickets with explicit-time verification helpers.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
#[cfg(any(test, feature = "std"))]
use std::time::SystemTime;
#[cfg(any(test, feature = "std"))]
use std::time::UNIX_EPOCH;

use iroh_base::PublicKey;
#[cfg(any(test, feature = "std"))]
use iroh_base::SecretKey;
use iroh_base::Signature;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

use crate::AspenClusterTicket;
use crate::ClusterTicketError;
use crate::ClusterTicketResult;
use crate::constants::CLOCK_SKEW_TOLERANCE_SECS;
#[cfg(any(test, feature = "std"))]
use crate::constants::DEFAULT_TICKET_VALIDITY_SECS;
use crate::constants::SIGNED_TICKET_VERSION;

/// Signed cluster ticket with Ed25519 signature verification.
#[derive(Clone, Serialize, Deserialize)]
pub struct SignedAspenClusterTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The inner ticket payload.
    pub ticket: AspenClusterTicket,
    /// Public key of the ticket creator.
    pub issuer: PublicKey,
    /// Unix timestamp when ticket was created.
    pub issued_at_secs: u64,
    /// Unix timestamp when ticket expires.
    pub expires_at_secs: u64,
    /// Nonce for replay prevention.
    pub nonce: [u8; 16],
    /// Ed25519 signature over the serialized payload.
    pub signature: Signature,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SignedTimestampValidationInput {
    issued_at_secs: u64,
    expires_at_secs: u64,
    now_secs: u64,
}

#[cfg(any(test, feature = "std"))]
struct SignMaterialInput<'a> {
    ticket: AspenClusterTicket,
    secret_key: &'a SecretKey,
    issued_at_secs: u64,
    expires_at_secs: u64,
    nonce: [u8; 16],
}

impl fmt::Debug for SignedAspenClusterTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignedAspenClusterTicket")
            .field("version", &self.version)
            .field("ticket_cluster_id", &self.ticket.cluster_id)
            .field("bootstrap_peer_count", &self.ticket.bootstrap.len())
            .field("issuer", &self.issuer)
            .field("issued_at_secs", &self.issued_at_secs)
            .field("expires_at_secs", &self.expires_at_secs)
            .field("nonce", &"<redacted: 16 bytes>")
            .field("signature", &"<redacted>")
            .finish()
    }
}

impl SignedAspenClusterTicket {
    /// Verify the signature and timestamps against an explicit validation time.
    pub fn verify_at(&self, now_secs: u64) -> Option<&AspenClusterTicket> {
        self.verify_with_error_at(now_secs).ok()
    }

    /// Verify the signature and timestamps with detailed errors.
    pub fn verify_with_error_at(&self, now_secs: u64) -> ClusterTicketResult<&AspenClusterTicket> {
        validate_signed_version(self.version)?;
        let payload_bytes = self.payload_bytes();
        if self.issuer.verify(&payload_bytes, &self.signature).is_err() {
            return Err(ClusterTicketError::InvalidSignature);
        }
        validate_signed_timestamps(SignedTimestampValidationInput {
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            now_secs,
        })?;
        Ok(&self.ticket)
    }

    /// Returns whether the ticket is expired at the supplied validation time.
    pub fn is_expired_at(&self, now_secs: u64) -> bool {
        self.expires_at_secs < now_secs
    }

    /// Serialize the signed ticket to a base32-encoded string.
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a signed ticket from a base32-encoded string.
    pub fn deserialize(input: &str) -> ClusterTicketResult<Self> {
        <Self as Ticket>::deserialize(input).map_err(|error| ClusterTicketError::Deserialize {
            reason: error.to_string(),
        })
    }

    /// Returns the issuer's public key.
    pub fn issuer(&self) -> &PublicKey {
        &self.issuer
    }

    /// Returns the ticket expiration timestamp.
    pub fn expires_at(&self) -> u64 {
        self.expires_at_secs
    }

    fn payload_bytes(&self) -> Vec<u8> {
        SignedTicketPayload {
            version: self.version,
            ticket: &self.ticket,
            issuer: self.issuer,
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            nonce: self.nonce,
        }
        .to_bytes()
    }
}

#[cfg(any(test, feature = "std"))]
impl SignedAspenClusterTicket {
    /// Create and sign a new cluster ticket using the current wall clock.
    pub fn sign(ticket: AspenClusterTicket, secret_key: &SecretKey) -> ClusterTicketResult<Self> {
        Self::sign_with_validity(ticket, secret_key, DEFAULT_TICKET_VALIDITY_SECS)
    }

    /// Create and sign a ticket with a custom validity duration.
    pub fn sign_with_validity(
        ticket: AspenClusterTicket,
        secret_key: &SecretKey,
        validity_secs: u64,
    ) -> ClusterTicketResult<Self> {
        let now_secs = current_unix_time_secs()?;
        let expires_at_secs = now_secs.saturating_add(validity_secs);
        let nonce = generate_nonce();
        Ok(sign_with_material(SignMaterialInput {
            ticket,
            secret_key,
            issued_at_secs: now_secs,
            expires_at_secs,
            nonce,
        }))
    }

    /// Verify the signature and timestamps using the current wall clock.
    pub fn verify(&self) -> Option<&AspenClusterTicket> {
        self.verify_at(current_unix_time_secs_or_zero())
    }

    /// Verify with detailed errors using the current wall clock.
    pub fn verify_with_error(&self) -> ClusterTicketResult<&AspenClusterTicket> {
        self.verify_with_error_at(current_unix_time_secs()?)
    }

    /// Returns whether the ticket is expired using the current wall clock.
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(current_unix_time_secs_or_zero())
    }
}

impl Ticket for SignedAspenClusterTicket {
    const KIND: &'static str = "aspensigned";

    fn to_bytes(&self) -> Vec<u8> {
        serialize_postcard_fail_closed(self, "SignedAspenClusterTicket")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

#[derive(Serialize)]
struct SignedTicketPayload<'a> {
    version: u8,
    ticket: &'a AspenClusterTicket,
    issuer: PublicKey,
    issued_at_secs: u64,
    expires_at_secs: u64,
    nonce: [u8; 16],
}

impl SignedTicketPayload<'_> {
    fn to_bytes(&self) -> Vec<u8> {
        serialize_postcard_fail_closed(self, "SignedTicketPayload")
    }
}

fn serialize_postcard_fail_closed<T: Serialize>(value: &T, type_name: &str) -> Vec<u8> {
    match postcard::to_allocvec(value) {
        Ok(bytes) => bytes,
        Err(error) => {
            debug_assert!(false, "{type_name} serialization must succeed for bounded fields: {error}");
            Vec::new()
        }
    }
}

fn validate_signed_version(version: u8) -> ClusterTicketResult<()> {
    if version > SIGNED_TICKET_VERSION {
        return Err(ClusterTicketError::UnsupportedSignedVersion {
            version,
            max_supported: SIGNED_TICKET_VERSION,
        });
    }
    Ok(())
}

fn validate_signed_timestamps(input: SignedTimestampValidationInput) -> ClusterTicketResult<()> {
    let max_issued_at_secs = input.now_secs.saturating_add(CLOCK_SKEW_TOLERANCE_SECS);
    if input.issued_at_secs > max_issued_at_secs {
        return Err(ClusterTicketError::SignedTicketIssuedInFuture {
            issued_at_secs: input.issued_at_secs,
            now_secs: input.now_secs,
        });
    }
    if input.expires_at_secs < input.now_secs {
        return Err(ClusterTicketError::ExpiredSignedTicket {
            expires_at_secs: input.expires_at_secs,
            now_secs: input.now_secs,
        });
    }
    Ok(())
}

#[cfg(any(test, feature = "std"))]
fn sign_with_material(input: SignMaterialInput<'_>) -> SignedAspenClusterTicket {
    debug_assert!(input.expires_at_secs >= input.issued_at_secs);

    let issuer = input.secret_key.public();
    let payload = SignedTicketPayload {
        version: SIGNED_TICKET_VERSION,
        ticket: &input.ticket,
        issuer,
        issued_at_secs: input.issued_at_secs,
        expires_at_secs: input.expires_at_secs,
        nonce: input.nonce,
    };
    let payload_bytes = payload.to_bytes();
    let signature = input.secret_key.sign(&payload_bytes);
    SignedAspenClusterTicket {
        version: SIGNED_TICKET_VERSION,
        ticket: input.ticket,
        issuer,
        issued_at_secs: input.issued_at_secs,
        expires_at_secs: input.expires_at_secs,
        nonce: input.nonce,
        signature,
    }
}

#[cfg(any(test, feature = "std"))]
#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "signed ticket helpers are the std shell boundary for wall-clock time"
)]
fn current_unix_time_secs() -> ClusterTicketResult<u64> {
    let elapsed_since_epoch =
        SystemTime::now().duration_since(UNIX_EPOCH).map_err(|error| ClusterTicketError::Deserialize {
            reason: error.to_string(),
        })?;
    Ok(elapsed_since_epoch.as_secs())
}

#[cfg(any(test, feature = "std"))]
fn current_unix_time_secs_or_zero() -> u64 {
    match current_unix_time_secs() {
        Ok(now_secs) => now_secs,
        Err(_) => 0,
    }
}

#[cfg(any(test, feature = "std"))]
fn generate_nonce() -> [u8; 16] {
    use rand::RngCore;

    let mut nonce = [0u8; 16];
    rand::rng().fill_bytes(&mut nonce);
    nonce
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    #[test]
    fn signed_ticket_debug_redacts_replay_and_signature_material() {
        let ticket =
            AspenClusterTicket::new(crate::ClusterTopicId::from_bytes([7u8; 32]), "redaction-cluster".to_string());
        let secret_key = SecretKey::from([9u8; 32]);
        let signed = sign_with_material(SignMaterialInput {
            ticket,
            secret_key: &secret_key,
            issued_at_secs: 1_700_000_000,
            expires_at_secs: 1_700_000_900,
            nonce: [0xAB; 16],
        });
        let debug = format!("{signed:?}");

        assert!(debug.contains("SignedAspenClusterTicket"));
        assert!(debug.contains("bootstrap_peer_count"));
        assert!(debug.contains("<redacted: 16 bytes>"));
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("nonce: [171"));
        assert!(!debug.contains("signature: Signature"));
    }
}

#[cfg(test)]
mod tests {
    use iroh_gossip::proto::TopicId;
    use iroh_tickets::Ticket;

    use super::SignedAspenClusterTicket;
    use crate::AspenClusterTicket;

    fn make_signed_ticket() -> SignedAspenClusterTicket {
        let key = iroh::SecretKey::from([1u8; 32]);
        let inner = AspenClusterTicket::with_bootstrap(
            TopicId::from_bytes([42u8; 32]),
            "test-cluster".to_string(),
            key.public(),
        );
        SignedAspenClusterTicket::sign(inner, &key).unwrap()
    }

    #[test]
    fn to_bytes_produces_nonempty_payload() {
        let signed = make_signed_ticket();
        let bytes = <SignedAspenClusterTicket as Ticket>::to_bytes(&signed);
        assert!(!bytes.is_empty(), "to_bytes must not produce empty payload");
    }

    #[test]
    fn roundtrip_via_ticket_trait() {
        let signed = make_signed_ticket();
        let serialized = signed.serialize();
        let restored = SignedAspenClusterTicket::deserialize(&serialized).unwrap();
        assert_eq!(restored.version, signed.version);
        assert_eq!(restored.issuer, signed.issuer);
        assert_eq!(restored.ticket, signed.ticket);
    }

    #[test]
    fn verify_roundtripped_ticket() {
        let signed = make_signed_ticket();
        let serialized = signed.serialize();
        let restored = SignedAspenClusterTicket::deserialize(&serialized).unwrap();
        assert!(restored.verify().is_some(), "roundtripped ticket must verify");
    }
}
