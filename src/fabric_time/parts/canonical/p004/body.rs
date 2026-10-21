
// r[impl molten.audit_f11.freshness]
pub fn runnable_key_value(key: &super::RunnableKey) -> crate::error::Result<preserves::IOValue> {
    super::validate_runnable_key(key).map_err(|error| validation_error("runnable occurrence", &[error]))?;
    Ok(crate::preserves_rail::record(FABRIC_RUNNABLE_RECORD, vec![
        field("service-id", crate::preserves_rail::string(&key.service_id)),
        field("generation", crate::preserves_rail::u64_value(key.generation)),
        field("runnable-id", crate::preserves_rail::string(&key.runnable_id)),
        field("occurrence-sequence", crate::preserves_rail::u64_value(key.occurrence_sequence)),
    ]))
}

/// Decode only the occurrence-fenced cohort. Legacy names never imply fresh issuance.
// r[impl molten.audit_f11.freshness]
pub fn parse_runnable_key(value: &preserves::IOValue) -> crate::error::Result<super::RunnableKey> {
    const FIELD_COUNT: usize = 4;
    const RUNNABLE_INDEX: usize = 2;
    const OCCURRENCE_INDEX: usize = 3;
    let fields = value.collect_simple_record(FABRIC_RUNNABLE_RECORD, Some(FIELD_COUNT))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected occurrence-fenced runnable v2 with four fields"))?;
    let key = super::RunnableKey {
        service_id: record_string_field(&fields[0], "service-id")?,
        generation: record_u64_field(&fields[1], "generation")?,
        runnable_id: record_string_field(&fields[RUNNABLE_INDEX], "runnable-id")?,
        occurrence_sequence: record_u64_field(&fields[OCCURRENCE_INDEX], "occurrence-sequence")?,
    };
    super::validate_runnable_key(&key).map_err(|error| validation_error("runnable occurrence", &[error]))?;
    Ok(key)
}

fn scheduler_state_value(state: &super::SchedulerState) -> crate::error::Result<preserves::IOValue> {
    let slots = state.runnables.iter().map(|slot| {
        let Some(runnable) = slot else {
            return Ok(crate::preserves_rail::record("vacant-runnable-v2", Vec::new()));
        };
        let phase = match runnable.phase {
            super::RunnablePhase::Ready => "ready",
            super::RunnablePhase::Running => "running",
            super::RunnablePhase::Blocked => "blocked",
            super::RunnablePhase::Completed => "completed",
            super::RunnablePhase::Cancelled => "cancelled",
        };
        Ok(crate::preserves_rail::record("occupied-runnable-v2", vec![
            field("occurrence", runnable_key_value(&runnable.key)?),
            field("priority", preserves::IOValue::new(i64::from(runnable.priority))),
            field("enqueue-sequence", crate::preserves_rail::u64_value(runnable.enqueue_sequence)),
            field("wait-turns", crate::preserves_rail::u64_value(runnable.wait_turns)),
            field("phase", crate::preserves_rail::string(phase)),
            field("terminal-sequence", optional_u64(runnable.terminal_sequence)),
        ]))
    }).collect::<crate::error::Result<Vec<_>>>()?;
    Ok(crate::preserves_rail::record("fabric-time-scheduler-state-v2", vec![
        field("profile-contract-ref", crate::preserves_rail::string(&state.profile_ref)),
        field("generation", crate::preserves_rail::u64_value(state.generation)),
        field("terminal-retention-limit", crate::preserves_rail::u64_value(state.terminal_retention_limit)),
        field("highest-occurrence-sequence", crate::preserves_rail::u64_value(state.highest_occurrence_sequence)),
        field("next-enqueue-sequence", crate::preserves_rail::u64_value(state.next_enqueue_sequence)),
        field("choice-sequence", crate::preserves_rail::u64_value(state.choice_sequence)),
        field("next-terminal-sequence", crate::preserves_rail::u64_value(state.next_terminal_sequence)),
        field("slots", crate::preserves_rail::sequence(slots)),
    ]))
}
