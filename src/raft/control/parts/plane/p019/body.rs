
fn consensus_engine_switchover_receipt_for_environment(
    input: &ConsensusEngineSwitchoverInput,
    requested_environment: &str,
) -> Result<ConsensusEngineSwitchoverReceipt> {
    validate_switchover_input(input)?;
    let registry = default_consensus_engine_registry()?;
    let target_admission = admit_consensus_engine(&registry, &ConsensusEngineAdmissionInput {
        algorithm_profile: input.target_profile.clone(),
        profile_version: input.target_version.clone(),
        requested_environment: requested_environment.to_string(),
        requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        required_capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL.to_string(),
            ENGINE_CAPABILITY_LINEARIZABLE_READ.to_string(),
            ENGINE_CAPABILITY_SWITCHOVER.to_string(),
        ],
    })?;
    let mut diagnostics = switchover_diagnostics(input, &target_admission)?;
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus switchover diagnostics")?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_switchover_receipt_value(input, decision, &diagnostics, &target_admission.receipt_ref)?;
    Ok(ConsensusEngineSwitchoverReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        source_profile: input.source_profile.clone(),
        target_profile: input.target_profile.clone(),
        target_engine_epoch: input.target_engine_epoch,
        diagnostics: std::mem::take(&mut diagnostics),
        value,
    })
}

fn validate_switchover_input(input: &ConsensusEngineSwitchoverInput) -> Result<()> {
    validate_algorithm_name(&input.source_profile)?;
    validate_algorithm_name(&input.target_profile)?;
    validate_non_empty(&input.source_version, "source consensus profile version")?;
    validate_non_empty(&input.target_version, "target consensus profile version")?;
    require_ref(&input.source_state_ref, "switchover source state ref")?;
    require_ref(&input.target_bootstrap_state_ref, "switchover target bootstrap state ref")?;
    validate_refs(&input.membership_refs, "switchover membership ref")?;
    validate_refs(&input.placement_refs, "switchover placement ref")?;
    validate_refs(&input.replay_conformance_refs, "switchover replay conformance ref")?;
    validate_refs(&input.currentness_evidence_refs, "switchover currentness ref")?;
    validate_refs(&input.operator_approval_refs, "switchover operator approval ref")?;
    validate_non_empty(&input.rollback_posture, "switchover rollback posture")
}

fn switchover_diagnostics(
    input: &ConsensusEngineSwitchoverInput,
    target_admission: &ConsensusEngineAdmissionReceipt,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.source_profile == input.target_profile && input.source_version == input.target_version {
        diagnostics.push("switchover target must differ from source profile/version".to_string());
    }
    if input.target_engine_epoch <= input.active_engine_epoch {
        diagnostics.push(format!(
            "target engine epoch {} must advance active epoch {}",
            input.target_engine_epoch, input.active_engine_epoch
        ));
    }
    if input.membership_refs.is_empty() {
        diagnostics.push("switchover requires membership/config refs".to_string());
    }
    if input.placement_refs.is_empty() {
        diagnostics.push("switchover requires placement refs".to_string());
    }
    if input.replay_conformance_refs.is_empty() {
        diagnostics.push("switchover requires replay/conformance evidence".to_string());
    }
    if input.currentness_evidence_refs.is_empty() {
        diagnostics.push("switchover requires current source-state evidence".to_string());
    }
    if input.operator_approval_refs.is_empty() {
        diagnostics.push("switchover requires operator approval refs".to_string());
    }
    if !SUPPORTED_SWITCHOVER_ROLLBACK_POSTURES.contains(&input.rollback_posture.as_str()) {
        diagnostics.push(format!("unsupported switchover rollback posture {}", input.rollback_posture));
    }
    if target_admission.decision != ENGINE_DECISION_PASS {
        diagnostics.push(format!("target engine admission denied: {}", target_admission.diagnostics.join(";")));
    }
    Ok(diagnostics)
}

fn consensus_engine_switchover_receipt_value(
    input: &ConsensusEngineSwitchoverInput,
    decision: &str,
    diagnostics: &[String],
    target_admission_ref: &str,
) -> Result<IoValue> {
    Ok(record("consensus-engine-switchover-receipt-v1", vec![
        string(CONSENSUS_ENGINE_SWITCHOVER_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("source", vec![string(&input.source_profile), string(&input.source_version)]),
        record("target", vec![string(&input.target_profile), string(&input.target_version)]),
        record("active-epoch", vec![u64_value(input.active_engine_epoch)]),
        record("target-epoch", vec![u64_value(input.target_engine_epoch)]),
        record("source-state", vec![string(&input.source_state_ref)]),
        record("target-bootstrap", vec![string(&input.target_bootstrap_state_ref)]),
        record("membership", vec![strings_sequence(&input.membership_refs)]),
        record("placement", vec![strings_sequence(&input.placement_refs)]),
        record("replay-conformance", vec![strings_sequence(&input.replay_conformance_refs)]),
        record("currentness", vec![strings_sequence(&input.currentness_evidence_refs)]),
        record("operator-approval", vec![strings_sequence(&input.operator_approval_refs)]),
        record("rollback", vec![string(&input.rollback_posture)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("target-engine-admitted", decision),
            ("fencing-epoch-advanced", decision),
            ("replay-conformance-bound", if input.replay_conformance_refs.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
            ("rollback-posture-declared", ENGINE_DECISION_PASS),
            ("target-admission-receipt", if target_admission_ref.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_switchover_receipt(value: &IoValue) -> Result<ConsensusEngineSwitchoverReceipt> {
    let fields = value
        .collect_simple_record(
            "consensus-engine-switchover-receipt-v1",
            Some(CONSENSUS_ENGINE_SWITCHOVER_FIELD_COUNT),
        )
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-switchover-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_SWITCHOVER_RECEIPT_SCHEMA, "consensus switchover schema")?;
    require_check(&parse_checks(&fields[15])?, "fencing-epoch-advanced", "consensus switchover receipt")?;
    let source_fields = value_to_iovalue(&fields[2]);
    let source = source_fields
        .collect_simple_record("source", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected consensus switchover source"))?;
    let target_fields = value_to_iovalue(&fields[3]);
    let target = target_fields
        .collect_simple_record("target", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("expected consensus switchover target"))?;
    Ok(ConsensusEngineSwitchoverReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        source_profile: required_string(&source[0], "source profile")?,
        target_profile: required_string(&target[0], "target profile")?,
        target_engine_epoch: record_u64(&fields[5], "target-epoch")?,
        diagnostics: parse_string_sequence(&fields[14], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.consensus.engine_switchover_fencing]
pub fn consensus_engine_epoch_gate(input: &ConsensusEngineEpochGateInput) -> Result<ConsensusEngineEpochGateReceipt> {
    validate_epoch_gate_input(input)?;
    let diagnostics = epoch_gate_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_epoch_gate_value(input, decision, &diagnostics)?;
    Ok(ConsensusEngineEpochGateReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        diagnostics,
        value,
    })
}

fn validate_epoch_gate_input(input: &ConsensusEngineEpochGateInput) -> Result<()> {
    validate_non_empty(&input.operation, "consensus engine epoch gate operation")?;
    validate_algorithm_name(&input.active_profile)?;
    validate_algorithm_name(&input.presented_profile)?;
    if let Some(reference) = &input.activation_receipt_ref {
        require_ref(reference, "engine activation receipt ref")?;
    }
    Ok(())
}

fn epoch_gate_diagnostics(input: &ConsensusEngineEpochGateInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.presented_profile != input.active_profile {
        diagnostics.push(format!(
            "inactive consensus engine profile {}; active profile {}",
            input.presented_profile, input.active_profile
        ));
    }
    if input.presented_engine_epoch < input.active_engine_epoch {
        diagnostics.push(format!(
            "stale engine epoch {}; active epoch {}",
            input.presented_engine_epoch, input.active_engine_epoch
        ));
    }
    if input.presented_engine_epoch > input.active_engine_epoch && input.activation_receipt_ref.is_none() {
        diagnostics.push(format!(
            "target engine epoch {} is not activated by a committed switchover receipt",
            input.presented_engine_epoch
        ));
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus engine epoch diagnostics")?;
    Ok(diagnostics)
}

fn consensus_engine_epoch_gate_value(
    input: &ConsensusEngineEpochGateInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-engine-epoch-gate-v1", vec![
        string(CONSENSUS_ENGINE_EPOCH_GATE_SCHEMA),
        record("decision", vec![string(decision)]),
        record("operation", vec![string(&input.operation)]),
        record("active-profile", vec![string(&input.active_profile)]),
        record("active-epoch", vec![u64_value(input.active_engine_epoch)]),
        record("presented-profile", vec![string(&input.presented_profile)]),
        record("presented-epoch", vec![u64_value(input.presented_engine_epoch)]),
        record("activation", vec![optional_ref_value(input.activation_receipt_ref.as_deref())]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("active-engine-epoch-bound", decision),
            ("stale-writer-fenced", if diagnostics.iter().any(|value| value.contains("stale engine epoch")) { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
            ("target-read-activation", if diagnostics.iter().any(|value| value.contains("not activated")) { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_epoch_gate(value: &IoValue) -> Result<ConsensusEngineEpochGateReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-epoch-gate-v1", Some(CONSENSUS_ENGINE_EPOCH_GATE_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-epoch-gate-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_EPOCH_GATE_SCHEMA, "consensus engine epoch gate schema")?;
    require_check(&parse_checks(&fields[9])?, "active-engine-epoch-bound", "consensus engine epoch gate")?;
    Ok(ConsensusEngineEpochGateReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        diagnostics: parse_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.testing.consensus_engine_conformance]
pub fn consensus_engine_conformance_receipt(
    input: &ConsensusEngineConformanceInput,
) -> Result<ConsensusEngineConformanceReceipt> {
    validate_conformance_input(input)?;
    let diagnostics = conformance_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_conformance_receipt_value(input, decision, &diagnostics)?;
    Ok(ConsensusEngineConformanceReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        fixture_id: input.fixture_id.clone(),
        diagnostics,
        value,
    })
}

fn validate_conformance_input(input: &ConsensusEngineConformanceInput) -> Result<()> {
    validate_algorithm_name(&input.algorithm_profile)?;
    validate_non_empty(&input.profile_version, "consensus conformance profile version")?;
    validate_non_empty(&input.fixture_id, "consensus conformance fixture id")?;
    validate_string_items(&input.passed_cases, "consensus conformance case")?;
    require_ref(&input.expected_state_ref, "consensus conformance expected state ref")?;
    require_ref(&input.actual_state_ref, "consensus conformance actual state ref")?;
    validate_refs(&input.normalized_receipt_refs, "consensus conformance normalized receipt ref")
}

fn conformance_diagnostics(input: &ConsensusEngineConformanceInput) -> Result<Vec<String>> {
    let mut diagnostics = required_conformance_cases()
        .iter()
        .filter(|required| !input.passed_cases.iter().any(|value| value == *required))
        .map(|required| format!("missing consensus engine conformance case {required}"))
        .collect::<Vec<_>>();
    if input.expected_state_ref != input.actual_state_ref {
        diagnostics.push(format!(
            "consensus engine replay state mismatch expected {} actual {}",
            input.expected_state_ref, input.actual_state_ref
        ));
    }
    if input.normalized_receipt_refs.is_empty() {
        diagnostics.push("consensus engine conformance requires normalized receipt refs".to_string());
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus conformance diagnostics")?;
    Ok(diagnostics)
}

fn required_conformance_cases() -> &'static [&'static str] {
    &[
        CONFORMANCE_CASE_PROPOSAL,
        CONFORMANCE_CASE_DUPLICATE_DENIAL,
        CONFORMANCE_CASE_LINEARIZABLE_READ,
        CONFORMANCE_CASE_LOCAL_STALE_READ,
        CONFORMANCE_CASE_SNAPSHOT_RECOVERY,
        CONFORMANCE_CASE_MEMBERSHIP_DENIAL,
        CONFORMANCE_CASE_CANONICAL_REPLAY,
        CONFORMANCE_CASE_NORMALIZED_RECEIPT,
    ]
}
