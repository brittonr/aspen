
fn validate_simulation_input(input: &ConsensusSimulationInput) -> Result<()> {
    validate_non_empty(&input.scenario, "consensus simulation scenario")?;
    validate_algorithm_name(&input.algorithm_profile)?;
    require_ref(&input.topology_ref, "consensus simulation topology ref")?;
    validate_refs(&input.membership_refs, "consensus simulation membership ref")?;
    require_ref(&input.fault_plan_ref, "consensus simulation fault-plan ref")?;
    validate_refs(&input.operation_ids, "consensus simulation operation id ref")?;
    validate_refs(&input.required_evidence_refs, "consensus simulation required evidence ref")?;
    if let Some(reference) = &input.proposer_ref {
        require_ref(reference, "consensus simulation proposer ref")?;
    }
    if let Some(reference) = &input.placement_ref {
        require_ref(reference, "consensus simulation placement ref")?;
    }
    validate_read_consistency_mode(&input.requested_read_consistency)?;
    match input.scenario.as_str() {
        SCENARIO_MAJORITY_PROGRESS
        | SCENARIO_MINORITY_DENIAL
        | SCENARIO_STALE_READ_CLASSIFICATION
        | SCENARIO_LEADERLESS_NON_LEADER_PROGRESS
        | SCENARIO_LEADERLESS_MISSING_EVIDENCE
        | SCENARIO_CONCURRENT_PROPOSAL_RESOLUTION
        | SCENARIO_UNSAFE_PLACEMENT => Ok(()),
        value => Err(MoltenError::invalid_harness(format!("unsupported consensus simulation scenario {value}"))),
    }
}

fn simulation_diagnostics(input: &ConsensusSimulationInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let quorum = majority_quorum_count(input.membership_refs.len())?;
    let has_quorum = input.connected_replicas >= quorum;
    match input.scenario.as_str() {
        SCENARIO_MAJORITY_PROGRESS => require_quorum(has_quorum, quorum, &mut diagnostics),
        SCENARIO_MINORITY_DENIAL => {
            if has_quorum {
                diagnostics.push("minority-denial scenario unexpectedly has majority reachability".to_string());
            }
        }
        SCENARIO_STALE_READ_CLASSIFICATION => stale_read_diagnostics(input, &mut diagnostics),
        SCENARIO_LEADERLESS_NON_LEADER_PROGRESS => {
            require_leaderless_experimental(input, &mut diagnostics);
            require_quorum(has_quorum, quorum, &mut diagnostics);
            if input.proposer_ref.is_none() {
                diagnostics.push("leaderless scenario requires proposer ref".to_string());
            }
        }
        SCENARIO_LEADERLESS_MISSING_EVIDENCE => {
            if input.algorithm_profile != CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL {
                diagnostics.push("missing-evidence scenario must use leaderless experimental profile".to_string());
            }
            if has_experimental_evidence(input) {
                diagnostics.push("missing-evidence scenario unexpectedly has all experimental evidence".to_string());
            }
        }
        SCENARIO_CONCURRENT_PROPOSAL_RESOLUTION => {
            require_quorum(has_quorum, quorum, &mut diagnostics);
            if input.operation_ids.is_empty() {
                diagnostics.push("concurrent proposal simulation requires operation ids".to_string());
            }
        }
        SCENARIO_UNSAFE_PLACEMENT => {
            if input.placement_ref.is_some() {
                diagnostics.push("unsafe-placement scenario unexpectedly has placement evidence".to_string());
            }
        }
        _ => diagnostics.push("unsupported consensus simulation scenario".to_string()),
    }
    Ok(diagnostics)
}

fn stale_read_diagnostics(input: &ConsensusSimulationInput, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if input.requested_read_consistency == READ_CONSISTENCY_LINEARIZABLE && !input.local_state_fresh {
        diagnostics.push_item("linearizable read denied without freshness evidence".to_string());
    }
}

fn require_leaderless_experimental(input: &ConsensusSimulationInput, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if input.algorithm_profile != CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL {
        diagnostics.push_item("scenario requires leaderless experimental profile".to_string());
    }
    if !has_experimental_evidence(input) {
        diagnostics.push_item("leaderless experimental profile missing required evidence".to_string());
    }
}

fn require_quorum(has_quorum: bool, quorum: usize, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if !has_quorum {
        diagnostics.push_item(format!("missing majority quorum of {quorum} replicas"));
    }
}

fn has_experimental_evidence(input: &ConsensusSimulationInput) -> bool {
    !input.required_evidence_refs.is_empty() && input.placement_ref.is_some()
}

fn consensus_simulation_receipt_value(
    input: &ConsensusSimulationInput,
    decision: &str,
    final_state_ref: Option<&str>,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-simulation-receipt-v1", vec![
        string(CONSENSUS_SIMULATION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("scenario", vec![string(&input.scenario)]),
        record("algorithm-profile", vec![string(&input.algorithm_profile)]),
        record("topology", vec![string(&input.topology_ref)]),
        record("membership", vec![strings_sequence(&input.membership_refs)]),
        record("fault-plan", vec![string(&input.fault_plan_ref)]),
        record("operations", vec![strings_sequence(&input.operation_ids)]),
        record("connected-replicas", vec![u64_value(usize_to_u64(input.connected_replicas)?)]),
        record("read-consistency", vec![string(&input.requested_read_consistency)]),
        record("final-state", vec![optional_ref_value(final_state_ref)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("deterministic-scheduler", "pass"),
            ("majority-quorum", decision),
            ("read-consistency-classified", "pass"),
            ("experimental-profile-gated", if input.algorithm_profile == CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL { "pass" } else { "diagnostic" }),
        ]),
    ]))
}

fn simulation_final_state_ref(input: &ConsensusSimulationInput) -> Result<String> {
    let mut operations = input.operation_ids.clone();
    operations.sort();
    canonical_hash(&record("consensus-simulation-final-state-v1", vec![
        string(&input.scenario),
        string(&input.algorithm_profile),
        strings_sequence(&input.membership_refs),
        strings_sequence(&operations),
        string(&input.fault_plan_ref),
    ]))
}

fn validate_algorithm_name(value: &str) -> Result<()> {
    match value {
        CONSENSUS_PROFILE_RAFT | CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported consensus algorithm profile {value}"))),
    }
}

fn majority_quorum_count(member_count: usize) -> Result<usize> {
    member_count
        .checked_div(MAJORITY_QUORUM_DIVISOR)
        .and_then(|value| value.checked_add(MAJORITY_QUORUM_OFFSET))
        .ok_or_else(|| MoltenError::invalid_harness("consensus majority quorum overflow"))
}

fn usize_to_u64(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| MoltenError::invalid_harness("consensus count overflow"))
}

fn validate_diagnostic_strings(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RAFT_DIAGNOSTICS, label)?;
    for value in values {
        validate_non_empty(value, label)?;
    }
    Ok(())
}
