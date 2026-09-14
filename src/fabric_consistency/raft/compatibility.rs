mod wire;

const CASE_COUNT: usize = 10;
const PAIR_COUNT: usize = 2;
const SNAPSHOT_CASE_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Family {
    Vote,
    Append,
    Read,
    Snapshot,
}

pub(super) struct Case {
    pub name: &'static str,
    pub family: Family,
    pub receiver: super::ReplicaState,
    pub envelope: super::ReplicaMessageEnvelope,
}

impl Case {
    fn new(name: &'static str, receiver: super::ReplicaState, envelope: super::ReplicaMessageEnvelope) -> Self {
        assert_eq!(receiver.node_id, envelope.to);
        let family = match envelope.message {
            super::RaftMessage::RequestVote { .. } | super::RaftMessage::VoteResponse { .. } => Family::Vote,
            super::RaftMessage::AppendEntries { .. } | super::RaftMessage::AppendResponse { .. } => Family::Append,
            super::RaftMessage::ReadProbe { .. } | super::RaftMessage::ReadAcknowledgement { .. } => Family::Read,
            super::RaftMessage::InstallSnapshot { .. } | super::RaftMessage::SnapshotResponse { .. } => {
                Family::Snapshot
            }
        };
        Self {
            name,
            family,
            receiver,
            envelope,
        }
    }
}

struct Election {
    request: Case,
    response: Case,
    empty_append: Case,
    leader: super::ReplicaState,
    follower: super::ReplicaState,
}

pub(super) fn cases() -> [Case; CASE_COUNT] {
    let election = election();
    let (append, append_response, committed) = replication(&election.leader, &election.follower);
    let [read, acknowledgement] = reads(&election.leader, &election.follower);
    let [snapshot, snapshot_response, empty_snapshot] = snapshots(&committed);
    [
        election.request,
        election.response,
        election.empty_append,
        append,
        append_response,
        read,
        acknowledgement,
        snapshot,
        snapshot_response,
        empty_snapshot,
    ]
}

fn election() -> Election {
    let group = super::tests::active_group();
    let node_a = super::tests::started_state(&group, super::tests::NODE_A);
    let node_b = super::tests::started_state(&group, super::tests::NODE_B);
    let election = super::apply_replica_event(&node_a, super::ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("compatibility election");
    let request = super::tests::sent_envelope_to(&election, super::tests::NODE_B);
    let vote = super::apply_replica_event(&node_b, super::ReplicaEvent::Message {
        envelope: request.clone(),
    })
    .expect("compatibility vote");
    let response = super::tests::sent_envelope_to(&vote, super::tests::NODE_A);
    let leadership = super::apply_replica_event(&election.next, super::ReplicaEvent::Message {
        envelope: response.clone(),
    })
    .expect("compatibility leadership");
    let empty_append = super::tests::sent_envelope_to(&leadership, super::tests::NODE_B);
    Election {
        request: Case::new("request-vote", node_b, request),
        response: Case::new("vote-response", election.next, response),
        empty_append: Case::new("append-empty", vote.next.clone(), empty_append),
        leader: leadership.next,
        follower: vote.next,
    }
}

fn replication(leader: &super::ReplicaState, follower: &super::ReplicaState) -> (Case, Case, super::ReplicaState) {
    let proposal = super::apply_replica_event(leader, super::ReplicaEvent::Propose {
        request_ref: super::tests::test_ref("compatibility-request"),
        command_ref: super::tests::test_ref("compatibility-command"),
        command_schema_ref: super::tests::test_ref("compatibility-command-schema"),
    })
    .expect("compatibility proposal");
    let append = super::tests::sent_envelope_to(&proposal, super::tests::NODE_B);
    let replicated = super::apply_replica_event(follower, super::ReplicaEvent::Message {
        envelope: append.clone(),
    })
    .expect("compatibility replication");
    let response = super::tests::sent_envelope_to(&replicated, super::tests::NODE_A);
    let committed = super::apply_replica_event(&proposal.next, super::ReplicaEvent::Message {
        envelope: response.clone(),
    })
    .expect("compatibility commit");
    (
        Case::new("append-entries", follower.clone(), append),
        Case::new("append-response", proposal.next, response),
        committed.next,
    )
}

fn reads(leader: &super::ReplicaState, follower: &super::ReplicaState) -> [Case; PAIR_COUNT] {
    let pending = super::apply_replica_event(leader, super::ReplicaEvent::Read {
        request_ref: super::tests::test_ref("compatibility-read"),
        mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
    })
    .expect("compatibility read");
    let probe = super::tests::sent_envelope_to(&pending, super::tests::NODE_B);
    let acknowledged = super::apply_replica_event(follower, super::ReplicaEvent::Message {
        envelope: probe.clone(),
    })
    .expect("compatibility acknowledgement");
    let acknowledgement = super::tests::sent_envelope_to(&acknowledged, super::tests::NODE_A);
    [
        Case::new("read-probe", follower.clone(), probe),
        Case::new("read-acknowledgement", pending.next, acknowledgement),
    ]
}

fn snapshots(committed: &super::ReplicaState) -> [Case; SNAPSHOT_CASE_COUNT] {
    let saved = super::apply_replica_event(committed, super::ReplicaEvent::CreateSnapshot {
        application_state_ref: super::tests::test_ref("compatibility-snapshot-state"),
    })
    .expect("compatibility snapshot");
    let heartbeat = super::apply_replica_event(&saved.next, super::ReplicaEvent::HeartbeatTimeout)
        .expect("compatibility snapshot heartbeat");
    let install = super::tests::sent_envelope_to(&heartbeat, super::tests::NODE_C);
    let follower = super::tests::started_state(&super::tests::active_group(), super::tests::NODE_C);
    let installed = super::apply_replica_event(&follower, super::ReplicaEvent::Message {
        envelope: install.clone(),
    })
    .expect("compatibility snapshot install");
    let response = super::tests::sent_envelope_to(&installed, super::tests::NODE_A);
    let mut empty = install.clone();
    let super::RaftMessage::InstallSnapshot { snapshot, .. } = &mut empty.message else {
        panic!("compatibility fixture must contain an install snapshot");
    };
    assert!(!snapshot.completed_requests.is_empty());
    snapshot.completed_requests.clear();
    snapshot.snapshot_ref = super::snapshot_ref(snapshot).expect("empty snapshot identity");
    [
        Case::new("install-snapshot", follower.clone(), install),
        Case::new("snapshot-response", saved.next, response),
        Case::new("snapshot-empty", follower, empty),
    ]
}
