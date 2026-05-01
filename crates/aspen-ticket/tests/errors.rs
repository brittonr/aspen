#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    no_panic,
    reason = "error-shape test uses panic for unexpected enum variant diagnostics"
)]

use aspen_ticket::AspenClusterTicket;
use aspen_ticket::ClusterTicketError;

const INVALID_UNSIGNED_TICKET: &str = "aspen!!!invalid!!!";

#[test]
fn malformed_unsigned_ticket_returns_deserialize_error() {
    let error = AspenClusterTicket::deserialize(INVALID_UNSIGNED_TICKET)
        .expect_err("malformed unsigned ticket should fail to deserialize");
    match error {
        ClusterTicketError::Deserialize { reason } => {
            assert!(!reason.is_empty(), "deserialize error reason should not be empty");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}
