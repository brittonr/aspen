#![cfg(feature = "std")]

use core::net::SocketAddr;

use aspen_cluster_types::NodeAddress;
use aspen_cluster_types::NodeTransportAddr;
use aspen_hooks_ticket::AspenHookTicket;

fn valid_test_node_address(seed: u8) -> NodeAddress {
    let endpoint_id = format!("{seed:02x}").repeat(32);
    NodeAddress::from_parts(endpoint_id, [NodeTransportAddr::Ip(SocketAddr::from((
        [127, 0, 0, 1],
        8000u16.saturating_add(u16::from(seed)),
    )))])
}

#[test]
fn test_std_wrappers_work() -> Result<(), aspen_hooks_ticket::HookTicketError> {
    let ticket = AspenHookTicket::new("cluster", vec![valid_test_node_address(1)])
        .with_event_type("write_committed")
        .with_expiry_hours(1);

    assert!(!ticket.is_expired());
    assert_ne!(ticket.expiry_string(), "never");

    let serialized = ticket.serialize();
    let parsed = AspenHookTicket::deserialize(&serialized)?;
    assert_eq!(parsed.cluster_id, "cluster");
    assert_eq!(parsed.event_type, "write_committed");
    Ok(())
}
