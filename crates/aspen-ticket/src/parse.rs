//! Ticket parsing utilities.

use anyhow::Result;
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;

use crate::AspenClusterTicket;

/// Parse a ticket string and return endpoint addresses.
///
/// Returns the topic ID, cluster ID, and list of endpoint addresses
/// with their direct socket addresses for connection.
///
/// # Example
///
/// ```
/// # use aspen_ticket::{AspenClusterTicket, parse_ticket_to_addrs};
/// # use iroh_gossip::proto::TopicId;
/// let ticket = AspenClusterTicket::new(TopicId::from_bytes([1u8; 32]), "test".into());
/// let ticket_str = ticket.serialize();
/// let (topic, cluster, addrs) = parse_ticket_to_addrs(&ticket_str)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_ticket_to_addrs(ticket_str: &str) -> Result<(TopicId, String, Vec<EndpointAddr>)> {
    if !ticket_str.starts_with("aspen") {
        anyhow::bail!("invalid ticket format: must start with 'aspen'")
    }

    let ticket = AspenClusterTicket::deserialize(ticket_str)?;
    let addrs = ticket.endpoint_addrs();
    Ok((ticket.topic_id, ticket.cluster_id, addrs))
}
