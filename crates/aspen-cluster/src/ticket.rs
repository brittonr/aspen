//! Re-exports of Aspen cluster ticket types from `aspen-ticket`.
//!
//! This module is maintained for backwards compatibility.
//! New code should import directly from `aspen_ticket`.

pub use aspen_ticket::AspenClusterTicket;
pub use aspen_ticket::AspenClusterTicketV2;
pub use aspen_ticket::BootstrapPeer;
pub use aspen_ticket::SignedAspenClusterTicket;
pub use aspen_ticket::parse_ticket_to_addrs;
