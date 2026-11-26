//! Bounded register profile and fully acknowledged history shared by the core regressions.

/// Test canonical-reference port: BLAKE3 over the JSON rendering. The production port is the
/// shell's canonical Preserves identity; the core only binds whatever the port returns.
pub(super) fn content_ref(profile: &crate::profile::Register) -> Result<alloc::string::String, crate::profile::Issue> {
    let bytes = serde_json::to_vec(profile).map_err(|_| crate::profile::Issue::Encoding)?;
    Ok(alloc::format!("blake3:{}", blake3::hash(&bytes).to_hex()))
}

/// Test encoded-size port matching the shell's `serde_json` publication encoding.
pub(super) fn encoded_len(history: &crate::history::Document) -> Result<u64, crate::history::Issue> {
    serde_json::to_vec(history)
        .map(|bytes| bytes.len() as u64)
        .map_err(|_| crate::history::Issue::Bounds)
}

pub(super) fn validate(
    input: &crate::profile::Admitted,
    plan: &crate::generator::OperationPlan,
    history: &crate::history::Document,
) -> Result<(), crate::history::Issue> {
    crate::history::validate(input, plan, history, encoded_len)
}

pub(super) fn register() -> crate::profile::Register {
    let reference = alloc::format!("blake3:{}", "a".repeat(64));
    crate::profile::Register {
        schema: "molten.live-register-profile.v1".into(),
        group: "g".into(),
        service: "registry".into(),
        keys: alloc::vec!["k".into()],
        initial_value: None,
        values: alloc::vec![-1, 0, 7],
        generator_ref: reference.clone(),
        adapter_ref: reference.clone(),
        model_ref: reference.clone(),
        outcome_ref: reference,
        seed: 0,
        read_weight: 1,
        write_weight: 1,
        clients: alloc::vec!["a".into(), "b".into()],
        endpoints: alloc::vec!["a:7180".into(), "b:7180".into()],
        concurrency: 2,
        operations: 12,
        control_operations: 3,
        consistency_model: "register".into(),
        recovery: crate::profile::RecoveryBounds {
            timeout_ms: 100,
            stable_membership_observations: 2,
            require_all_endpoint_reads: true,
        },
        evidence: crate::profile::EvidenceBounds {
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

pub(super) fn admitted() -> crate::profile::Admitted {
    crate::profile::admit(&register(), content_ref).expect("bounded profile")
}

pub(super) fn observation(outcome: crate::history::PublicOutcome) -> crate::history::AttemptObservation {
    crate::history::AttemptObservation {
        ordinal: 0,
        event_index: 0,
        started_ns: 10,
        completed_ns: 20,
        source_artifact: alloc::format!("blake3:{}", "b".repeat(64)),
        outcome,
    }
}

pub(super) fn history(input: &crate::profile::Admitted) -> crate::history::Document {
    let plan = crate::generator::generate(input).expect("admitted plan");
    let mut events = alloc::vec::Vec::with_capacity(plan.operations.len() * 2);
    for operation in &plan.operations {
        let value = match operation.function {
            crate::generator::Function::Read => Some(crate::generator::Value::Null),
            crate::generator::Function::Write => None,
        };
        let mut observed = observation(crate::history::PublicOutcome::Acknowledged(value));
        observed.event_index = events.len() as u64;
        observed.started_ns = events.len() as u64 * 10;
        observed.completed_ns = observed.started_ns + 1;
        events.extend(crate::history::project_attempt(input, operation, &observed).expect("pair"));
    }
    let evidence = &input.profile().evidence;
    crate::history::Document {
        schema_version: 2,
        workload_profile: input.profile_ref().into(),
        source_artifact: observation(crate::history::PublicOutcome::Pending).source_artifact,
        profile: crate::history::Profile {
            profile_id: input.profile_ref().into(),
            model: "register".into(),
            independent_key_decomposition: false,
            bounds: crate::history::CheckerBounds {
                max_operations: 36,
                max_depth: 36,
                max_states: evidence.checker_states,
                max_branches: evidence.checker_branches,
                max_memo_bytes: evidence.checker_memo_bytes,
                max_reductions: evidence.checker_reductions,
            },
        },
        completeness: crate::history::Completeness {
            invocations: 12,
            completions: 12,
            pending: 0,
            applied_faults: 0,
            observed_faults: 0,
        },
        events,
    }
}
