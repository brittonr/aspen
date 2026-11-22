use std::collections::BTreeMap;

use preserves::IOValue;
use tokio::sync::mpsc::error::TryRecvError;

use super::super::tests::NODE_A;
use super::super::tests::NODE_B;
use super::super::tests::NODE_C;
use super::super::tests::test_ref;
use super::*;
use crate::fabric_consistency::ReadMode;
use crate::fabric_transport::cross_process::tests::listener_with_secret;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const CAPTURE_TAG: &str = "live-raft-read-index-trace-v1";
const CAPTURE_STEPS: usize = 6;
const MAX_PACKED_CAPTURE_BYTES: usize = 4_096;

// r[impl molten.testing.live_raft_trace.capture]
// r[verify molten.testing.live_raft_trace.capture]
#[tokio::test]
#[ignore = "opt-in live Raft read-index diagnostic; select explicitly with --ignored --nocapture"]
async fn capture_live_raft_read_index_trace() {
    let group = super::super::tests::active_group();
    let listener_a = listener_with_secret(NODE_A_SECRET_BYTE).await;
    let listener_b = listener_with_secret(NODE_B_SECRET_BYTE).await;
    let listener_c = listener_with_secret(NODE_C_SECRET_BYTE).await;
    let endpoints = BTreeMap::from([
        (NODE_A.to_string(), listener_a.handoff().clone()),
        (NODE_B.to_string(), listener_b.handoff().clone()),
        (NODE_C.to_string(), listener_c.handoff().clone()),
    ]);
    let [id_a, id_b, id_c] = [NODE_A, NODE_B, NODE_C].map(|id| &endpoints[id].descriptor.public_endpoint_identity);
    assert!(id_a != id_b && id_a != id_c && id_b != id_c, "trace needs distinct loopback listeners");

    let mut a = setup::build_node(&group, NODE_A, listener_a, &endpoints).await.expect("trace node A");
    let mut b = setup::build_node(&group, NODE_B, listener_b, &endpoints).await.expect("trace node B");
    let mut c = setup::build_node(&group, NODE_C, listener_c, &endpoints).await.expect("trace node C");
    let steps = capture(&mut a, &mut b, &mut c).await;
    let state = b.service.state();
    assert_eq!(state.profile.group_binding_ref, group.binding_ref);
    assert!(state.membership.voters.iter().map(String::as_str).eq([NODE_A, NODE_B, NODE_C]));
    let value = record(CAPTURE_TAG, vec![
        string(&state.profile.group_binding_ref),
        string(&state.membership.membership_ref),
        sequence(state.membership.voters.iter().map(string).collect()),
        sequence(Vec::from(steps)),
    ]);
    setup::close_node(a).await;
    setup::close_node(b).await;
    setup::close_node(c).await;

    let bytes = canonical_bytes(&value).expect("pack live Raft trace");
    assert!(bytes.len() <= MAX_PACKED_CAPTURE_BYTES, "packed live Raft trace exceeds byte bound");
    println!("{CAPTURE_TAG} {} {}", content_ref_from_bytes(&bytes), hex::encode(&bytes));
}

// Only these turns are applied; no inbox/timer loop runs. C receives frames but does not
// handle Raft events. A's first write is acknowledged before B learns of any commit.
async fn capture(a: &mut Node, b: &mut Node, c: &mut Node) -> [IOValue; CAPTURE_STEPS] {
    let earlier_request = test_ref("read-index-trace-earlier-write");
    let retry_request = test_ref("read-index-trace-retry");
    let new_request = test_ref("read-index-trace-new-write");
    let current_request = test_ref("read-index-trace-current");
    assert_ne!(retry_request, current_request);

    workflow::elect_node_a(a, b, c).await.expect("trace elect A");
    let applied = workflow::replicate_until_commit(a, b, c, &earlier_request).await.expect("trace earlier commit");
    let (earlier_index, proposal) = observed_proposal(a, &applied, &earlier_request);
    let first_leader = LeaderBoundary::observe(&applied.next);
    assert_eq!(first_leader.node_id, NODE_A);
    assert_eq!(first_leader.commit_index, earlier_index);
    assert_eq!(first_leader.last_applied, earlier_index);
    let earlier_ack = record("earlier-write-ack", vec![first_leader.value(), proposal]);

    workflow::elect_node_a(b, a, c).await.expect("trace elect B");
    let checkpoint = LeaderBoundary::observe(b.service.state());
    assert_eq!(checkpoint.node_id, NODE_B);
    assert!(checkpoint.term > first_leader.term);
    assert_eq!(checkpoint.last_log_index, earlier_index);
    assert_eq!(checkpoint.last_log_term, first_leader.commit_term);
    assert_eq!(b.service.state().log.last().expect("B's replicated write").request_ref, earlier_request);
    assert!(checkpoint.commit_index < earlier_index);
    assert_ne!(checkpoint.commit_term, checkpoint.term);
    assert!(b.service.state().pending_reads.is_empty());

    let outcome = b
        .service
        .handle_event(ReplicaEvent::Read {
            request_ref: retry_request.clone(),
            mode: ReadMode::Linearizable,
        })
        .await;
    let retry = workflow::require_applied_transition(outcome).expect("trace retry read turn");
    let (retried_index, retry_outcome) = observed_read(b, &retry, &retry_request, ReadDisposition::Retryable);
    let retry_boundary = LeaderBoundary::observe(&retry.next);
    assert_eq!(retry_boundary, checkpoint);
    assert_eq!(retried_index, retry_boundary.last_applied);
    assert!(retry.next.pending_reads.is_empty());

    let applied = workflow::replicate_until_commit(b, a, c, &new_request).await.expect("trace B commit");
    let persist = applied
        .observations
        .iter()
        .find(|effect| effect.kind == ReplicaEffectKind::PersistCommit)
        .expect("missing executed B commit persistence");
    // The commit of B's current-term entry also publishes the now-covered earlier entry.
    let covered = observed_outcome(b, &applied, ReplicaEffectKind::ProposalOutcome);
    assert_eq!(covered.kind, ReplicaObservationKind::Proposal {
        request_ref: earlier_request,
        disposition: ProposalDisposition::Committed,
        committed_index: Some(earlier_index),
    });
    let (new_index, proposal) = observed_proposal(b, &applied, &new_request);
    let committed = LeaderBoundary::observe(&applied.next);
    assert_eq!(committed.node_id, checkpoint.node_id);
    assert_eq!(committed.term, checkpoint.term);
    assert_eq!(committed.commit_term, committed.term);
    assert_eq!(committed.commit_index, new_index);
    assert_eq!(committed.last_log_index, new_index);
    assert_eq!(committed.last_applied, new_index);
    assert!(new_index > earlier_index);
    assert!(applied.next.pending_reads.is_empty());

    let [started, current] = capture_quorum_read(b, a, c, &current_request, &committed, earlier_index).await;
    [
        earlier_ack,
        record("leader-checkpoint", vec![checkpoint.value()]),
        record("read-retry", vec![retry_boundary.value(), retry_outcome]),
        record("new-commit", vec![committed.value(), proposal, string(&persist.evidence_ref)]),
        started,
        current,
    ]
}

async fn capture_quorum_read(
    leader: &mut Node,
    follower: &mut Node,
    other: &mut Node,
    request_ref: &str,
    committed: &LeaderBoundary,
    earlier_index: u64,
) -> [IOValue; 2] {
    let (started, probe) = workflow::start_quorum_read(leader, follower, other, request_ref)
        .await
        .expect("trace start quorum read");
    assert!(started.transport_failures.is_empty());
    let started_boundary = LeaderBoundary::observe(&started.next);
    assert_eq!(started_boundary, *committed);
    let pending = started.next.pending_reads.get(request_ref).expect("missing B pending quorum read");
    assert_eq!(pending.request_ref, request_ref);
    assert_eq!(pending.term, committed.term);
    assert_eq!(pending.required_index, committed.commit_index);
    assert!(pending.acknowledgements.iter().map(String::as_str).eq([committed.node_id.as_str()]));
    assert!(
        matches!(leader._control_receiver.try_recv(), Err(TryRecvError::Empty)),
        "read observed before A's ACK"
    );
    let start = record("read-started", vec![
        started_boundary.value(),
        string(&pending.request_ref),
        u64_value(pending.term),
        u64_value(pending.required_index),
        sequence(pending.acknowledgements.iter().map(string).collect()),
    ]);

    let received = workflow::quorum_read_acknowledgement(leader, follower, probe).await.expect("trace receive A ACK");
    let ack = observed_ack(&received, &started.next, pending);
    let outcome = leader.service.handle_event(received.event).await;
    let applied = workflow::require_applied_transition(outcome).expect("trace handle A ACK on B");
    let (read_index, read_outcome) = observed_read(leader, &applied, request_ref, ReadDisposition::Current);
    let completed = LeaderBoundary::observe(&applied.next);
    assert_eq!(completed, *committed);
    assert!(applied.next.pending_reads.is_empty(), "B left the completed read pending");
    assert_eq!(read_index, pending.required_index);
    assert!(read_index >= earlier_index, "current quorum read omitted an acknowledged write");
    [
        start,
        record("quorum-read-current", vec![completed.value(), ack, read_outcome]),
    ]
}

fn observed_ack(received: &ReceivedReplicaEvent, state: &ReplicaState, pending: &PendingReplicaRead) -> IOValue {
    let ReplicaEvent::Message {
        envelope:
            ReplicaMessageEnvelope {
                from,
                to,
                group_binding_ref,
                message:
                    Message::ReadAcknowledgement {
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
    assert_eq!(from, NODE_A);
    assert_eq!(from, follower_id);
    assert_ne!(from, to);
    assert_eq!(*to, state.node_id);
    assert_eq!(*group_binding_ref, state.profile.group_binding_ref);
    assert_eq!(*term, pending.term);
    assert_eq!(*request_ref, pending.request_ref);
    assert!(!pending.acknowledgements.contains(from));
    record("read-ack", vec![
        string(from),
        string(to),
        u64_value(*term),
        string(request_ref),
        string(&received.transport_evidence.request_ref),
    ])
}

fn observed_outcome(
    node: &mut Node,
    applied: &ExecutedReplicaTransition,
    kind: ReplicaEffectKind,
) -> ReplicaObservation {
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

fn observed_proposal(node: &mut Node, applied: &ExecutedReplicaTransition, expected_request: &str) -> (u64, IOValue) {
    let observation = observed_outcome(node, applied, ReplicaEffectKind::ProposalOutcome);
    let ReplicaObservationKind::Proposal {
        request_ref,
        disposition,
        committed_index: Some(index),
    } = observation.kind
    else {
        panic!("expected a proposal channel observation with an index for {expected_request}");
    };
    assert_eq!(request_ref, expected_request);
    assert_eq!(disposition, ProposalDisposition::Committed);
    assert_eq!(applied.next.completed_requests.get(&request_ref), Some(&index));
    let value = record("proposal-outcome", vec![
        string(request_ref),
        string(disposition.as_str()),
        u64_value(index),
        string(observation.receipt_ref),
    ]);
    (index, value)
}

fn observed_read(
    node: &mut Node,
    applied: &ExecutedReplicaTransition,
    expected_request: &str,
    expected_disposition: ReadDisposition,
) -> (u64, IOValue) {
    let observation = observed_outcome(node, applied, ReplicaEffectKind::ReadOutcome);
    let ReplicaObservationKind::Read {
        request_ref,
        mode,
        disposition,
        observed_index,
    } = observation.kind
    else {
        panic!("expected a read channel observation for {expected_request}");
    };
    assert_eq!(request_ref, expected_request);
    assert_eq!(mode, ReadMode::Linearizable);
    assert_eq!(disposition, expected_disposition);
    let value = record("read-outcome", vec![
        string(request_ref),
        string(mode.as_str()),
        string(disposition.as_str()),
        u64_value(observed_index),
        string(observation.receipt_ref),
    ]);
    (observed_index, value)
}

#[derive(Debug, PartialEq, Eq)]
struct LeaderBoundary {
    node_id: String,
    term: u64,
    last_log_index: u64,
    last_log_term: u64,
    commit_index: u64,
    commit_term: u64,
    last_applied: u64,
}

impl LeaderBoundary {
    fn observe(state: &ReplicaState) -> Self {
        assert_eq!(state.role, ReplicaRole::Leader);
        assert!(state.snapshot.is_none(), "the fixed trace does not compact its log");
        let last = state.log.last().expect("missing leader log tip in trace");
        let commit_term = if state.commit_index == INITIAL_COMMIT_INDEX {
            INITIAL_TERM // Raft's term at the empty prefix.
        } else {
            state
                .log
                .iter()
                .find(|entry| entry.index == state.commit_index)
                .expect("missing commit entry in trace")
                .term
        };
        Self {
            node_id: state.node_id.clone(),
            term: state.current_term,
            last_log_index: last.index,
            last_log_term: last.term,
            commit_index: state.commit_index,
            commit_term,
            last_applied: state.last_applied,
        }
    }

    fn value(&self) -> IOValue {
        record("leader-boundary", vec![
            string(&self.node_id),
            u64_value(self.term),
            u64_value(self.last_log_index),
            u64_value(self.last_log_term),
            u64_value(self.commit_index),
            u64_value(self.commit_term),
            u64_value(self.last_applied),
        ])
    }
}
