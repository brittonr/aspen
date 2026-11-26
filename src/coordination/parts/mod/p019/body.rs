const COORDINATION_LIVE_TIME_PROFILE_ID: &str = "molten.coordination.lease-clock";
/// The lease clock admits one monotonic reading at a time and no timers, runnables, or entropy of its own.
const COORDINATION_LIVE_TIME_UNIT_BOUND: u64 = 1;

fn coordination_live_time_profile() -> crate::fabric_time::TimeProfileDescriptor {
    crate::fabric_time::TimeProfileDescriptor {
        schema: crate::fabric_time::FABRIC_TIME_PROFILE_SCHEMA.to_string(),
        profile_id: COORDINATION_LIVE_TIME_PROFILE_ID.to_string(),
        profile_ref: content_ref_from_bytes(COORDINATION_LIVE_TIME_PROFILE_ID.as_bytes()),
        kind: crate::fabric_time::TimeProfileKind::Live,
        supported_domains: crate::fabric_time::REQUIRED_TIME_DOMAINS.to_vec(),
        max_duration_ticks: MAX_COORDINATION_LEASE_TICKS,
        max_uncertainty_ticks: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_timers: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_runnables: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_entropy_request_bytes: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_entropy_total_bytes: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_scheduler_concurrency: COORDINATION_LIVE_TIME_UNIT_BOUND,
        max_scheduler_queue_depth: COORDINATION_LIVE_TIME_UNIT_BOUND,
        fairness_bound_turns: None,
        scheduler_policy: crate::fabric_time::SchedulerPolicy {
            ordering: crate::fabric_time::SchedulerOrdering::PriorityThenFifo,
            replay: crate::fabric_time::SchedulerReplayPolicy::RecordedChoiceRequired,
            overload: crate::fabric_time::SchedulerOverloadPolicy::Reject,
        },
        evidence_mode: crate::fabric_time::TimeEvidenceMode::SelectedSemanticBoundaries,
        non_claims: crate::fabric_time::REQUIRED_TIME_NON_CLAIMS.to_vec(),
    }
}

/// Admits a clock sample for the next mutation of `state`: it must come from the committed time basis,
/// may not regress, and may not pass [`MAX_COORDINATION_TIME_TICKS`], so every lease expiry stays
/// representable.
fn admit_time_sample(state: &CoordinationState, sample: &CoordinationTimeSample) -> Result<u64> {
    if sample.time_basis_ref != state.time_basis_ref {
        return Err(MoltenError::invalid_harness(format!(
            "coordination clock basis {} does not match committed time basis {}",
            sample.time_basis_ref, state.time_basis_ref
        )));
    }
    if sample.ticks < state.time_ticks {
        return Err(MoltenError::invalid_harness(format!(
            "coordination clock sample {} regresses below committed time {}",
            sample.ticks, state.time_ticks
        )));
    }
    if sample.ticks > MAX_COORDINATION_TIME_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination time exhausted at {}; no lease granted there would be representable",
            sample.ticks
        )));
    }
    Ok(sample.ticks)
}

/// Lease expiry for a lock acquire or election granted at clock reading `now`. Reads an optional
/// `<lease ttl>`; without one the lease lasts [`DEFAULT_COORDINATION_LEASE_TICKS`].
fn lease_expiry(payload: Option<&IoValue>, now: u64) -> Result<u64> {
    let ttl = match payload {
        Some(value) => {
            let fields = simple_record(value, "lease", 1)?;
            required_u64(&fields[0], "coordination lease ttl")?
        }
        None => DEFAULT_COORDINATION_LEASE_TICKS,
    };
    if ttl == 0 || ttl > MAX_COORDINATION_LEASE_TICKS {
        return Err(MoltenError::invalid_harness(format!(
            "coordination lease ttl {ttl} must be within 1..={MAX_COORDINATION_LEASE_TICKS} clock ticks"
        )));
    }
    now.checked_add(ttl)
        .ok_or_else(|| MoltenError::invalid_harness("coordination lease expiry overflows clock ticks"))
}
