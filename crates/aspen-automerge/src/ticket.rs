//! Automerge sync ticket: endpoint address + capability token in one string.
//!
//! An `AutomergeSyncTicket` bundles everything a client needs to connect to a
//! node and sync automerge documents: the iroh endpoint address for QUIC
//! connectivity, and the capability token for authorization.
//!
//! # Format
//!
//! ```text
//! amsync1{base64url(postcard(AutomergeSyncTicket))}
//! ```

use aspen_auth::CapabilityToken;
use iroh::EndpointAddr;
use serde::Deserialize;
use serde::Serialize;

/// Prefix for automerge sync ticket strings.
pub const TICKET_PREFIX: &str = "amsync1";

/// A self-contained ticket for connecting to an automerge sync endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomergeSyncTicket {
    /// Endpoint address (node ID + relay URL + direct addresses).
    pub addr: EndpointAddr,
    /// Capability token granting sync access.
    pub token: Vec<u8>,
}

impl AutomergeSyncTicket {
    /// Create a new sync ticket.
    ///
    /// Returns an error if the capability token cannot be encoded (e.g.,
    /// exceeds `MAX_TOKEN_SIZE`).
    pub fn new(addr: EndpointAddr, token: &CapabilityToken) -> Result<Self, TicketError> {
        let encoded = token.encode().map_err(|e| TicketError::Encode(e.to_string()))?;
        Ok(Self { addr, token: encoded })
    }

    /// Serialize to a shareable string.
    ///
    /// Returns an error if postcard serialization fails (should not happen
    /// for a validly-constructed ticket).
    pub fn serialize(&self) -> Result<String, TicketError> {
        use base64::Engine;
        let bytes = postcard::to_stdvec(self).map_err(|e| TicketError::Encode(e.to_string()))?;
        Ok(format!("{}{}", TICKET_PREFIX, base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)))
    }

    /// Deserialize from a string.
    pub fn deserialize(s: &str) -> Result<Self, TicketError> {
        use base64::Engine;
        let data = s.strip_prefix(TICKET_PREFIX).ok_or(TicketError::WrongPrefix)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(data)
            .map_err(|e| TicketError::Decode(e.to_string()))?;
        postcard::from_bytes(&bytes).map_err(|e| TicketError::Decode(e.to_string()))
    }

    /// Extract the capability token.
    pub fn capability_token(&self) -> Result<CapabilityToken, TicketError> {
        CapabilityToken::decode(&self.token).map_err(|e| TicketError::Decode(e.to_string()))
    }
}

/// Errors when parsing a sync ticket.
#[derive(Debug, thiserror::Error)]
pub enum TicketError {
    #[error("ticket must start with '{TICKET_PREFIX}'")]
    WrongPrefix,

    #[error("decode error: {0}")]
    Decode(String),

    #[error("encode error: {0}")]
    Encode(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_addr() -> EndpointAddr {
        let key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        EndpointAddr::new(key.public())
    }

    fn make_test_token() -> CapabilityToken {
        use std::time::Duration;
        let key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        aspen_auth::TokenBuilder::new(key)
            .with_capability(aspen_auth::Capability::Full {
                prefix: "automerge:".into(),
            })
            .with_lifetime(Duration::from_secs(3600))
            .build()
            .unwrap()
    }

    #[test]
    fn roundtrip() {
        let addr = make_test_addr();
        let token = make_test_token();
        let ticket = AutomergeSyncTicket::new(addr.clone(), &token).unwrap();

        let s = ticket.serialize().unwrap();
        assert!(s.starts_with(TICKET_PREFIX));

        let restored = AutomergeSyncTicket::deserialize(&s).unwrap();
        assert_eq!(restored.addr.id, addr.id);

        let restored_token = restored.capability_token().unwrap();
        assert_eq!(restored_token.issuer, token.issuer);
    }

    #[test]
    fn wrong_prefix_rejected() {
        assert!(AutomergeSyncTicket::deserialize("garbage").is_err());
    }

    #[test]
    fn corrupt_data_rejected() {
        assert!(AutomergeSyncTicket::deserialize("amsync1!!!notbase64").is_err());
    }

    #[test]
    fn oversized_token_rejected_at_construction() {
        use std::time::Duration;
        let key = iroh::SecretKey::generate(&mut rand::rngs::ThreadRng::default());
        // Build a token with 32 capabilities, each with a very long prefix,
        // to push the encoded size over MAX_TOKEN_SIZE (8KB).
        let mut builder = aspen_auth::TokenBuilder::new(key).with_lifetime(Duration::from_secs(3600));
        for i in 0..32 {
            builder = builder.with_capability(aspen_auth::Capability::Full {
                prefix: format!("automerge:{i}:{}", "x".repeat(300)),
            });
        }
        let big_token = builder.build().unwrap();

        let addr = make_test_addr();
        let result = AutomergeSyncTicket::new(addr, &big_token);
        assert!(result.is_err(), "oversized token should be rejected");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("encode error"), "error should mention encoding: {err_msg}");
    }

    #[test]
    fn serialize_returns_nonempty_payload() {
        let addr = make_test_addr();
        let token = make_test_token();
        let ticket = AutomergeSyncTicket::new(addr, &token).unwrap();
        let s = ticket.serialize().unwrap();
        // Strip prefix and verify the base64 payload is not empty.
        let payload = s.strip_prefix(TICKET_PREFIX).unwrap();
        assert!(!payload.is_empty(), "serialized ticket payload must not be empty");
    }
}
