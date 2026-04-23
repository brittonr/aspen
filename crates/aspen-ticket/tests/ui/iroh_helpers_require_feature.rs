use aspen_ticket::{parse_ticket_to_addrs, AspenClusterTicket};
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;

fn main() {
    let endpoint_addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
    let _ticket = AspenClusterTicket::with_bootstrap_addr(
        TopicId::from_bytes([1u8; 32]),
        "ui-cluster".to_string(),
        &endpoint_addr,
    );
    let _ = parse_ticket_to_addrs("aspeninvalid");
}
