//! Start admission: the profile, membership, and initial state a replica may start from.
//!
//! The `molten` shell checks the consistency-group binding and the fabric port cohort, then
//! composes these checks into a [`ReplicaStartPlan`].

use crate::model::*;

/// The fabric ports a replica binds: transport, durable log, snapshot store, timer, entropy,
/// membership, and placement.
pub const REQUIRED_REPLICA_PORT_COUNT: usize = 7;
/// The fewest effects that hold one complete static-replica step.
pub const MINIMUM_LIVE_REPLICA_STEP_EFFECTS: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaStartPlan {
    pub state: ReplicaState,
    pub service_id: String,
    pub application_manifest_ref: String,
    pub initial_effects: Vec<ReplicaEffect>,
    pub port_binding_refs: Vec<String>,
    pub production_admitted: bool,
}

/// The membership binding the consistency group admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipBinding<'a> {
    pub membership_ref: &'a str,
    pub config_epoch: u64,
}

const PROFILE_REFS: usize = 11;

fn profile_refs(profile: &ReplicaProfile) -> [(&str, &'static str); PROFILE_REFS] {
    [
        (profile.profile_ref.as_str(), "live Raft profile ref"),
        (profile.group_binding_ref.as_str(), "live Raft group binding ref"),
        (profile.protocol_ref.as_str(), "live Raft protocol ref"),
        (profile.durable_log_ref.as_str(), "live Raft durable log ref"),
        (profile.snapshot_store_ref.as_str(), "live Raft snapshot store ref"),
        (profile.timer_profile_ref.as_str(), "live Raft timer profile ref"),
        (profile.entropy_profile_ref.as_str(), "live Raft entropy profile ref"),
        (profile.placement_ref.as_str(), "live Raft placement ref"),
        (profile.fencing_ref.as_str(), "live Raft fencing ref"),
        (profile.supervision_ref.as_str(), "live Raft supervision ref"),
        (profile.resource_profile_ref.as_str(), "live Raft resource profile ref"),
    ]
}

pub fn validate_profile(profile: &ReplicaProfile) -> crate::denial::Result<()> {
    for (reference, label) in profile_refs(profile) {
        crate::denial::refs::validate_labelled_content_ref(reference, label)?;
    }
    if profile.service_generation == 0 || profile.fencing_epoch == 0 {
        return Err(crate::denial::ReplicaError::Denied("live Raft generation and fencing epoch must be positive"));
    }
    if profile.heartbeat_ticks == 0
        || profile.election_min_ticks <= profile.heartbeat_ticks
        || profile.election_max_ticks < profile.election_min_ticks
    {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft timer bounds require heartbeat < election minimum <= election maximum",
        ));
    }
    if profile.max_log_entries == 0 || profile.max_log_entries > MAX_REPLICA_LOG_ENTRIES {
        return Err(crate::denial::ReplicaError::Denied("live Raft log-entry bound is outside the admitted range"));
    }
    validate_retention_bounds(profile)?;
    if profile.max_message_entries == 0 || profile.max_message_entries > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(crate::denial::ReplicaError::Denied("live Raft message-entry bound is outside the admitted range"));
    }
    if profile.max_effects_per_step < MINIMUM_LIVE_REPLICA_STEP_EFFECTS
        || profile.max_effects_per_step > MAX_REPLICA_EFFECTS
    {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft effect bound cannot admit a complete static-replica step",
        ));
    }
    Ok(())
}

/// The snapshot threshold must leave room below the log bound for entries that are not yet applied,
/// and the duplicate-request window must be positive and no wider than the largest admitted log.
// r[impl molten.consensus.snapshot_threshold]
// r[impl molten.consensus.bounded_request_dedup]
pub fn validate_retention_bounds(profile: &ReplicaProfile) -> crate::denial::Result<()> {
    if profile.snapshot_threshold_entries == 0 || profile.snapshot_threshold_entries >= profile.max_log_entries {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft snapshot threshold must be positive and below the log-entry bound",
        ));
    }
    let maximum_window = u64::try_from(MAX_REPLICA_LOG_ENTRIES).map_err(|_| {
        crate::denial::ReplicaError::Overflow("live Raft log-entry bound exceeds duplicate-request index range")
    })?;
    if profile.request_retention_entries == 0 || profile.request_retention_entries > maximum_window {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft duplicate-request window is outside the admitted range",
        ));
    }
    Ok(())
}

/// Admit the static membership a node starts in; returns it with its voters sorted.
pub fn validate_membership(
    node_id: &str,
    mut membership: StaticMembership,
    binding: &MembershipBinding<'_>,
) -> crate::denial::Result<StaticMembership> {
    crate::denial::refs::validate_identifier(node_id, "live Raft node id")?;
    crate::denial::refs::validate_labelled_content_ref(&membership.membership_ref, "live Raft membership ref")?;
    if membership.membership_ref != binding.membership_ref || membership.config_epoch != binding.config_epoch {
        return Err(crate::denial::ReplicaError::Denied("live Raft membership ref or configuration epoch is stale"));
    }
    if membership.voters.len() != STATIC_VOTER_COUNT {
        return Err(crate::denial::ReplicaError::VoterCount {
            required: STATIC_VOTER_COUNT,
        });
    }
    let mut unique = alloc::collections::BTreeSet::new();
    for voter in &membership.voters {
        crate::denial::refs::validate_identifier(voter, "live Raft voter id")?;
        if !unique.insert(voter.as_str()) {
            return Err(crate::denial::ReplicaError::Denied("live Raft membership contains a duplicate voter"));
        }
    }
    if !unique.contains(node_id) {
        return Err(crate::denial::ReplicaError::Denied(
            "live Raft node is absent from the admitted static membership",
        ));
    }
    membership.voters.sort();
    Ok(membership)
}

/// The state a node starts in: a running follower voter at the initial term with an empty log.
pub fn initial_state(
    node_id: String,
    membership: StaticMembership,
    profile: ReplicaProfile,
    identity: &dyn crate::identity::ReplicaIdentity,
) -> crate::denial::Result<ReplicaState> {
    let active_election_timer_ref = identity.election_timer_ref(&crate::identity::ElectionTimer {
        group_binding_ref: &profile.group_binding_ref,
        node_id: &node_id,
        service_generation: profile.service_generation,
        term: INITIAL_TERM,
        sequence: INITIAL_ELECTION_TIMER_SEQUENCE,
    })?;
    Ok(ReplicaState {
        profile,
        node_id,
        membership,
        role: ReplicaRole::Follower,
        lifecycle: ReplicaLifecycle::Running,
        rejoin: ReplicaRejoin::Voter,
        current_term: INITIAL_TERM,
        election_timer_sequence: INITIAL_ELECTION_TIMER_SEQUENCE,
        active_election_timer_ref,
        voted_for: None,
        leader_id: None,
        log: Vec::new(),
        commit_index: INITIAL_COMMIT_INDEX,
        last_applied: INITIAL_COMMIT_INDEX,
        snapshot: None,
        completed_requests: alloc::collections::BTreeMap::new(),
        pending_reads: alloc::collections::BTreeMap::new(),
        votes_received: alloc::collections::BTreeSet::new(),
        next_index: alloc::collections::BTreeMap::new(),
        match_index: alloc::collections::BTreeMap::new(),
        quorum_confirmed_term: None,
    })
}

/// The first step a started replica executes: persist the initial hard state, arm its timer.
pub fn initial_effects(state: &ReplicaState) -> crate::denial::Result<Vec<ReplicaEffect>> {
    let effects = vec![
        ReplicaEffect::PersistHardState {
            term: INITIAL_TERM,
            voted_for: None,
            is_recovery_required: false,
        },
        ReplicaEffect::ArmElectionTimer {
            timer_ref: state.active_election_timer_ref.clone(),
        },
    ];
    if effects.len() > state.profile.max_effects_per_step {
        return Err(crate::denial::ReplicaError::Denied("live Raft startup exceeds its admitted effect bound"));
    }
    Ok(effects)
}
