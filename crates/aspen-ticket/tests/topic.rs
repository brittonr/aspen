#![feature(register_tool)]
#![register_tool(tigerstyle)]
#![allow(
    no_panic,
    reason = "error-shape test uses panic for unexpected enum variant diagnostics"
)]

use aspen_ticket::ClusterTicketError;
use aspen_ticket::ClusterTopicId;

const SHORT_TOPIC_LEN_BYTES: usize = 31;
const REQUIRED_TOPIC_LEN_BYTES_MESSAGE: &str = "expected 32 bytes";
const SHORT_TOPIC_FILL_BYTE: u8 = 7;
#[cfg(feature = "iroh")]
const RUNTIME_TOPIC_FILL_BYTE: u8 = 13;
#[cfg(feature = "iroh")]
const TOPIC_LEN_BYTES: usize = 32;

#[test]
fn cluster_topic_id_rejects_wrong_length() {
    let bytes = [SHORT_TOPIC_FILL_BYTE; SHORT_TOPIC_LEN_BYTES];
    let error = ClusterTopicId::try_from_slice(&bytes).expect_err("31-byte topic should fail");
    match error {
        ClusterTicketError::InvalidTopicId { reason } => {
            assert!(reason.contains(REQUIRED_TOPIC_LEN_BYTES_MESSAGE), "unexpected reason: {reason}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[cfg(feature = "iroh")]
#[test]
fn cluster_topic_id_roundtrips_to_iroh_topic() {
    let topic_id = ClusterTopicId::from_bytes([RUNTIME_TOPIC_FILL_BYTE; TOPIC_LEN_BYTES]);
    let runtime_topic = topic_id.to_topic_id();
    let roundtripped = ClusterTopicId::from(runtime_topic);
    assert_eq!(roundtripped, topic_id);
}
