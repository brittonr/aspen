use aspen_auth_core::{Audience, Capability, CapabilityToken};
use aspen_cluster_types::{NodeAddress, NodeTransportAddr};
use aspen_hooks_ticket::AspenHookTicket;
use aspen_ticket::{AspenClusterTicket, BootstrapPeer, ClusterEndpointId, ClusterTopicId};
use core::net::SocketAddr;

pub fn build_portable_auth_token(issuer: iroh_base::PublicKey) -> CapabilityToken {
    CapabilityToken {
        version: 1,
        issuer,
        audience: Audience::Bearer,
        capabilities: vec![Capability::Read { prefix: "cfg/".into() }],
        issued_at: 1_700_000_000,
        expires_at: 1_700_003_600,
        nonce: Some([1u8; 16]),
        proof: None,
        delegation_depth: 0,
        facts: vec![],
        signature: [2u8; 64],
    }
}

pub fn build_cluster_ticket() -> AspenClusterTicket {
    let mut ticket = AspenClusterTicket::new(ClusterTopicId::from_bytes([9u8; 32]), "i7-cluster".into());
    ticket
        .add_bootstrap_peer(BootstrapPeer::new(ClusterEndpointId::new("i7-endpoint")))
        .expect("portable peer fits");
    ticket
}

pub fn build_hook_ticket() -> AspenHookTicket {
    let addr = NodeAddress::from_parts(
        "i7-hook-endpoint",
        [NodeTransportAddr::Ip(SocketAddr::from(([127, 0, 0, 1], 7007)))],
    );
    AspenHookTicket::new("i7-hooks", vec![addr]).with_event_type("write_committed")
}
