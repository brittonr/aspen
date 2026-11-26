const COORDINATION_CLOCK_TRANSITION_SCHEMA: &str = "molten.coordination.clock-transition.v1";
const COORDINATION_CLOCK_NAMESPACE: &str = "coordination:clock";
const COORDINATION_CLOCK_REGISTRY_NAME: &str = "time-basis";
const COORDINATION_CLOCK_SESSION: &str = "coordination:clock-transition";

/// A committed change of coordination time basis (a clock epoch change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinationClockTransition {
    pub transition_ref: String,
    /// The retiring basis and its ticks at the shared reading (the last committed sample if the retiring clock
    /// no longer reads, as after a restart).
    pub from: CoordinationTimeSample,
    /// The new basis and its ticks at that same reading: the start of the new basis.
    pub to: CoordinationTimeSample,
    pub before_state_ref: String,
    pub after_state_ref: String,
    pub raft_commit_ref: String,
    pub value: IoValue,
    pub evidence_values: Vec<IoValue>,
}

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// Node capability, not a request operation: the node shell recreates the runtime's live coordination clock
/// (for example after the monotonic clock source is reopened) and commits the basis change. No coordination
/// request, payload, or CLI command reaches it. The caller supplies no clock: the runtime opens a fresh
/// sealed live clock itself, so no frozen or arbitrary clock can be installed.
///
/// Leases are re-anchored with their remaining TTL, translated through one shared reading of the node monotonic
/// source: every check runs first, then a single instant is converted into both the retiring and the new
/// basis, and each lease's new expiry is `new_now + (old_expiry - old_now)`, saturating at zero. No time is
/// lost between the two readings, so an expired lease stays expired and repeated recreations, whatever
/// authority ref accompanies them, never extend a lease. Fencing state is preserved. The committed transition
/// record carries both readings, so replaying it reproduces the re-anchored state.
pub fn recreate_coordination_live_clock(
    runtime: &mut CoordinationRuntime,
    authority_refs: &[String],
) -> Result<CoordinationClockTransition> {
    transition_coordination_clock(runtime, CoordinationClock::live()?, authority_refs)
}

/// Installs an admitted live clock: it must be a sealed live clock whose basis ref matches its provenance,
/// and the runtime's current clock must be live. Simulation runtimes keep their deterministic clock.
fn transition_coordination_clock(
    runtime: &mut CoordinationRuntime,
    clock: CoordinationClock,
    authority_refs: &[String],
) -> Result<CoordinationClockTransition> {
    clock.verify_live_provenance()?;
    if !runtime.clock.is_live() {
        return Err(MoltenError::invalid_harness(
            "coordination simulation runtimes keep their deterministic clock; only live runtimes change clock basis",
        ));
    }
    commit_clock_transition(runtime, clock, authority_refs, live_shared_reading)
}

/// The retiring and the new basis's ticks at one reading of a shared instant source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SharedClockReading {
    retiring_ticks: u64,
    next_ticks: u64,
}

/// Converts one node monotonic reading into both live bases. If the retiring clock no longer reads on the
/// committed basis (as after a restart), the retiring side is credited only up to the last committed sample.
/// That conservative gap can keep a lease live for the unobserved time between that sample and the restart,
/// but it never expires a lease early.
fn live_shared_reading(
    retiring: &CoordinationClock,
    state: &CoordinationState,
    next: &CoordinationClock,
) -> Result<SharedClockReading> {
    let ClockSource::Live(next_live) = &next.source else {
        return Err(MoltenError::invalid_harness("coordination clock transitions translate only between live clocks"));
    };
    let instant = read_node_monotonic_instant();
    let next_ticks = next_live.ticks_at(instant).map_err(MoltenError::from)?;
    let retiring_ticks = match &retiring.source {
        ClockSource::Live(live) if retiring.time_basis_ref == state.time_basis_ref => {
            live.ticks_at(instant).map_err(MoltenError::from)?
        }
        _ => state.time_ticks,
    };
    Ok(SharedClockReading { retiring_ticks, next_ticks })
}

/// Commits a clock basis transition whose clock the caller has already admitted. All checks run before
/// `read_shared` takes the single reading that both bases are translated through.
fn commit_clock_transition(
    runtime: &mut CoordinationRuntime,
    clock: CoordinationClock,
    authority_refs: &[String],
    read_shared: impl FnOnce(&CoordinationClock, &CoordinationState, &CoordinationClock) -> Result<SharedClockReading>,
) -> Result<CoordinationClockTransition> {
    if clock.time_basis_ref() == runtime.state.time_basis_ref {
        return Err(MoltenError::invalid_harness(
            "coordination clock is already on the committed time basis; a clock transition needs a new basis",
        ));
    }
    if authority_refs.is_empty() {
        return Err(MoltenError::invalid_harness("coordination clock transition requires authority evidence"));
    }
    validate_refs(authority_refs, "coordination clock transition authority ref")?;
    let reading = read_shared(&runtime.clock, &runtime.state, &clock)?;
    if reading.retiring_ticks < runtime.state.time_ticks {
        return Err(MoltenError::invalid_harness(format!(
            "coordination retiring clock reading {} regresses below committed time {}",
            reading.retiring_ticks, runtime.state.time_ticks
        )));
    }
    let from = CoordinationTimeSample {
        time_basis_ref: runtime.state.time_basis_ref.clone(),
        ticks: reading.retiring_ticks,
    };
    let to = CoordinationTimeSample {
        time_basis_ref: clock.time_basis_ref().to_string(),
        ticks: reading.next_ticks,
    };
    let after_state = rebase_coordination_time(&runtime.state, &from, &to)?;
    let before_snapshot = snapshot_from_state(&runtime.state)?;
    let after_snapshot = snapshot_from_state(&after_state)?;
    let value = coordination_clock_transition_value(&from, &to, &before_snapshot.state_ref, &after_snapshot.state_ref)?;
    let transition_ref = canonical_hash(&value)?;
    let proposal = propose_clock_transition(runtime, &transition_ref, &before_snapshot.state_ref, authority_refs)?;
    if proposal.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "control-plane commit denied for coordination clock transition: {}",
            proposal.registry_receipt.diagnostics.join("; ")
        )));
    }
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    runtime.state = after_state;
    runtime.clock = clock;
    let mut evidence_values = vec![
        proposal.envelope.value.clone(),
        proposal.commit_receipt.value.clone(),
        proposal.registry_receipt.value.clone(),
    ];
    if let Some(log_entry) = &proposal.log_entry {
        evidence_values.push(log_entry.value.clone());
    }
    evidence_values.push(value.clone());
    evidence_values.push(after_snapshot.value.clone());
    Ok(CoordinationClockTransition {
        transition_ref,
        from,
        to,
        before_state_ref: before_snapshot.state_ref,
        after_state_ref: after_snapshot.state_ref,
        raft_commit_ref: proposal.commit_receipt.receipt_ref,
        value,
        evidence_values,
    })
}

fn propose_clock_transition(
    runtime: &mut CoordinationRuntime,
    transition_ref: &str,
    before_state_ref: &str,
    authority_refs: &[String],
) -> Result<Proposal> {
    let command = crate::raft_control_plane::control_registry_command_value(
        &crate::raft_control_plane::ControlRegistryCommandInput {
            operation: "set-coordination-state".to_string(),
            namespace: COORDINATION_CLOCK_NAMESPACE.to_string(),
            name: COORDINATION_CLOCK_REGISTRY_NAME.to_string(),
            target_ref: Some(transition_ref.to_string()),
        },
    )?;
    let envelope =
        crate::raft_control_plane::raft_command_envelope_value(&crate::raft_control_plane::RaftCommandEnvelopeInput {
            group_ref: runtime.raft.manifest.manifest_ref.clone(),
            client_session: COORDINATION_CLOCK_SESSION.to_string(),
            sequence: runtime.next_sequence,
            command,
            authority_refs: authority_refs.to_vec(),
            policy_refs: runtime.manifest.policy_refs.clone(),
            resource_refs: runtime.manifest.resource_refs.clone(),
            evidence_refs: vec![before_state_ref.to_string(), transition_ref.to_string()],
        })?;
    crate::raft_control_plane::propose_control_registry_command(&mut runtime.raft, &envelope)
}

fn coordination_clock_transition_value(
    from: &CoordinationTimeSample,
    to: &CoordinationTimeSample,
    before_state_ref: &str,
    after_state_ref: &str,
) -> Result<IoValue> {
    validate_ref(&from.time_basis_ref, "coordination clock transition from basis ref")?;
    validate_ref(&to.time_basis_ref, "coordination clock transition to basis ref")?;
    validate_ref(before_state_ref, "coordination clock transition before state ref")?;
    validate_ref(after_state_ref, "coordination clock transition after state ref")?;
    Ok(record("coordination-clock-transition-v1", vec![
        string(COORDINATION_CLOCK_TRANSITION_SCHEMA),
        record("from", vec![string(&from.time_basis_ref), u64_value(from.ticks)]),
        record("to", vec![string(&to.time_basis_ref), u64_value(to.ticks)]),
        record("before-state", vec![string(before_state_ref)]),
        record("after-state", vec![string(after_state_ref)]),
        checks_value(&[
            ("leases-reanchored-remaining-ttl", "pass"),
            ("fencing-state-preserved", "pass"),
        ]),
    ]))
}

/// Pure core of a clock epoch change: moves committed time to the new basis start `to` and re-anchors each
/// lease to expire its remaining TTL after that start. The remaining TTL is the old expiry minus `from.ticks`,
/// the latest observation of the retiring basis, saturating at zero.
fn rebase_coordination_time(
    state: &CoordinationState,
    from: &CoordinationTimeSample,
    to: &CoordinationTimeSample,
) -> Result<CoordinationState> {
    validate_ref(&to.time_basis_ref, "coordination clock transition basis ref")?;
    if to.ticks > MAX_COORDINATION_TIME_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination time exhausted at {}; no lease granted there would be representable",
            to.ticks
        )));
    }
    let mut next = state.clone();
    for lock in next.locks.values_mut() {
        lock.lease_expires_at = reanchored_expiry(lock.lease_expires_at, from.ticks, to.ticks)?;
    }
    for election in next.elections.values_mut() {
        election.lease_expires_at = reanchored_expiry(election.lease_expires_at, from.ticks, to.ticks)?;
    }
    next.time_basis_ref = to.time_basis_ref.clone();
    next.time_ticks = to.ticks;
    Ok(next)
}

fn reanchored_expiry(expires_at: u64, observed_old_ticks: u64, new_start: u64) -> Result<u64> {
    new_start
        .checked_add(expires_at.saturating_sub(observed_old_ticks))
        .ok_or_else(|| MoltenError::invalid_harness("coordination lease re-anchoring overflows clock ticks"))
}
