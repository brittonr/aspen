//! Unified ticket parsing for V1 and V2 ticket formats.

use std::collections::BTreeSet;

use anyhow::Result;
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;

use crate::AspenClusterTicket;
use crate::AspenClusterTicketV2;

/// Parse either a V1 or V2 ticket string and return endpoint addresses.
///
/// This function provides a unified interface for ticket parsing:
/// - V1 tickets (aspen...) return addresses with only endpoint IDs
/// - V2 tickets (aspenv2...) return addresses with direct socket addresses
///
/// Use this when you want to connect regardless of ticket version.
pub fn parse_ticket_to_addrs(ticket_str: &str) -> Result<(TopicId, String, Vec<EndpointAddr>)> {
    if ticket_str.starts_with("aspenv2") {
        let ticket = AspenClusterTicketV2::deserialize(ticket_str)?;
        let addrs = ticket.endpoint_addrs();
        Ok((ticket.topic_id, ticket.cluster_id, addrs))
    } else if ticket_str.starts_with("aspen") {
        let ticket = AspenClusterTicket::deserialize(ticket_str)?;
        let addrs = ticket
            .bootstrap
            .iter()
            .map(|id| EndpointAddr {
                id: *id,
                addrs: BTreeSet::new(),
            })
            .collect();
        Ok((ticket.topic_id, ticket.cluster_id, addrs))
    } else {
        anyhow::bail!("invalid ticket format: must start with 'aspen' or 'aspenv2'")
    }
}
