//! Signed cluster tickets with Ed25519 signature verification.

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

use crate::AspenClusterTicket;
use crate::constants::CLOCK_SKEW_TOLERANCE_SECS;
use crate::constants::DEFAULT_TICKET_VALIDITY_SECS;
use crate::constants::SIGNED_TICKET_VERSION;

/// Signed cluster ticket with Ed25519 signature verification.
///
/// Provides cryptographic authentication for cluster join tickets:
/// - Proves ticket was created by a known cluster member
/// - Prevents ticket forgery by malicious actors
/// - Includes timestamp-based expiration for replay prevention
/// - Uses 128-bit nonce for additional replay protection
///
/// Tiger Style: Fixed-size signature (64 bytes), bounded fields, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAspenClusterTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The inner ticket payload (topic, bootstrap, cluster_id).
    pub ticket: AspenClusterTicket,
    /// Public key of the ticket creator (cluster member who signed).
    pub issuer: PublicKey,
    /// Unix timestamp when ticket was created (seconds since epoch).
    pub issued_at_secs: u64,
    /// Unix timestamp when ticket expires (seconds since epoch).
    pub expires_at_secs: u64,
    /// Random nonce for replay prevention (128 bits).
    pub nonce: [u8; 16],
    /// Ed25519 signature over the serialized payload (64 bytes).
    pub signature: Signature,
}

impl SignedAspenClusterTicket {
    /// Create and sign a new cluster ticket.
    ///
    /// Signs the ticket payload with the provided secret key. The issuer's
    /// public key is derived from the secret key and embedded in the ticket.
    ///
    /// # Arguments
    /// * `ticket` - The unsigned cluster ticket payload
    /// * `secret_key` - The signer's Iroh secret key
    ///
    /// # Returns
    /// A signed ticket with 24-hour validity (default TTL).
    pub fn sign(ticket: AspenClusterTicket, secret_key: &SecretKey) -> Result<Self> {
        Self::sign_with_validity(ticket, secret_key, DEFAULT_TICKET_VALIDITY_SECS)
    }

    /// Create and sign a ticket with custom validity duration.
    ///
    /// # Arguments
    /// * `ticket` - The unsigned cluster ticket payload
    /// * `secret_key` - The signer's Iroh secret key
    /// * `validity_secs` - How long the ticket is valid (in seconds)
    pub fn sign_with_validity(ticket: AspenClusterTicket, secret_key: &SecretKey, validity_secs: u64) -> Result<Self> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before Unix epoch")?.as_secs();

        // Generate random nonce for replay prevention
        let mut nonce = [0u8; 16];
        {
            use rand::RngCore;
            rand::rng().fill_bytes(&mut nonce);
        }

        // Build the unsigned payload for signing
        let payload = SignedTicketPayload {
            version: SIGNED_TICKET_VERSION,
            ticket: &ticket,
            issuer: secret_key.public(),
            issued_at_secs: now,
            expires_at_secs: now.saturating_add(validity_secs),
            nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes()?;

        // Sign with Ed25519
        let signature = secret_key.sign(&payload_bytes);

        Ok(Self {
            version: SIGNED_TICKET_VERSION,
            ticket,
            issuer: secret_key.public(),
            issued_at_secs: now,
            expires_at_secs: now.saturating_add(validity_secs),
            nonce,
            signature,
        })
    }

    /// Verify the signature and return the inner ticket if valid.
    ///
    /// Performs the following checks:
    /// 1. Signature verification using issuer's public key
    /// 2. Timestamp validation (not expired, not in future)
    /// 3. Version compatibility check
    ///
    /// Returns `None` if any validation fails (fail-fast, no error details
    /// to prevent information leakage to attackers).
    pub fn verify(&self) -> Option<&AspenClusterTicket> {
        // Reject unknown future versions
        if self.version > SIGNED_TICKET_VERSION {
            return None;
        }

        // Build the payload for signature verification
        let payload = SignedTicketPayload {
            version: self.version,
            ticket: &self.ticket,
            issuer: self.issuer,
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            nonce: self.nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes().ok()?;

        // Verify Ed25519 signature
        if self.issuer.verify(&payload_bytes, &self.signature).is_err() {
            return None;
        }

        // Verify timestamps (with clock skew tolerance)
        if !self.is_timestamp_valid() {
            return None;
        }

        Some(&self.ticket)
    }

    /// Verify signature and timestamps, returning detailed error on failure.
    ///
    /// Unlike `verify()`, this method provides error context for debugging.
    /// Use this for logging/diagnostics, NOT for security decisions.
    pub fn verify_with_error(&self) -> Result<&AspenClusterTicket> {
        // Reject unknown future versions
        if self.version > SIGNED_TICKET_VERSION {
            anyhow::bail!("unsupported ticket version {} (max supported: {})", self.version, SIGNED_TICKET_VERSION);
        }

        // Build the payload for signature verification
        let payload = SignedTicketPayload {
            version: self.version,
            ticket: &self.ticket,
            issuer: self.issuer,
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            nonce: self.nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes().context("failed to serialize ticket payload for verification")?;

        // Verify Ed25519 signature
        self.issuer.verify(&payload_bytes, &self.signature).context("signature verification failed")?;

        // Verify timestamps
        self.verify_timestamps()?;

        Ok(&self.ticket)
    }

    /// Check if the timestamp is valid (not expired, not in future).
    fn is_timestamp_valid(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

        // Check not issued in the future (with clock skew tolerance)
        if self.issued_at_secs > now.saturating_add(CLOCK_SKEW_TOLERANCE_SECS) {
            return false;
        }

        // Check not expired
        if self.expires_at_secs < now {
            return false;
        }

        true
    }

    /// Verify timestamps with detailed error messages.
    fn verify_timestamps(&self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before Unix epoch")?.as_secs();

        // Check not issued in the future (with clock skew tolerance)
        if self.issued_at_secs > now.saturating_add(CLOCK_SKEW_TOLERANCE_SECS) {
            anyhow::bail!(
                "ticket issued_at {} is in the future (now: {}, tolerance: {}s)",
                self.issued_at_secs,
                now,
                CLOCK_SKEW_TOLERANCE_SECS
            );
        }

        // Check not expired
        if self.expires_at_secs < now {
            anyhow::bail!("ticket expired at {} (now: {})", self.expires_at_secs, now);
        }

        Ok(())
    }

    /// Serialize the signed ticket to a base32-encoded string.
    ///
    /// Format: `aspensigned{base32-encoded-postcard-payload}`
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a signed ticket from a base32-encoded string.
    ///
    /// Note: This only parses the ticket, it does NOT verify the signature.
    /// Call `verify()` after deserialization to validate the ticket.
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize signed Aspen ticket")
    }

    /// Returns the issuer's public key.
    pub fn issuer(&self) -> &PublicKey {
        &self.issuer
    }

    /// Returns the ticket expiration timestamp.
    pub fn expires_at(&self) -> u64 {
        self.expires_at_secs
    }

    /// Returns whether the ticket has expired.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.expires_at_secs < now
    }
}

impl Ticket for SignedAspenClusterTicket {
    const KIND: &'static str = "aspensigned";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect(
            "SignedAspenClusterTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

/// Internal payload structure for signing (excludes the signature field).
///
/// This struct is used to serialize the data that gets signed, ensuring
/// the signature is computed over a canonical byte representation.
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
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize ticket payload")
    }
}
