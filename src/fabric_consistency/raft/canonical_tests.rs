use super::tests::NODE_A;
use super::tests::active_group;
use super::tests::elect_node_a;
use super::tests::sent_envelope_to;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;

const TRAILING_SENTINEL_BYTE: u8 = 0xff;
const NODE_B: &str = "node-b";

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn canonical_raft_vote_and_append_envelopes_roundtrip_exactly() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        entropy_ref: test_ref("canonical-election-entropy"),
    })
    .expect("election transition");
    let vote = sent_envelope_to(&election, NODE_B);
    assert_roundtrip(&vote);

    let (leader, _follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("canonical-proposal-request"),
        command_ref: test_ref("canonical-proposal-command"),
        command_schema_ref: test_ref("canonical-proposal-schema"),
    })
    .expect("proposal transition");
    let append = sent_envelope_to(&proposal, NODE_B);
    assert_roundtrip(&append);
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn canonical_raft_decoder_rejects_trailing_bytes_and_sender_substitution() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        entropy_ref: test_ref("canonical-negative-entropy"),
    })
    .expect("negative election transition");
    let envelope = sent_envelope_to(&election, NODE_B);
    let mut bytes = canonical_replica_message(&envelope).expect("canonical envelope").bytes;
    bytes.push(TRAILING_SENTINEL_BYTE);
    assert!(parse_canonical_replica_message(&bytes).is_err());

    let mut substituted = envelope;
    substituted.from = "node-c".to_string();
    let error = canonical_replica_message(&substituted).expect_err("substituted sender must deny");
    assert!(error.to_string().contains("embedded sender"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn canonical_raft_encoder_rejects_over_bound_append_batches() {
    let group = active_group();
    let state = started_state(&group, NODE_A);
    let entry_count = MAX_REPLICA_MESSAGE_ENTRIES + 1;
    let entries = (0..entry_count)
        .map(|offset| {
            let index = u64::try_from(offset + 1).expect("bounded test index");
            ReplicatedEntry {
                index,
                term: INITIAL_LOG_INDEX,
                request_ref: test_ref(&format!("over-bound-request-{offset}")),
                command_ref: test_ref(&format!("over-bound-command-{offset}")),
                command_schema_ref: test_ref("over-bound-schema"),
            }
        })
        .collect();
    let envelope = ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref,
        service_generation: state.profile.service_generation,
        from: NODE_A.to_string(),
        to: NODE_B.to_string(),
        message: RaftMessage::AppendEntries {
            term: INITIAL_LOG_INDEX,
            leader_id: NODE_A.to_string(),
            prev_log_index: INITIAL_COMMIT_INDEX,
            prev_log_term: INITIAL_COMMIT_INDEX,
            entries,
            leader_commit: INITIAL_COMMIT_INDEX,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    };
    let error = canonical_replica_message(&envelope).expect_err("over-bound append must deny");
    assert!(error.to_string().contains("message bound"));
}

fn assert_roundtrip(envelope: &ReplicaMessageEnvelope) {
    let first = canonical_replica_message(envelope).expect("first canonical envelope");
    let second = canonical_replica_message(envelope).expect("second canonical envelope");
    let parsed = parse_canonical_replica_message(&first.bytes).expect("strict canonical parse");
    assert_eq!(first, second);
    assert_eq!(&parsed, envelope);
    assert_eq!(first.envelope_ref, crate::preserves_rail::content_ref_from_bytes(&first.bytes));
}
