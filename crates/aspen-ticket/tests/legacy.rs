#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    no_unwrap,
    reason = "iroh_tickets::Ticket requires infallible serialization in legacy rejection fixture"
)]

use std::net::SocketAddr;

use aspen_ticket::AspenClusterTicket;
use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LegacyBootstrapPeer {
    endpoint_id: EndpointId,
    direct_addrs: Vec<SocketAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LegacyAspenClusterTicket {
    topic_id: TopicId,
    bootstrap: Vec<LegacyBootstrapPeer>,
    cluster_id: String,
}

impl Ticket for LegacyAspenClusterTicket {
    const KIND: &'static str = "aspen";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("legacy AspenClusterTicket serialization should succeed in tests")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

fn legacy_ticket_string() -> String {
    let endpoint_id = iroh::SecretKey::from([7u8; 32]).public();
    let ticket = LegacyAspenClusterTicket {
        topic_id: TopicId::from_bytes([9u8; 32]),
        bootstrap: vec![LegacyBootstrapPeer {
            endpoint_id,
            direct_addrs: vec![SocketAddr::from(([127, 0, 0, 1], 7777))],
        }],
        cluster_id: "legacy-cluster".to_string(),
    };
    Ticket::serialize(&ticket)
}

#[test]
fn legacy_unsigned_ticket_is_rejected() {
    let legacy_ticket = legacy_ticket_string();
    let error = AspenClusterTicket::deserialize(&legacy_ticket).expect_err("legacy payload should not deserialize");
    let message = error.to_string();
    assert!(message.contains("deserialize"), "unexpected error: {message}");
}
