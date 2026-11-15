use super::*;

mod evidence;

pub(super) fn collect(
    input: &crate::fabric_simulation::bounded_client::Input,
    directory: &std::path::Path,
    executed_requests: Vec<String>,
) -> crate::error::Result<crate::fabric_simulation::bounded_client::History> {
    let input_ref = crate::fabric_simulation::bounded_client::input_ref(input)?;
    let mut records = Vec::with_capacity(input.participants.len());
    for participant in &input.participants {
        let sample_path = directory.join(format!("{participant}-client-checkpoint.preserves"));
        if !sample_path.try_exists().map_err(crate::error::Failure::from)? {
            continue;
        }
        records.push(observe_participant(input, directory, participant)?);
    }
    evidence::validate_owner_cohort(&records)?;
    let participants = records.into_iter().map(|(participant, _owner)| participant).collect();
    let is_applied = input.participants[1..]
        .iter()
        .all(|participant| directory.join(format!("{participant}-client-fault-applied.preserves")).is_file());
    Ok(crate::fabric_simulation::bounded_client::History {
        input_ref,
        executed_requests,
        fault: if is_applied {
            crate::fabric_simulation::bounded_client::FaultApplication::Applied
        } else {
            crate::fabric_simulation::bounded_client::FaultApplication::Requested
        },
        participants,
        deployed_control: input.control,
        is_control_reached: evidence::reached(input, directory)?,
    })
}

fn observe_participant(
    input: &crate::fabric_simulation::bounded_client::Input,
    directory: &std::path::Path,
    participant: &str,
) -> crate::error::Result<(crate::fabric_simulation::bounded_client::Participant, Observation)> {
    let sample = read_value(&directory.join(format!("{participant}-client-checkpoint.preserves")))?;
    let fields = crate::preserves_rail::simple_record_fields(&sample, "molten-client-native-sample-v1", 10)?;
    let input_ref = crate::fabric_simulation::bounded_client::input_ref(input)?;
    let owner = read_value(&directory.join(format!("{participant}-client-checkpoint-owner.preserves")))?;
    let active = parse(&owner)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&owner)?;
    let sample_request = canonical::required_string(&fields[2], "application request")?;
    let is_bound =
        evidence::is_checkpoint_bound(&fields, &input_ref, participant, &receipt_ref, &sample_request, &active)?;
    let terminal_path = path(directory, participant);
    let terminal = if terminal_path.try_exists().map_err(crate::error::Failure::from)? {
        Some(parse(&read_value(&terminal_path)?)?)
    } else {
        None
    };
    let recovery_path = directory.join(format!("{participant}-client-reopen.preserves"));
    let (recovery, reopen_term) = if recovery_path.try_exists().map_err(crate::error::Failure::from)? {
        let (recovery, term) = evidence::local_recovery(directory, participant, &input_ref)?;
        (recovery, Some(term))
    } else {
        (crate::fabric_simulation::bounded_client::RecoverySource::Unavailable, None)
    };
    let (ticks, start_term, end_term, is_fault_off) =
        observed_horizon(directory, participant, input, reopen_term, terminal.as_ref().map(|terminal| terminal.term))?;
    let health = evidence::parse_health(&fields[5])?;
    let health = if let Some(terminal) = &terminal {
        evidence::terminal_health(directory, participant, &input_ref, terminal, health)?
    } else {
        health
    };
    let is_terminal_bound =
        terminal.as_ref().is_none_or(|terminal| evidence::is_terminal_bound(terminal, &active, participant));
    let participant = crate::fabric_simulation::bounded_client::Participant {
        participant: participant.to_string(),
        receipt_ref,
        request_ref: sample_request,
        is_receipt_bound: is_bound && is_terminal_bound,
        is_ready: active.process_id > 0,
        has_membership: active
            .admitted_voters
            .iter()
            .map(String::as_str)
            .eq(crate::fabric_simulation::bounded_client::CLIENT_PARTICIPANTS),
        acknowledged: active.request_completed,
        application_count: crate::fabric_simulation::bounded_client::unsigned_integer(&fields[3])?,
        snapshot_restored: active.application_restored,
        recovered_acknowledgement: terminal.as_ref().is_some_and(|terminal| terminal.request_completed),
        // Native readback observes visibility. It does not claim machine-loss
        // synchronization merely because a process crash/reopen succeeded.
        persistence: (active.durable_record_count > 0 || active.durable_snapshot_count > 0)
            .then_some(crate::fabric_durability::PersistenceObservation::AppliedProcessVisible),
        requested_durability: crate::fabric_durability::DurabilityLevel::ProcessLoss,
        health_before: health,
        recovery,
        start_term,
        end_term,
        observed_ticks: ticks,
        is_fault_off,
    };
    Ok((participant, active))
}

pub(super) fn observed_horizon(
    directory: &std::path::Path,
    participant: &str,
    input: &crate::fabric_simulation::bounded_client::Input,
    reopen_term: Option<u64>,
    terminal_term: Option<u64>,
) -> crate::error::Result<(u64, u64, Option<u64>, bool)> {
    let horizon_path = directory.join(format!("{participant}-client-horizon.preserves"));
    if !horizon_path.try_exists().map_err(crate::error::Failure::from)? {
        return Ok((0, reopen_term.unwrap_or(0), None, false));
    }
    let value = read_value(&horizon_path)?;
    let fields = crate::preserves_rail::simple_record_fields(&value, "molten-client-native-horizon-v2", 8)?;
    if canonical::required_string(&fields[0], "horizon input")?
        != crate::fabric_simulation::bounded_client::input_ref(input)?
        || canonical::required_string(&fields[1], "horizon participant")? != participant
    {
        return Err(crate::error::Failure::invalid_harness("participant horizon binding mismatch"));
    }
    let start_term = crate::fabric_simulation::bounded_client::unsigned_integer(&fields[2])?;
    let first_step = crate::fabric_simulation::bounded_client::unsigned_integer(&fields[3])?;
    let last_step = crate::fabric_simulation::bounded_client::unsigned_integer(&fields[4])?;
    let first_term = crate::fabric_simulation::bounded_client::unsigned_integer(&fields[5])?;
    let last_term = crate::fabric_simulation::bounded_client::unsigned_integer(&fields[6])?;
    let is_fault_off = canonical::required_bool(&fields[7], "fault off throughout interval")?;
    // A terminal election cannot retroactively stabilize an earlier interval.
    // The marker must cover the requested duration after advancement from the
    // independently observed reopen term, inside the admitted loop budget.
    if reopen_term != Some(start_term)
        || first_term <= start_term
        || last_term < first_term
        || last_step.checked_sub(first_step) != Some(input.recovery_ticks)
        || last_step >= input.max_event_loop_iterations
        || terminal_term.is_some_and(|term| term < last_term)
        || !is_fault_off
    {
        return Err(crate::error::Failure::invalid_harness("participant recovery interval is not stabilized"));
    }
    Ok((input.recovery_ticks, start_term, terminal_term.map(|_| last_term), is_fault_off))
}
