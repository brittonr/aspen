#[path = "observations.rs"]
mod observations;

const CAPTURE_TAG: &str = "live-raft-read-index-trace-v1";
const CAPTURE_STEPS: usize = 6;
const MAX_PACKED_CAPTURE_BYTES: usize = 4_096;

// r[impl molten.testing.live_raft_trace.capture]
// r[verify molten.testing.live_raft_trace.capture]
#[tokio::test]
#[ignore = "opt-in live Raft read-index diagnostic; select explicitly with --ignored --nocapture"]
async fn records_read_index_after_leader_change() {
    let group = super::super::tests::active_group();
    let listener_a =
        crate::fabric_transport::cross_process::tests::listener_with_secret(super::NODE_A_SECRET_BYTE).await;
    let listener_b =
        crate::fabric_transport::cross_process::tests::listener_with_secret(super::NODE_B_SECRET_BYTE).await;
    let listener_c =
        crate::fabric_transport::cross_process::tests::listener_with_secret(super::NODE_C_SECRET_BYTE).await;
    let node_a = super::super::tests::NODE_A;
    let node_b = super::super::tests::NODE_B;
    let node_c = super::super::tests::NODE_C;
    let endpoints = std::collections::BTreeMap::from([
        (node_a.to_string(), listener_a.handoff().clone()),
        (node_b.to_string(), listener_b.handoff().clone()),
        (node_c.to_string(), listener_c.handoff().clone()),
    ]);
    let [id_a, id_b, id_c] = [node_a, node_b, node_c].map(|id| &endpoints[id].descriptor.public_endpoint_identity);
    assert_ne!(id_a, id_b, "trace needs distinct loopback listeners");
    assert_ne!(id_a, id_c, "trace needs distinct loopback listeners");
    assert_ne!(id_b, id_c, "trace needs distinct loopback listeners");

    let mut a = super::setup::build_node(&group, node_a, listener_a, &endpoints).await.expect("trace node A");
    let mut b = super::setup::build_node(&group, node_b, listener_b, &endpoints).await.expect("trace node B");
    let mut c = super::setup::build_node(&group, node_c, listener_c, &endpoints).await.expect("trace node C");
    let steps = run_turns(&mut a, &mut b, &mut c).await;
    let state = b.service.state();
    assert_eq!(state.profile.group_binding_ref, group.binding_ref);
    assert!(state.membership.voters.iter().map(String::as_str).eq([node_a, node_b, node_c]));
    let value = crate::codec::record(CAPTURE_TAG, vec![
        crate::codec::string(&state.profile.group_binding_ref),
        crate::codec::string(&state.membership.membership_ref),
        crate::codec::sequence(state.membership.voters.iter().map(crate::codec::string).collect()),
        crate::codec::sequence(Vec::from(steps)),
    ]);
    super::setup::close_node(a).await;
    super::setup::close_node(b).await;
    super::setup::close_node(c).await;

    let bytes = crate::codec::canonical_bytes(&value).expect("pack live Raft trace");
    assert!(bytes.len() <= MAX_PACKED_CAPTURE_BYTES, "packed live Raft trace exceeds byte bound");
    println!("{CAPTURE_TAG} {} {}", crate::codec::content_ref_from_bytes(&bytes), hex::encode(&bytes));
}

// Only these turns are applied; no inbox/timer loop runs. C receives frames but does not
// handle Raft events. A's first write is acknowledged before B learns of any commit.
async fn run_turns(
    a: &mut super::Node,
    b: &mut super::Node,
    c: &mut super::Node,
) -> [preserves::IOValue; CAPTURE_STEPS] {
    let earlier_request = super::super::tests::test_ref("read-index-trace-earlier-write");
    let retry_request = super::super::tests::test_ref("read-index-trace-retry");
    let new_request = super::super::tests::test_ref("read-index-trace-new-write");
    let current_request = super::super::tests::test_ref("read-index-trace-current");
    assert_ne!(retry_request, current_request);

    let (first_leader, earlier_index, earlier_ack) = write_before_change(a, b, c, &earlier_request).await;

    super::workflow::elect_node_a(b, a, c).await.expect("trace elect B");
    let checkpoint = LeaderBoundary::observe(b.service.state());
    assert_eq!(checkpoint.node_id, super::super::tests::NODE_B);
    assert!(checkpoint.term > first_leader.term);
    assert_eq!(checkpoint.last_log_index, earlier_index);
    assert_eq!(checkpoint.last_log_term, first_leader.commit_term);
    assert_eq!(b.service.state().log.last().expect("B's replicated write").request_ref, earlier_request);
    assert!(checkpoint.commit_index < earlier_index);
    assert_ne!(checkpoint.commit_term, checkpoint.term);
    assert!(b.service.state().pending_reads.is_empty());

    let outcome = b
        .service
        .handle_event(super::super::ReplicaEvent::Read {
            request_ref: retry_request.clone(),
            mode: crate::fabric_consistency::ReadMode::Linearizable,
        })
        .await;
    let retry = super::workflow::require_applied_transition(outcome).expect("trace retry read turn");
    let (retried_index, retry_outcome) =
        observations::observed_read(b, &retry, &retry_request, super::super::ReadDisposition::Retryable);
    let retry_boundary = LeaderBoundary::observe(&retry.next);
    assert_eq!(retry_boundary, checkpoint);
    assert_eq!(retried_index, retry_boundary.last_applied);
    assert!(retry.next.pending_reads.is_empty());

    let applied = super::workflow::replicate_until_commit(b, a, c, &new_request).await.expect("trace B commit");
    let persist = applied
        .observations
        .iter()
        .find(|effect| effect.kind == super::super::executor::ReplicaEffectKind::PersistCommit)
        .expect("missing executed B commit persistence");
    // The commit of B's current-term entry also publishes the now-covered earlier entry.
    let covered =
        observations::observed_outcome(b, &applied, super::super::executor::ReplicaEffectKind::ProposalOutcome);
    assert_eq!(covered.kind, super::super::control::ReplicaObservationKind::Proposal {
        request_ref: earlier_request,
        disposition: super::super::ProposalDisposition::Committed,
        committed_index: Some(earlier_index),
    });
    let (new_index, proposal) = observations::observed_proposal(b, &applied, &new_request);
    let committed = LeaderBoundary::observe(&applied.next);
    assert_eq!(committed.node_id, checkpoint.node_id);
    assert_eq!(committed.term, checkpoint.term);
    assert_eq!(committed.commit_term, committed.term);
    assert_eq!(committed.commit_index, new_index);
    assert_eq!(committed.last_log_index, new_index);
    assert_eq!(committed.last_applied, new_index);
    assert!(new_index > earlier_index);
    assert!(applied.next.pending_reads.is_empty());

    let [started, current] = complete_quorum_read(b, a, c, &current_request, &committed, earlier_index).await;
    [
        earlier_ack,
        crate::codec::record("leader-checkpoint", vec![checkpoint.value()]),
        crate::codec::record("read-retry", vec![retry_boundary.value(), retry_outcome]),
        crate::codec::record("new-commit", vec![
            committed.value(),
            proposal,
            crate::codec::string(&persist.evidence_ref),
        ]),
        started,
        current,
    ]
}

async fn write_before_change(
    a: &mut super::Node,
    b: &mut super::Node,
    c: &mut super::Node,
    request: &str,
) -> (LeaderBoundary, u64, preserves::IOValue) {
    super::workflow::elect_node_a(a, b, c).await.expect("trace elect A");
    let applied = super::workflow::replicate_until_commit(a, b, c, request).await.expect("trace earlier commit");
    let (index, proposal) = observations::observed_proposal(a, &applied, request);
    let first_leader = LeaderBoundary::observe(&applied.next);
    assert_eq!(first_leader.node_id, super::super::tests::NODE_A);
    assert_eq!(first_leader.commit_index, index);
    assert_eq!(first_leader.last_applied, index);
    let ack = crate::codec::record("earlier-write-ack", vec![first_leader.value(), proposal]);
    (first_leader, index, ack)
}

async fn complete_quorum_read(
    leader: &mut super::Node,
    follower: &mut super::Node,
    other: &mut super::Node,
    request_ref: &str,
    committed: &LeaderBoundary,
    earlier_index: u64,
) -> [preserves::IOValue; 2] {
    let (started, probe) = super::workflow::start_quorum_read(leader, follower, other, request_ref)
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
        matches!(leader._control_receiver.try_recv(), Err(tokio::sync::mpsc::error::TryRecvError::Empty)),
        "read observed before A's ACK"
    );
    let start = crate::codec::record("read-started", vec![
        started_boundary.value(),
        crate::codec::string(&pending.request_ref),
        crate::codec::u64_value(pending.term),
        crate::codec::u64_value(pending.required_index),
        crate::codec::sequence(pending.acknowledgements.iter().map(crate::codec::string).collect()),
    ]);

    let received = super::workflow::quorum_read_acknowledgement(leader, follower, probe)
        .await
        .expect("trace receive A ACK");
    let ack = observations::observed_ack(&received, &started.next, pending);
    let outcome = leader.service.handle_event(received.event).await;
    let applied = super::workflow::require_applied_transition(outcome).expect("trace handle A ACK on B");
    let (read_index, read_outcome) =
        observations::observed_read(leader, &applied, request_ref, super::super::ReadDisposition::Current);
    let completed = LeaderBoundary::observe(&applied.next);
    assert_eq!(completed, *committed);
    assert!(applied.next.pending_reads.is_empty(), "B left the completed read pending");
    assert_eq!(read_index, pending.required_index);
    assert!(read_index >= earlier_index, "current quorum read omitted an acknowledged write");
    [
        start,
        crate::codec::record("quorum-read-current", vec![completed.value(), ack, read_outcome]),
    ]
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
    fn observe(state: &super::super::ReplicaState) -> Self {
        assert_eq!(state.role, super::super::ReplicaRole::Leader);
        assert!(state.snapshot.is_none(), "the fixed trace does not compact its log");
        let last = state.log.last().expect("missing leader log tip in trace");
        let commit_term = if state.commit_index == super::super::INITIAL_COMMIT_INDEX {
            super::super::INITIAL_TERM // Raft's term at the empty prefix.
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

    fn value(&self) -> preserves::IOValue {
        crate::codec::record("leader-boundary", vec![
            crate::codec::string(&self.node_id),
            crate::codec::u64_value(self.term),
            crate::codec::u64_value(self.last_log_index),
            crate::codec::u64_value(self.last_log_term),
            crate::codec::u64_value(self.commit_index),
            crate::codec::u64_value(self.commit_term),
            crate::codec::u64_value(self.last_applied),
        ])
    }
}
