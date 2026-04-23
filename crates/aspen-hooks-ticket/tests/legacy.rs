use aspen_hooks_ticket::AspenHookTicket;
use aspen_hooks_ticket::HookTicketError;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

const HOOK_TICKET_PREFIX: &str = "aspenhook";
const TICKET_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LegacyAspenHookTicket {
    version: u8,
    cluster_id: String,
    bootstrap_peers: Vec<iroh::EndpointAddr>,
    event_type: String,
    default_payload: Option<String>,
    auth_token: Option<[u8; 32]>,
    expires_at_secs: u64,
    relay_url: Option<String>,
    priority: u8,
}

impl Ticket for LegacyAspenHookTicket {
    const KIND: &'static str = HOOK_TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("legacy hook ticket serialization should succeed in tests")
    }

    fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, iroh_tickets::ParseError> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }
}

#[test]
fn test_legacy_serialized_ticket_is_rejected() {
    let secret_key = iroh::SecretKey::from([7u8; 32]);
    let legacy_ticket = LegacyAspenHookTicket {
        version: TICKET_VERSION,
        cluster_id: "legacy-cluster".to_string(),
        bootstrap_peers: vec![iroh::EndpointAddr::from(secret_key.public())],
        event_type: "write_committed".to_string(),
        default_payload: None,
        auth_token: None,
        expires_at_secs: 0,
        relay_url: None,
        priority: 0,
    };

    let serialized = Ticket::serialize(&legacy_ticket);
    let result = AspenHookTicket::deserialize_at(&serialized, 10);
    assert!(matches!(result, Err(HookTicketError::Deserialize { .. })));
}
