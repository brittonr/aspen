
fn collect_engine_admission_diagnostics(
    descriptor: &ConsensusEngineDescriptor,
    input: &ConsensusEngineAdmissionInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    if !descriptor.enabled {
        diagnostics.push_item(format!("consensus engine {} is disabled", descriptor.profile_id));
    }
    // r[impl molten.consensus.algorithm_profile_manifest]
    // r[impl molten.fabric_consistency.production_admission]
    if input.requested_environment == CONSENSUS_ENVIRONMENT_PRODUCTION
        && descriptor.production_admission_status != PRODUCTION_STATUS_ADMITTED
    {
        diagnostics.push_item(format!(
            "consensus engine {} is not admitted for production runtime; status {}",
            descriptor.profile_id, descriptor.production_admission_status
        ));
    }
    if descriptor.required_evidence_refs.is_empty() {
        diagnostics.push_item(format!("consensus engine {} missing proof/model evidence", descriptor.profile_id));
    }
    if descriptor.conformance_receipt_refs.is_empty() {
        diagnostics.push_item(format!("consensus engine {} missing conformance refs", descriptor.profile_id));
    }
    if !descriptor
        .supported_read_consistency_modes
        .iter()
        .any(|mode| mode == &input.requested_read_consistency)
    {
        diagnostics.push_item(format!(
            "unsupported read consistency mode {} for consensus engine {}",
            input.requested_read_consistency, descriptor.profile_id
        ));
    }
    for capability in &input.required_capabilities {
        if !descriptor.capabilities.iter().any(|value| value == capability) {
            diagnostics.push_item(format!(
                "unsupported consensus engine capability {capability} for {}",
                descriptor.profile_id
            ));
        }
    }
    ensure_count_at_most(input.required_capabilities.len(), MAX_RAFT_REFS, "consensus engine required capabilities")
}

fn consensus_engine_admission_receipt_value(
    input: &ConsensusEngineAdmissionInput,
    descriptor: Option<&ConsensusEngineDescriptor>,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_read_consistency_mode(&input.requested_read_consistency)?;
    validate_string_items(&input.required_capabilities, "consensus engine required capability")?;
    Ok(record("consensus-engine-admission-receipt-v1", vec![
        string(CONSENSUS_ENGINE_ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&input.algorithm_profile)]),
        record("version", vec![string(&input.profile_version)]),
        record("environment", vec![string(&input.requested_environment)]),
        record("read-consistency", vec![string(&input.requested_read_consistency)]),
        record("capabilities", vec![strings_sequence(&input.required_capabilities)]),
        record("descriptor", vec![optional_ref_value(descriptor.map(|entry| entry.descriptor_ref.as_str()))]),
        record("implementation", vec![string(descriptor.map_or("none", |entry| entry.implementation_id.as_str()))]),
        record("evidence", vec![strings_sequence(descriptor.map_or(&[][..], |entry| entry.required_evidence_refs.as_slice()))]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("engine-registry-resolved", decision),
            ("production-admission-policy", decision),
            ("conformance-evidence-bound", if descriptor.is_some_and(|entry| !entry.conformance_receipt_refs.is_empty()) { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY }),
        ]),
    ]))
}

pub fn parse_consensus_engine_admission_receipt(value: &IoValue) -> Result<ConsensusEngineAdmissionReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-admission-receipt-v1", Some(CONSENSUS_ENGINE_ADMISSION_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-admission-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_ADMISSION_RECEIPT_SCHEMA, "consensus engine admission schema")?;
    require_check(&parse_checks(&fields[11])?, "engine-registry-resolved", "consensus engine admission")?;
    Ok(ConsensusEngineAdmissionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        descriptor: None,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn resolve_control_registry_engine(manifest: &RaftGroupManifest) -> Result<ConsensusEngineAdmissionReceipt> {
    resolve_control_registry_engine_for_environment(manifest, CONSENSUS_ENVIRONMENT_PRODUCTION)
}

pub fn resolve_control_registry_model_engine(
    manifest: &RaftGroupManifest,
) -> Result<ConsensusEngineAdmissionReceipt> {
    resolve_control_registry_engine_for_environment(manifest, CONSENSUS_ENVIRONMENT_MODEL)
}

fn resolve_control_registry_engine_for_environment(
    manifest: &RaftGroupManifest,
    requested_environment: &str,
) -> Result<ConsensusEngineAdmissionReceipt> {
    let registry = default_consensus_engine_registry()?;
    admit_consensus_engine(&registry, &ConsensusEngineAdmissionInput {
        algorithm_profile: manifest.algorithm_profile.clone(),
        profile_version: manifest.admitted_profile_version.clone(),
        requested_environment: requested_environment.to_string(),
        requested_read_consistency: READ_CONSISTENCY_LINEARIZABLE.to_string(),
        required_capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL.to_string(),
            ENGINE_CAPABILITY_LINEARIZABLE_READ.to_string(),
            ENGINE_CAPABILITY_SNAPSHOT.to_string(),
            ENGINE_CAPABILITY_RECOVERY.to_string(),
        ],
    })
}

pub fn consensus_engine_readback_summary(descriptor: &ConsensusEngineDescriptor) -> String {
    format!(
        "consensus-engine profile={} version={} implementation={} enabled={} production={} capabilities={} currentness={} conformance={} caveats={}",
        descriptor.profile_id,
        descriptor.profile_version,
        descriptor.implementation_id,
        descriptor.enabled,
        descriptor.production_admission_status,
        descriptor.capabilities.join(","),
        descriptor.currentness_evidence_classes.join(","),
        descriptor.conformance_receipt_refs.len(),
        descriptor.caveats.join(",")
    )
}

// r[impl molten.consensus.engine_interface]
// r[impl molten.consensus.engine_portable_state]
pub fn normalized_raft_commit_receipt_value(receipt: &RaftCommitReceipt, engine_epoch: u64) -> Result<IoValue> {
    let state_ref = receipt.log_entry_ref.as_deref();
    consensus_engine_receipt_value(&ConsensusEngineReceiptValueInput {
        receipt_kind: NORMALIZED_RECEIPT_KIND_COMMIT,
        decision: &receipt.decision,
        engine_profile: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        engine_epoch,
        group_ref: &receipt.group_ref,
        operation_ref: &receipt.command_ref,
        state_ref,
        currentness_ref: receipt.log_entry_ref.as_deref(),
        source_receipt_ref: Some(&receipt.receipt_ref),
        evidence_refs: receipt.log_entry_ref.as_slice(),
        diagnostics: &[],
    })
}

pub fn normalized_raft_read_receipt_value(receipt: &RaftReadReceipt, engine_epoch: u64) -> Result<IoValue> {
    let read_fields = read_receipt_fields(&receipt.value)?;
    consensus_engine_receipt_value(&ConsensusEngineReceiptValueInput {
        receipt_kind: NORMALIZED_RECEIPT_KIND_READ,
        decision: &receipt.decision,
        engine_profile: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        engine_epoch,
        group_ref: &read_fields.group_ref,
        operation_ref: receipt.target_ref.as_deref().unwrap_or(&read_fields.state_ref),
        state_ref: Some(&read_fields.state_ref),
        currentness_ref: Some(&receipt.receipt_ref),
        source_receipt_ref: Some(&receipt.receipt_ref),
        evidence_refs: read_fields.read_index_predicate_ref.as_slice(),
        diagnostics: &receipt.diagnostics,
    })
}

struct ReadReceiptFields {
    group_ref: String,
    state_ref: String,
    read_index_predicate_ref: Option<String>,
}

fn read_receipt_fields(value: &IoValue) -> Result<ReadReceiptFields> {
    let fields = value
        .collect_simple_record("raft-read-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <raft-read-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::RAFT_READ_RECEIPT_SCHEMA, "raft read receipt schema")?;
    Ok(ReadReceiptFields {
        group_ref: record_ref(&fields[2], "group")?,
        state_ref: record_ref(&fields[3], "state")?,
        read_index_predicate_ref: record_optional_ref(&fields[10], "read-index-predicate")?,
    })
}

struct ConsensusEngineReceiptValueInput<'a> {
    receipt_kind: &'a str,
    decision: &'a str,
    engine_profile: &'a str,
    profile_version: &'a str,
    engine_epoch: u64,
    group_ref: &'a str,
    operation_ref: &'a str,
    state_ref: Option<&'a str>,
    currentness_ref: Option<&'a str>,
    source_receipt_ref: Option<&'a str>,
    evidence_refs: &'a [String],
    diagnostics: &'a [String],
}

fn consensus_engine_receipt_value(input: &ConsensusEngineReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_algorithm_name(input.engine_profile)?;
    validate_non_empty(input.profile_version, "consensus engine receipt profile version")?;
    validate_non_empty(input.receipt_kind, "consensus engine receipt kind")?;
    require_ref(input.group_ref, "consensus engine receipt group ref")?;
    require_ref(input.operation_ref, "consensus engine receipt operation ref")?;
    if let Some(reference) = input.state_ref {
        require_ref(reference, "consensus engine receipt state ref")?;
    }
    if let Some(reference) = input.currentness_ref {
        require_ref(reference, "consensus engine receipt currentness ref")?;
    }
    if let Some(reference) = input.source_receipt_ref {
        require_ref(reference, "consensus engine source receipt ref")?;
    }
    validate_refs(input.evidence_refs, "consensus engine receipt evidence ref")?;
    validate_diagnostic_strings(input.diagnostics, "consensus engine receipt diagnostic")?;
    Ok(record("consensus-engine-receipt-v1", vec![
        string(CONSENSUS_ENGINE_RECEIPT_SCHEMA),
        record("kind", vec![string(input.receipt_kind)]),
        record("decision", vec![string(input.decision)]),
        record("engine-profile", vec![string(input.engine_profile)]),
        record("profile-version", vec![string(input.profile_version)]),
        record("engine-epoch", vec![u64_value(input.engine_epoch)]),
        record("group", vec![string(input.group_ref)]),
        record("operation", vec![string(input.operation_ref)]),
        record("state", vec![optional_ref_value(input.state_ref)]),
        record("currentness", vec![optional_ref_value(input.currentness_ref)]),
        record("source-receipt", vec![optional_ref_value(input.source_receipt_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
    ]))
}

pub fn parse_consensus_engine_receipt(value: &IoValue) -> Result<ConsensusEngineReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-receipt-v1", Some(CONSENSUS_ENGINE_RECEIPT_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_RECEIPT_SCHEMA, "consensus engine receipt schema")?;
    let receipt_kind = record_string(&fields[1], "kind")?;
    let decision = record_string(&fields[2], "decision")?;
    validate_decision(&decision)?;
    Ok(ConsensusEngineReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        receipt_kind,
        engine_profile: record_string(&fields[3], "engine-profile")?,
        profile_version: record_string(&fields[4], "profile-version")?,
        engine_epoch: record_u64(&fields[5], "engine-epoch")?,
        state_ref: record_optional_ref(&fields[8], "state")?,
        source_receipt_ref: record_optional_ref(&fields[10], "source-receipt")?,
        diagnostics: parse_string_sequence(&fields[12], "diagnostics")?,
        value: value.clone(),
    })
}

// r[impl molten.consensus.engine_switchover_receipts]
// r[impl molten.testing.consensus_switchover_fixtures]
pub fn consensus_engine_switchover_receipt(
    input: &ConsensusEngineSwitchoverInput,
) -> Result<ConsensusEngineSwitchoverReceipt> {
    consensus_engine_switchover_receipt_for_environment(input, CONSENSUS_ENVIRONMENT_PRODUCTION)
}

pub fn consensus_engine_model_switchover_receipt(
    input: &ConsensusEngineSwitchoverInput,
) -> Result<ConsensusEngineSwitchoverReceipt> {
    consensus_engine_switchover_receipt_for_environment(input, CONSENSUS_ENVIRONMENT_MODEL)
}
