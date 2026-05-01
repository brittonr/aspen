//! Ticket parsing utilities.

#[cfg(feature = "iroh")]
use iroh_base::EndpointAddr;
#[cfg(feature = "iroh")]
use iroh_gossip::proto::TopicId;

#[cfg(feature = "iroh")]
use crate::AspenClusterTicket;
#[cfg(feature = "iroh")]
use crate::ClusterTicketResult;

/// Parse a ticket string and return runtime endpoint addresses.
#[cfg(feature = "iroh")]
pub fn parse_ticket_to_addrs(ticket_str: &str) -> ClusterTicketResult<(TopicId, String, Vec<EndpointAddr>)> {
    if !ticket_str.starts_with("aspen") {
        return Err(crate::ClusterTicketError::Deserialize {
            reason: "invalid ticket format: must start with 'aspen'".to_string(),
        });
    }
    let ticket = AspenClusterTicket::deserialize(ticket_str)?;
    let addrs = ticket.try_endpoint_addrs()?;
    Ok((ticket.topic_id.to_topic_id(), ticket.cluster_id, addrs))
}

#[cfg(all(test, not(feature = "iroh")))]
pub fn parse_ticket_to_addrs(
    ticket_str: &str,
) -> crate::ClusterTicketResult<(iroh_gossip::proto::TopicId, alloc::string::String, alloc::vec::Vec<iroh::EndpointAddr>)>
{
    if !ticket_str.starts_with("aspen") {
        return Err(crate::ClusterTicketError::Deserialize {
            reason: "invalid ticket format: must start with 'aspen'".to_string(),
        });
    }
    let ticket = crate::AspenClusterTicket::deserialize(ticket_str)?;
    let addrs = ticket.try_endpoint_addrs()?;
    Ok((ticket.topic_id.to_topic_id(), ticket.cluster_id, addrs))
}
