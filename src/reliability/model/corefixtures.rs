//! Bounded register profile and fully acknowledged history shared by the core regressions.

pub(super) fn register() -> crate::live_reliability::profile::Register {
    let reference = format!("blake3:{}", "a".repeat(64));
    crate::live_reliability::profile::Register {
        schema: "molten.live-register-profile.v1".into(),
        group: "g".into(),
        service: "registry".into(),
        keys: vec!["k".into()],
        initial_value: None,
        values: vec![-1, 0, 7],
        generator_ref: reference.clone(),
        adapter_ref: reference.clone(),
        model_ref: reference.clone(),
        outcome_ref: reference,
        seed: 0,
        read_weight: 1,
        write_weight: 1,
        clients: vec!["a".into(), "b".into()],
        endpoints: vec!["a:7180".into(), "b:7180".into()],
        concurrency: 2,
        operations: 12,
        control_operations: 3,
        consistency_model: "register".into(),
        recovery: crate::live_reliability::profile::RecoveryBounds {
            timeout_ms: 100,
            stable_membership_observations: 2,
            require_all_endpoint_reads: true,
        },
        evidence: crate::live_reliability::profile::EvidenceBounds {
            max_attempts_per_operation: 3,
            max_events: 72,
            max_uncertain: 10,
            max_bytes: 100_000,
            checker_states: 50_000,
            checker_branches: 100_000,
            checker_memo_bytes: 8_388_608,
            checker_reductions: 256,
        },
    }
}

pub(super) fn admitted() -> crate::live_reliability::profile::Admitted {
    crate::live_reliability::profile::admit(&register()).expect("bounded profile")
}

pub(super) fn observation(
    outcome: crate::live_reliability::history::PublicOutcome,
) -> crate::live_reliability::history::AttemptObservation {
    crate::live_reliability::history::AttemptObservation {
        ordinal: 0,
        event_index: 0,
        started_ns: 10,
        completed_ns: 20,
        source_artifact: format!("blake3:{}", "b".repeat(64)),
        outcome,
    }
}

pub(super) fn history(
    input: &crate::live_reliability::profile::Admitted,
) -> crate::live_reliability::history::Document {
    let plan = crate::live_reliability::generator::generate(input).expect("admitted plan");
    let mut events = Vec::with_capacity(plan.operations.len() * 2);
    for operation in &plan.operations {
        let value = match operation.function {
            crate::live_reliability::generator::Function::Read => Some(crate::live_reliability::generator::Value::Null),
            crate::live_reliability::generator::Function::Write => None,
        };
        let mut observed = observation(crate::live_reliability::history::PublicOutcome::Acknowledged(value));
        observed.event_index = events.len() as u64;
        observed.started_ns = events.len() as u64 * 10;
        observed.completed_ns = observed.started_ns + 1;
        events.extend(crate::live_reliability::history::project_attempt(input, operation, &observed).expect("pair"));
    }
    let evidence = &input.profile().evidence;
    crate::live_reliability::history::Document {
        schema_version: 2,
        workload_profile: input.profile_ref().into(),
        source_artifact: observation(crate::live_reliability::history::PublicOutcome::Pending).source_artifact,
        profile: crate::live_reliability::history::Profile {
            profile_id: input.profile_ref().into(),
            model: "register".into(),
            independent_key_decomposition: false,
            bounds: crate::live_reliability::history::CheckerBounds {
                max_operations: 36,
                max_depth: 36,
                max_states: evidence.checker_states,
                max_branches: evidence.checker_branches,
                max_memo_bytes: evidence.checker_memo_bytes,
                max_reductions: evidence.checker_reductions,
            },
        },
        completeness: crate::live_reliability::history::Completeness {
            invocations: 12,
            completions: 12,
            pending: 0,
            applied_faults: 0,
            observed_faults: 0,
        },
        events,
    }
}
