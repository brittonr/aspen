use preserves::ValueImpl;

use super::*;

const HARD_STATE_ARITY: usize = 2;
const LOG_MUTATION_ARITY: usize = 2;
const SINGLE_INDEX_ARITY: usize = 1;
const SNAPSHOT_ARITY: usize = 9;
const COMPLETED_REQUEST_ARITY: usize = 2;
const OPTIONAL_VALUE_ARITY: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaPlan {
    pub start_plan: ReplicaStartPlan,
    pub recovery_ref: String,
    pub durable_commit_index: u64,
    pub replay_entry_count: usize,
}

pub fn plan_replica(
    mut start_plan: ReplicaStartPlan,
    durable_records: &[crate::fabric_durability::LogRecord],
    snapshot_bytes: Option<&[u8]>,
    snapshot_covered_sequence: Option<u64>,
) -> crate::error::Result<ReplicaPlan> {
    let mut state = reset_state(&start_plan.state)?;
    let snapshot = snapshot_bytes.map(parse_snapshot).transpose()?;
    if snapshot.is_none() && snapshot_covered_sequence.is_some() {
        return Err(crate::error::Failure::invalid_harness("snapshot coverage requires a snapshot"));
    }
    let (durable_commit_index, durable_record_refs) = replay_durable_records(
        &mut state, durable_records, snapshot, snapshot_covered_sequence,
    )?;
    if durable_commit_index > support_last_log_index(&state) {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft recovery commit boundary exceeds durable log",
        ));
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
    finish_recovered_state(&mut state, durable_commit_index)?;
    let effects = initial_effects(&state, &replay_entries)?;
    let recovery_ref = r#ref(&state, &durable_record_refs, snapshot_bytes)?;
    start_plan.state = state;
    start_plan.initial_effects = effects;
    start_plan.production_admitted = false;
    Ok(ReplicaPlan {
        start_plan,
        recovery_ref,
        durable_commit_index,
        replay_entry_count: replay_entries.len(),
    })
}

/// Replays the gap-free, content-verified durable records into `state`, returning the durable
/// commit index and the replayed record refs.
fn replay_durable_records(
    state: &mut ReplicaState,
    durable_records: &[crate::fabric_durability::LogRecord],
    snapshot: Option<ReplicaSnapshot>,
    snapshot_covered_sequence: Option<u64>,
) -> crate::error::Result<(u64, Vec<String>)> {
    let mut expected_sequence = 0_u64;
    let mut durable_commit_index = INITIAL_COMMIT_INDEX;
    let mut durable_record_refs = Vec::with_capacity(durable_records.len());
    let snapshot_boundary = snapshot.as_ref().map(|snapshot| (snapshot.last_included_index, snapshot.last_included_term));
    replay_snapshot(state, &mut durable_commit_index, snapshot)?;
    let mut boundary_term = None;
    let mut is_suffix_retained = None;
    let mut is_coverage_seen = snapshot_covered_sequence.is_none();
    for record in durable_records {
        if record.sequence != expected_sequence {
            return Err(crate::error::Failure::invalid_harness("live Raft recovery durable sequence has a gap"));
        }
        expected_sequence = expected_sequence.checked_add(NEXT_LOG_INDEX_STEP).ok_or_else(|| {
            crate::error::Failure::invalid_harness("live Raft recovery durable sequence overflow")
        })?;
        if crate::preserves_rail::content_ref_from_bytes(&record.value) != record.value_ref {
            return Err(crate::error::Failure::invalid_harness("live Raft recovery durable record content ref mismatch"));
        }
        if snapshot_covered_sequence.is_some_and(|covered| record.sequence <= covered) {
            replay_snapshot_prefix(state, &mut durable_commit_index, &record.value,
                &mut boundary_term, &mut is_suffix_retained)?;
        } else {
            replay_record(state, &mut durable_commit_index, &record.value)?;
        }
        if Some(record.sequence) == snapshot_covered_sequence {
            is_coverage_seen = true;
            if snapshot_boundary.is_some_and(|(_, term)| !is_suffix_retained.unwrap_or(Some(term) == boundary_term)) {
                if snapshot_boundary.is_some_and(|(index, _)| durable_commit_index > index) {
                    return Err(crate::error::Failure::invalid_harness("snapshot conflicts with a committed suffix"));
                }
                state.log.clear();
            }
        }
        durable_record_refs.push(record.value_ref.clone());
    }
    if !is_coverage_seen {
        return Err(crate::error::Failure::invalid_harness("snapshot coverage exceeds durable records"));
    }
    Ok((durable_commit_index, durable_record_refs))
}

fn reset_state(initial: &ReplicaState) -> crate::error::Result<ReplicaState> {
    crate::preserves_rail::validate_content_ref(&initial.profile.group_binding_ref)?;
    let mut state = initial.clone();
    state.role = ReplicaRole::Follower;
    state.lifecycle = ReplicaLifecycle::Running;
    state.rejoin = ReplicaRejoin::Voter;
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


fn apply_log_mutation(
    state: &mut ReplicaState,
    durable_commit_index: u64,
    truncate_from: Option<u64>,
    entries: Vec<ReplicatedEntry>,
) -> crate::error::Result<()> {
    if let Some(truncate_from) = truncate_from {
        if truncate_from <= durable_commit_index {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft recovery attempted to truncate committed state",
            ));
        }
        state.log.retain(|entry| entry.index < truncate_from);
    }
    let mut expected_index = support_last_log_index(state)
        .checked_add(NEXT_LOG_INDEX_STEP)
        .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft recovery log index overflow"))?;
    for entry in entries {
        if entry.index != expected_index || entry.term == INITIAL_TERM {
            return Err(crate::error::Failure::invalid_harness("live Raft recovery log entries are noncontiguous"));
        }
        for reference in [&entry.request_ref, &entry.command_ref, &entry.command_schema_ref] {
            crate::preserves_rail::validate_content_ref(reference)?;
        }
        expected_index = expected_index
            .checked_add(NEXT_LOG_INDEX_STEP)
            .ok_or_else(|| crate::error::Failure::invalid_harness("live Raft recovery log index overflow"))?;
        state.log.push(entry);
    }
    if state.log.len() > state.profile.max_log_entries {
        return Err(crate::error::Failure::invalid_harness("live Raft recovery exceeds the admitted log bound"));
    }
    Ok(())
}

fn parse_entries(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Vec<ReplicatedEntry>> {
    let sequence = value
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovered Raft entries must be a sequence"))?;
    sequence.as_ref().as_slice().iter().map(canonical::parse_entry).collect()
}

fn optional_u64(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<u64>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = canonical::required_record(value, "some", OPTIONAL_VALUE_ARITY)?;
    Ok(Some(canonical::required_u64(&fields[0], "optional recovered index")?))
}

fn optional_string(value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let fields = canonical::required_record(value, "some", OPTIONAL_VALUE_ARITY)?;
    Ok(Some(canonical::required_string(&fields[0], "optional recovered voter")?))
}

pub(super) fn parse_snapshot(bytes: &[u8]) -> crate::error::Result<ReplicaSnapshot> {
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
        return Err(crate::error::Failure::invalid_harness("live Raft recovered snapshot identity mismatch"));
    }
    Ok(snapshot)
}

fn parse_completed_requests(
    value: &preserves::Value<preserves::IOValue>,
) -> crate::error::Result<std::collections::BTreeMap<String, u64>> {
    let sequence = value
        .collect_sequence()
        .ok_or_else(|| crate::error::Failure::invalid_harness("recovered completed requests must be a sequence"))?;
    let mut completed = std::collections::BTreeMap::new();
    for value in sequence.as_ref().as_slice() {
        let fields = canonical::required_record(value, "completed-request", COMPLETED_REQUEST_ARITY)?;
        let request_ref = canonical::required_string(&fields[0], "recovered completed request ref")?;
        let index = canonical::required_u64(&fields[1], "recovered completed request index")?;
        crate::preserves_rail::validate_content_ref(&request_ref)?;
        if completed.insert(request_ref, index).is_some() {
            return Err(crate::error::Failure::invalid_harness(
                "recovered snapshot contains duplicate completed request",
            ));
        }
    }
    Ok(completed)
}
