pub(super) fn observed_ack(
    received: &crate::fabric_consistency::raft::iroh::ReceivedReplicaEvent,
    state: &crate::fabric_consistency::raft::model::ReplicaState,
    pending: &crate::fabric_consistency::raft::model::PendingReplicaRead,
) -> preserves::IOValue {
    let crate::fabric_consistency::raft::model::ReplicaEvent::Message {
        envelope:
            crate::fabric_consistency::raft::model::ReplicaMessageEnvelope {
                from,
                to,
                group_binding_ref,
                message:
                    crate::fabric_consistency::raft::model::Message::ReadAcknowledgement {
                        term,
                        follower_id,
                        request_ref,
                        ..
                    },
                ..
            },
    } = &received.event
    else {
        panic!("expected a delivered Raft read acknowledgement");
    };
    assert_eq!(from, crate::fabric_consistency::raft::tests::NODE_A);
    assert_eq!(from, follower_id);
    assert_ne!(from, to);
    assert_eq!(*to, state.node_id);
    assert_eq!(*group_binding_ref, state.profile.group_binding_ref);
    assert_eq!(*term, pending.term);
    assert_eq!(*request_ref, pending.request_ref);
    assert!(!pending.acknowledgements.contains(from));
    crate::codec::record("read-ack", vec![
        crate::codec::string(from),
        crate::codec::string(to),
        crate::codec::u64_value(*term),
        crate::codec::string(request_ref),
        crate::codec::string(&received.transport_evidence.request_ref),
    ])
}

pub(super) fn observed_outcome(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
    applied: &crate::fabric_consistency::raft::executor::ExecutedReplicaTransition,
    kind: crate::fabric_consistency::raft::executor::ReplicaEffectKind,
) -> crate::fabric_consistency::raft::control::ReplicaObservation {
    assert!(applied.transport_failures.is_empty(), "trace turn had transport failures");
    let observation = node._control_receiver.try_recv().unwrap_or_else(|error| {
        panic!("missing {} channel observation on {}: {error}", kind.as_str(), applied.next.node_id)
    });
    assert!(
        applied
            .observations
            .iter()
            .any(|effect| effect.kind == kind && effect.evidence_ref == observation.receipt_ref),
        "channel receipt does not match the applied {} effect on {}",
        kind.as_str(),
        applied.next.node_id,
    );
    observation
}

pub(super) fn observed_proposal(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
    applied: &crate::fabric_consistency::raft::executor::ExecutedReplicaTransition,
    expected_request: &str,
) -> (u64, preserves::IOValue) {
    let observation =
        observed_outcome(node, applied, crate::fabric_consistency::raft::executor::ReplicaEffectKind::ProposalOutcome);
    let crate::fabric_consistency::raft::control::ReplicaObservationKind::Proposal {
        request_ref,
        disposition,
        committed_index: Some(index),
    } = observation.kind
    else {
        panic!("expected a proposal channel observation with an index for {expected_request}");
    };
    assert_eq!(request_ref, expected_request);
    assert_eq!(disposition, crate::fabric_consistency::raft::model::ProposalDisposition::Committed);
    assert_eq!(applied.next.completed_requests.get(&request_ref), Some(&index));
    let value = crate::codec::record("proposal-outcome", vec![
        crate::codec::string(request_ref),
        crate::codec::string(disposition.as_str()),
        crate::codec::u64_value(index),
        crate::codec::string(observation.receipt_ref),
    ]);
    (index, value)
}

pub(super) fn observed_read(
    node: &mut crate::fabric_consistency::raft::live_cluster::Node,
    applied: &crate::fabric_consistency::raft::executor::ExecutedReplicaTransition,
    expected_request: &str,
    expected_disposition: crate::fabric_consistency::raft::model::ReadDisposition,
) -> (u64, preserves::IOValue) {
    let observation =
        observed_outcome(node, applied, crate::fabric_consistency::raft::executor::ReplicaEffectKind::ReadOutcome);
    let crate::fabric_consistency::raft::control::ReplicaObservationKind::Read {
        request_ref,
        mode,
        disposition,
        observed_index,
    } = observation.kind
    else {
        panic!("expected a read channel observation for {expected_request}");
    };
    assert_eq!(request_ref, expected_request);
    assert_eq!(mode, crate::fabric_consistency::ReadMode::Linearizable);
    assert_eq!(disposition, expected_disposition);
    let value = crate::codec::record("read-outcome", vec![
        crate::codec::string(request_ref),
        crate::codec::string(mode.as_str()),
        crate::codec::string(disposition.as_str()),
        crate::codec::u64_value(observed_index),
        crate::codec::string(observation.receipt_ref),
    ]);
    (observed_index, value)
}
