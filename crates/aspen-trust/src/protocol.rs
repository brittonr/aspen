//! Trust protocol message types.
//!
//! These messages travel over the trust-specific Iroh ALPN and drive
//! epoch rotation share collection.

use serde::Deserialize;
use serde::Serialize;

use crate::shamir::Share;

/// Initial trust protocol version.
pub const TRUST_PROTOCOL_VERSION: u16 = 1;

/// Request a node's share for a specific epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetShareRequest {
    /// Epoch whose share is being requested.
    pub epoch: u64,
}

/// Response carrying a node's share for a specific epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareResponse {
    /// Epoch the returned share belongs to.
    pub epoch: u64,
    /// The requested share.
    pub share: Share,
}

/// Trust protocol requests sent over QUIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustRequest {
    /// Request a share for an epoch.
    GetShare(GetShareRequest),
}

/// Trust protocol responses sent over QUIC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustResponse {
    /// Share reply for a successful GetShare request.
    Share(ShareResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_roundtrip_message_serde() {
        let share = Share {
            x: 7,
            y: [3; crate::shamir::SECRET_SIZE],
        };
        let message = TrustResponse::Share(ShareResponse { epoch: 11, share });
        let bytes = postcard::to_allocvec(&message).unwrap();
        let decoded: TrustResponse = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn test_get_share_request_serde() {
        let request = TrustRequest::GetShare(GetShareRequest { epoch: 42 });
        let bytes = postcard::to_allocvec(&request).unwrap();
        let decoded: TrustRequest = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, request);
    }
}
