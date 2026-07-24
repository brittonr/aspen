use preserves::IOValue;
use preserves::Value;
use preserves::ValueImpl;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_durability::LogRecord;

const HARD_STATE_ARITY: usize = 2;
const LOG_MUTATION_ARITY: usize = 2;
const SINGLE_INDEX_ARITY: usize = 1;
const SNAPSHOT_ARITY: usize = 9;
const COMPLETED_REQUEST_ARITY: usize = 2;
const OPTIONAL_VALUE_ARITY: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaRecoveryPlan {
    pub start_plan: ReplicaStartPlan,
    pub recovery_ref: String,
    pub durable_commit_index: u64,
    pub replay_entry_count: usize,
}

pub fn plan_replica_recovery(
    mut start_plan: ReplicaStartPlan,
    durable_records: &[LogRecord],
    snapshot_bytes: Option<&[u8]>,
) -> Result<ReplicaRecoveryPlan> {
    let mut state = reset_recovery_state(&start_plan.state)?;
    let mut expected_sequence = 0_u64;
    let mut durable_commit_index = INITIAL_COMMIT_INDEX;
    let mut durable_record_refs = Vec::with_capacity(durable_records.len());
    for record in durable_records {
        if record.sequence != expected_sequence {
            return Err(MoltenError::invalid_harness("live Raft recovery durable sequence has a gap"));
        }
        expected_sequence = expected_sequence
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("live Raft recovery durable sequence overflow"))?;
        if crate::preserves_rail::content_ref_from_bytes(&record.value) != record.value_ref {
            return Err(MoltenError::invalid_harness("live Raft recovery durable record content ref mismatch"));
        }
        replay_record(&mut state, &mut durable_commit_index, &record.value)?;
        durable_record_refs.push(record.value_ref.clone());
    }
    let snapshot = snapshot_bytes.map(parse_snapshot).transpose()?;
    install_recovery_snapshot(&mut state, durable_commit_index, snapshot)?;
    if durable_commit_index > support_last_log_index(&state) {
        return Err(MoltenError::invalid_harness("live Raft recovery commit boundary exceeds durable log"));
    }
    state.commit_index = durable_commit_index;
    let replay_start = state.snapshot.as_ref().map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index);
    let replay_entries = state
        .log
        .iter()
        .filter(|entry| entry.index > replay_start && entry.index <= durable_commit_index)
        .cloned()
        .collect::<Vec<_>>();
    for entry in state.log.iter().filter(|entry| entry.index <= durable_commit_index) {
        state.completed_requests.insert(entry.request_ref.clone(), entry.index);
    }
    state.last_applied = durable_commit_index;
    state.election_timer_sequence = INITIAL_ELECTION_TIMER_SEQUENCE;
    state.active_election_timer_ref = election_timer_ref(
        &state.profile.group_binding_ref,
        &state.node_id,
        state.profile.service_generation,
        state.current_term,
        state.election_timer_sequence,
    )?;
    validate_recovered_replica_state(&state)?;

    let mut effects = Vec::new();
    if let Some(snapshot) = &state.snapshot {
        effects.push(ReplicaEffect::RestoreApplicationSnapshot {
            snapshot: snapshot.clone(),
        });
    }
    if !replay_entries.is_empty() {
        effects.push(ReplicaEffect::ApplyCommitted {
            entries: replay_entries.clone(),
        });
    }
    effects.push(ReplicaEffect::ArmElectionTimer {
        timer_ref: state.active_election_timer_ref.clone(),
    });
    if effects.len() > state.profile.max_effects_per_step {
        return Err(MoltenError::invalid_harness("live Raft recovery exceeds the admitted effect bound"));
    }
    let recovery_ref = recovery_ref(&state, &durable_record_refs, snapshot_bytes)?;
    start_plan.state = state;
    start_plan.initial_effects = effects;
    start_plan.production_admitted = false;
    Ok(ReplicaRecoveryPlan {
        start_plan,
        recovery_ref,
        durable_commit_index,
        replay_entry_count: replay_entries.len(),
    })
}

fn reset_recovery_state(initial: &ReplicaState) -> Result<ReplicaState> {
    crate::preserves_rail::validate_content_ref(&initial.profile.group_binding_ref)?;
    let mut state = initial.clone();
    state.role = ReplicaRole::Follower;
    state.lifecycle = ReplicaLifecycle::Running;
    state.current_term = INITIAL_TERM;
    state.voted_for = None;
    state.leader_id = None;
    state.log.clear();
    state.commit_index = INITIAL_COMMIT_INDEX;
    state.last_applied = INITIAL_COMMIT_INDEX;
    state.snapshot = None;
    state.completed_requests.clear();
    state.pending_reads.clear();
    state.votes_received.clear();
    state.next_index.clear();
    state.match_index.clear();
    state.quorum_confirmed_term = None;
    Ok(state)
}

fn replay_record(state: &mut ReplicaState, durable_commit_index: &mut u64, bytes: &[u8]) -> Result<()> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let value = &decoded.value;
    if let Some(fields) = value.collect_simple_record("raft-hard-state-v1", Some(HARD_STATE_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let term = canonical::required_u64(&fields[0], "recovered hard-state term")?;
        if term < state.current_term {
            return Err(MoltenError::invalid_harness("live Raft recovery hard-state term regressed"));
        }
        state.current_term = term;
        state.voted_for = optional_string(&fields[1])?;
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-mutation-v1", Some(LOG_MUTATION_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let truncate_from = optional_u64(&fields[0])?;
        let entries = parse_entries(&fields[1])?;
        apply_log_mutation(state, *durable_commit_index, truncate_from, entries)?;
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-log-flush-v1", Some(SINGLE_INDEX_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let through_index = canonical::required_u64(&fields[0], "recovered flush boundary")?;
        if through_index > support_last_log_index(state) {
            return Err(MoltenError::invalid_harness("live Raft recovery flush boundary exceeds durable log"));
        }
        return Ok(());
    }
    if let Some(fields) = value.collect_simple_record("raft-commit-boundary-v1", Some(SINGLE_INDEX_ARITY)) {
        let fields = fields.iter().collect::<Vec<_>>();
        let through_index = canonical::required_u64(&fields[0], "recovered commit boundary")?;
        if through_index <= *durable_commit_index || through_index > support_last_log_index(state) {
            return Err(MoltenError::invalid_harness("live Raft recovery commit boundary is stale or beyond the log"));
        }
        *durable_commit_index = through_index;
        return Ok(());
    }
    Err(MoltenError::invalid_harness("live Raft recovery found an unsupported durable record"))
}

fn apply_log_mutation(
    state: &mut ReplicaState,
    durable_commit_index: u64,
    truncate_from: Option<u64>,
    entries: Vec<ReplicatedEntry>,
) -> Result<()> {
    if let Some(truncate_from) = truncate_from {
        if truncate_from <= durable_commit_index {
            return Err(MoltenError::invalid_harness("live Raft recovery attempted to truncate committed state"));
        }
        state.log.retain(|entry| entry.index < truncate_from);
    }
    let mut expected_index = support_last_log_index(state)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| MoltenError::invalid_harness("live Raft recovery log index overflow"))?;
    for entry in entries {
        if entry.index != expected_index || entry.term == INITIAL_TERM {
            return Err(MoltenError::invalid_harness("live Raft recovery log entries are noncontiguous"));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| MoltenError::invalid_harness("live Raft recovery log index overflow"))?;
        state.log.push(entry);
    }
    if state.log.len() > state.profile.max_log_entries {
        return Err(MoltenError::invalid_harness("live Raft recovery exceeds the admitted log bound"));
    }
    Ok(())
}

fn parse_entries(value: &Value<IOValue>) -> Result<Vec<ReplicatedEntry>> {
    let sequence = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("recovered Raft entries must be a sequence"))?;
    sequence.as_ref().as_slice().iter().map(canonical::parse_entry).collect()
}

fn optional_u64(value: &Value<IOValue>) -> Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = canonical::required_record(value, "some", OPTIONAL_VALUE_ARITY)?;
    Ok(Some(canonical::required_u64(&fields[0], "optional recovered index")?))
}

fn optional_string(value: &Value<IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = canonical::required_record(value, "some", OPTIONAL_VALUE_ARITY)?;
    Ok(Some(canonical::required_string(&fields[0], "optional recovered voter")?))
}

pub(super) fn parse_snapshot(bytes: &[u8]) -> Result<ReplicaSnapshot> {
    let decoded = crate::preserves_rail::strict_canonical_decode(bytes)?;
    let fields = canonical::required_record(&decoded.value, "raft-replica-snapshot-v1", SNAPSHOT_ARITY)?;
    let snapshot = ReplicaSnapshot {
        snapshot_ref: canonical::required_string(&fields[0], "recovered snapshot ref")?,
        group_binding_ref: canonical::required_string(&fields[1], "recovered snapshot group ref")?,
        membership_ref: canonical::required_string(&fields[2], "recovered snapshot membership ref")?,
        config_epoch: canonical::required_u64(&fields[3], "recovered snapshot config epoch")?,
        fencing_epoch: canonical::required_u64(&fields[4], "recovered snapshot fencing epoch")?,
        last_included_index: canonical::required_u64(&fields[5], "recovered snapshot index")?,
        last_included_term: canonical::required_u64(&fields[6], "recovered snapshot term")?,
        application_state_ref: canonical::required_string(&fields[7], "recovered snapshot application state ref")?,
        completed_requests: parse_completed_requests(&fields[8])?,
    };
    if snapshot.snapshot_ref != snapshot_ref(&snapshot)? {
        return Err(MoltenError::invalid_harness("live Raft recovered snapshot identity mismatch"));
    }
    Ok(snapshot)
}

fn parse_completed_requests(value: &Value<IOValue>) -> Result<std::collections::BTreeMap<String, u64>> {
    let sequence = value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("recovered completed requests must be a sequence"))?;
    let mut completed = std::collections::BTreeMap::new();
    for value in sequence.as_ref().as_slice() {
        let fields = canonical::required_record(value, "completed-request", COMPLETED_REQUEST_ARITY)?;
        let request_ref = canonical::required_string(&fields[0], "recovered completed request ref")?;
        let index = canonical::required_u64(&fields[1], "recovered completed request index")?;
        crate::preserves_rail::validate_content_ref(&request_ref)?;
        if completed.insert(request_ref, index).is_some() {
            return Err(MoltenError::invalid_harness("recovered snapshot contains duplicate completed request"));
        }
    }
    Ok(completed)
}

fn install_recovery_snapshot(
    state: &mut ReplicaState,
    durable_commit_index: u64,
    snapshot: Option<ReplicaSnapshot>,
) -> Result<()> {
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.group_binding_ref != state.profile.group_binding_ref
        || snapshot.membership_ref != state.membership.membership_ref
        || snapshot.config_epoch != state.membership.config_epoch
        || snapshot.fencing_epoch != state.profile.fencing_epoch
        || snapshot.last_included_index > durable_commit_index
    {
        return Err(MoltenError::invalid_harness("live Raft recovered snapshot binding or boundary mismatch"));
    }
    for index in snapshot.completed_requests.values() {
        if *index == INITIAL_COMMIT_INDEX || *index > snapshot.last_included_index {
            return Err(MoltenError::invalid_harness("recovered snapshot completed request index is out of range"));
        }
    }
    state.completed_requests.clone_from(&snapshot.completed_requests);
    state.log.retain(|entry| entry.index > snapshot.last_included_index);
    state.snapshot = Some(snapshot);
    Ok(())
}

fn support_last_log_index(state: &ReplicaState) -> u64 {
    state.log.last().map_or_else(
        || state.snapshot.as_ref().map_or(INITIAL_COMMIT_INDEX, |snapshot| snapshot.last_included_index),
        |entry| entry.index,
    )
}

fn recovery_ref(state: &ReplicaState, durable_record_refs: &[String], snapshot_bytes: Option<&[u8]>) -> Result<String> {
    let snapshot_content_ref = snapshot_bytes.map(crate::preserves_rail::content_ref_from_bytes);
    let snapshot_value = snapshot_content_ref.as_deref().map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |reference| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(reference)]),
    );
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("raft-recovery-plan-v1", vec![
        crate::preserves_rail::string(&state.profile.group_binding_ref),
        crate::preserves_rail::string(&state.node_id),
        crate::preserves_rail::u64_value(state.profile.service_generation),
        crate::preserves_rail::u64_value(state.current_term),
        crate::preserves_rail::u64_value(state.commit_index),
        crate::preserves_rail::sequence(durable_record_refs.iter().map(crate::preserves_rail::string).collect()),
        snapshot_value,
    ]))
}
