use aspen_ticket::ClusterTicketError;
use aspen_ticket::ClusterTopicId;

#[test]
fn cluster_topic_id_rejects_wrong_length() {
    let bytes = [7u8; 31];
    let error = ClusterTopicId::try_from_slice(&bytes).expect_err("31-byte topic should fail");
    match error {
        ClusterTicketError::InvalidTopicId { reason } => {
            assert!(reason.contains("expected 32 bytes"), "unexpected reason: {reason}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[cfg(feature = "iroh")]
#[test]
fn cluster_topic_id_roundtrips_to_iroh_topic() {
    let topic_id = ClusterTopicId::from_bytes([13u8; 32]);
    let runtime_topic = topic_id.to_topic_id();
    let roundtripped = ClusterTopicId::from(runtime_topic);
    assert_eq!(roundtripped, topic_id);
}
