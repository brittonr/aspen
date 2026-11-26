//! Fixtures for driving the kernel directly: a deterministic test identity port, a started
//! three-voter group, and the elections and commits most tests start from.

use crate::model::*;
pub use crate::transition::StaleTimerEvent;
pub use crate::transition::is_snapshot_due;
pub use crate::transition::stale_timer_event;

const SERVICE_GENERATION: u64 = 1;
const CONFIG_EPOCH: u64 = 1;
const FENCING_EPOCH: u64 = 1;
const HEARTBEAT_TICKS: u64 = 2;
const ELECTION_MIN_TICKS: u64 = 4;
const ELECTION_MAX_TICKS: u64 = 8;
const EFFECT_LIMIT: usize = 16;
pub const TEST_REQUEST_RETENTION_ENTRIES: u64 = 2_048;
pub const NODE_A: &str = "node-a";
pub const NODE_B: &str = "node-b";
pub const NODE_C: &str = "node-c";

/// A deterministic identity port: BLAKE3 over a domain tag and length-framed fields. It is not the
/// shell's canonical Preserves identity, but it is injective over the same facts, which is all the
/// transition function relies on.
pub struct TestIdentity;

struct Frame(Vec<u8>);

impl Frame {
    fn new(domain: &str) -> Self {
        let mut frame = Self(Vec::new());
        frame.text(domain);
        frame
    }

    fn text(&mut self, value: &str) -> &mut Self {
        self.number(value.len() as u64);
        self.0.extend_from_slice(value.as_bytes());
        self
    }

    fn number(&mut self, value: u64) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }

    fn entry(&mut self, entry: &ReplicatedEntry) -> &mut Self {
        self.number(entry.index)
            .number(entry.term)
            .text(&entry.request_ref)
            .text(&entry.command_ref)
            .text(&entry.command_schema_ref)
    }

    fn finish(&self) -> String {
        format!("blake3:{}", blake3::hash(&self.0).to_hex())
    }
}

impl crate::identity::ReplicaIdentity for TestIdentity {
    fn election_timer_ref(
        &self,
        timer: &crate::identity::ElectionTimer<'_>,
    ) -> Result<String, crate::denial::IdentityError> {
        Ok(Frame::new("election-timer")
            .text(timer.group_binding_ref)
            .text(timer.node_id)
            .number(timer.service_generation)
            .number(timer.term)
            .number(timer.sequence)
            .finish())
    }

    fn snapshot_ref(&self, snapshot: &ReplicaSnapshot) -> Result<String, crate::denial::IdentityError> {
        let mut frame = Frame::new("snapshot");
        frame
            .text(&snapshot.group_binding_ref)
            .text(&snapshot.membership_ref)
            .number(snapshot.config_epoch)
            .number(snapshot.fencing_epoch)
            .number(snapshot.last_included_index)
            .number(snapshot.last_included_term)
            .text(&snapshot.application_state_ref)
            .number(snapshot.completed_requests.len() as u64);
        for (request_ref, index) in &snapshot.completed_requests {
            frame.text(request_ref).number(*index);
        }
        Ok(frame.finish())
    }

    fn recovery_barrier_ref(&self, timer_ref: &str, term: u64) -> Result<String, crate::denial::IdentityError> {
        Ok(Frame::new("recovery-barrier").text(timer_ref).number(term).finish())
    }

    fn entry_ref(&self, entry: &ReplicatedEntry) -> Result<String, crate::denial::IdentityError> {
        Ok(Frame::new("entry").entry(entry).finish())
    }

    fn recovery_corpus_ref(
        &self,
        corpus: &crate::identity::RecoveryCorpus<'_>,
    ) -> Result<String, crate::denial::IdentityError> {
        let mut frame = Frame::new("recovery-corpus");
        frame.number(corpus.term).number(corpus.commit_index);
        match corpus.snapshot {
            Some(snapshot) => frame.number(1).text(&snapshot.snapshot_ref),
            None => frame.number(0),
        };
        frame.number(corpus.log.len() as u64);
        for entry in corpus.log {
            frame.entry(entry);
        }
        Ok(frame.finish())
    }
}

pub fn test_ref(label: &str) -> String {
    Frame::new("test-ref").text(label).finish()
}

pub fn apply_replica_event(state: &ReplicaState, event: ReplicaEvent) -> crate::denial::Result<ReplicaTransition> {
    crate::transition::apply_replica_event(state, event, &TestIdentity)
}

pub fn snapshot_ref(snapshot: &ReplicaSnapshot) -> crate::denial::Result<String> {
    Ok(crate::identity::ReplicaIdentity::snapshot_ref(&TestIdentity, snapshot)?)
}

pub fn election_timer_ref(state: &ReplicaState) -> String {
    crate::identity::ReplicaIdentity::election_timer_ref(&TestIdentity, &crate::identity::ElectionTimer {
        group_binding_ref: &state.profile.group_binding_ref,
        node_id: &state.node_id,
        service_generation: state.profile.service_generation,
        term: state.current_term,
        sequence: state.election_timer_sequence,
    })
    .expect("test election timer ref")
}

pub fn profile() -> ReplicaProfile {
    ReplicaProfile {
        profile_ref: test_ref("live-raft-profile"),
        group_binding_ref: test_ref("group-binding"),
        service_generation: SERVICE_GENERATION,
        protocol_ref: test_ref("protocol"),
        durable_log_ref: test_ref("durable-log"),
        snapshot_store_ref: test_ref("snapshot-store"),
        timer_profile_ref: test_ref("timer-profile"),
        entropy_profile_ref: test_ref("entropy-profile"),
        placement_ref: test_ref("placement"),
        fencing_ref: test_ref("fencing"),
        fencing_epoch: FENCING_EPOCH,
        supervision_ref: test_ref("supervision"),
        resource_profile_ref: test_ref("resources"),
        heartbeat_ticks: HEARTBEAT_TICKS,
        election_min_ticks: ELECTION_MIN_TICKS,
        election_max_ticks: ELECTION_MAX_TICKS,
        max_log_entries: MAX_REPLICA_LOG_ENTRIES,
        snapshot_threshold_entries: MAX_REPLICA_LOG_ENTRIES / 2,
        request_retention_entries: TEST_REQUEST_RETENTION_ENTRIES,
        max_message_entries: MAX_REPLICA_MESSAGE_ENTRIES,
        max_effects_per_step: EFFECT_LIMIT,
    }
}

pub fn membership() -> StaticMembership {
    StaticMembership {
        membership_ref: test_ref("membership"),
        config_epoch: CONFIG_EPOCH,
        voters: vec![NODE_C.to_string(), NODE_A.to_string(), NODE_B.to_string()],
    }
}

/// A node of the test group as start admission leaves it.
pub fn started_state(node_id: &str) -> ReplicaState {
    let profile = profile();
    crate::admission::validate_profile(&profile).expect("admitted profile");
    let membership_ref = test_ref("membership");
    let membership =
        crate::admission::validate_membership(node_id, membership(), &crate::admission::MembershipBinding {
            membership_ref: &membership_ref,
            config_epoch: CONFIG_EPOCH,
        })
        .expect("admitted membership");
    crate::admission::initial_state(node_id.to_string(), membership, profile, &TestIdentity).expect("started replica")
}

pub fn sent_envelope_to(transition: &ReplicaTransition, recipient: &str) -> ReplicaMessageEnvelope {
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            ReplicaEffect::Send { envelope } if envelope.to == recipient => Some(envelope.clone()),
            _ => None,
        })
        .expect("sent Raft envelope")
}

pub fn elect_node_a() -> (ReplicaState, ReplicaState) {
    let node_a = started_state(NODE_A);
    let node_b = started_state(NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("node A election");
    let vote_request = sent_envelope_to(&election, NODE_B);
    let vote =
        apply_replica_event(&node_b, ReplicaEvent::Message { envelope: vote_request }).expect("node B vote response");
    let vote_response = sent_envelope_to(&vote, NODE_A);
    let leader = apply_replica_event(&election.next, ReplicaEvent::Message {
        envelope: vote_response,
    })
    .expect("node A leadership");
    assert_eq!(leader.next.role, ReplicaRole::Leader);
    (leader.next, vote.next)
}

pub fn committed_leader() -> ReplicaState {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("committed-helper-request"),
        command_ref: test_ref("committed-helper-command"),
        command_schema_ref: test_ref("committed-helper-schema"),
    })
    .expect("helper proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("helper follower append");
    let response = sent_envelope_to(&replicated, NODE_A);
    apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("helper majority commit")
        .next
}
