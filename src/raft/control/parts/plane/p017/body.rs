
// r[impl molten.fabric_consistency.production_admission]
pub fn default_raft_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: CONSENSUS_PROFILE_RAFT,
        profile_version: CONSENSUS_PROFILE_VERSION_RAFT,
        implementation_id: "in-process-raft-control-registry-v1",
        enabled: true,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE, READ_CONSISTENCY_LOCAL_STALE],
        capabilities: vec![
            ENGINE_CAPABILITY_PROPOSAL,
            ENGINE_CAPABILITY_LINEARIZABLE_READ,
            ENGINE_CAPABILITY_LOCAL_STALE_READ,
            ENGINE_CAPABILITY_SNAPSHOT,
            ENGINE_CAPABILITY_RECOVERY,
            ENGINE_CAPABILITY_MEMBERSHIP_CONFIG,
            ENGINE_CAPABILITY_PLACEMENT_VALIDATION,
            ENGINE_CAPABILITY_READBACK_SUMMARY,
            ENGINE_CAPABILITY_SWITCHOVER,
        ],
        currentness_evidence_classes: vec![CURRENTNESS_CLASS_RAFT_QUORUM_COMMIT, CURRENTNESS_CLASS_READ_INDEX],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_JOINT_CONSENSUS, MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: PRODUCTION_STATUS_MODEL_ONLY,
        required_evidence_refs: vec![
            synthetic_ref("raft-pure-transition-model-evidence")?,
            synthetic_ref("raft-deterministic-simulation-evidence")?,
            synthetic_ref("raft-placement-model-evidence")?,
            synthetic_ref("raft-membership-model-evidence")?,
        ],
        conformance_receipt_refs: vec![synthetic_ref("raft-model-conformance-receipt")?],
        caveats: {
            let mut caveats = default_consensus_caveats();
            caveats.push("in-process-model-does-not-prove-live-quorum".to_string());
            caveats
        },
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

fn leaderless_experimental_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: CONSENSUS_PROFILE_LEADERLESS_EXPERIMENTAL,
        profile_version: CONSENSUS_PROFILE_VERSION_LEADERLESS_EXPERIMENTAL,
        implementation_id: "in-process-leaderless-quorum-diagnostic-v1",
        enabled: true,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE, READ_CONSISTENCY_LOCAL_STALE],
        capabilities: vec![ENGINE_CAPABILITY_LINEARIZABLE_READ, ENGINE_CAPABILITY_LOCAL_STALE_READ],
        currentness_evidence_classes: vec!["leaderless-quorum-certificate"],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: PRODUCTION_STATUS_EXPERIMENTAL,
        required_evidence_refs: Vec::new(),
        conformance_receipt_refs: Vec::new(),
        caveats: default_consensus_caveats(),
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

fn disabled_fixture_engine_descriptor() -> Result<ConsensusEngineDescriptor> {
    let input = ConsensusEngineDescriptorInput {
        profile_id: "disabled-fixture-engine",
        profile_version: "disabled-fixture-v1",
        implementation_id: "disabled-fixture-implementation-v1",
        enabled: false,
        supported_read_consistency_modes: vec![READ_CONSISTENCY_LINEARIZABLE],
        capabilities: vec![ENGINE_CAPABILITY_PROPOSAL],
        currentness_evidence_classes: vec!["fixture-currentness"],
        membership_capabilities: vec![MEMBERSHIP_CAPABILITY_DENY_UNSUPPORTED],
        production_admission_status: ENGINE_STATUS_DISABLED,
        required_evidence_refs: vec![synthetic_ref("disabled-evidence")?],
        conformance_receipt_refs: vec![synthetic_ref("disabled-conformance")?],
        caveats: vec!["disabled fixture only".to_string()],
    };
    parse_consensus_engine_descriptor(&consensus_engine_descriptor_value(&input)?)
}

struct ConsensusEngineDescriptorInput<'a> {
    profile_id: &'a str,
    profile_version: &'a str,
    implementation_id: &'a str,
    enabled: bool,
    supported_read_consistency_modes: Vec<&'a str>,
    capabilities: Vec<&'a str>,
    currentness_evidence_classes: Vec<&'a str>,
    membership_capabilities: Vec<&'a str>,
    production_admission_status: &'a str,
    required_evidence_refs: Vec<String>,
    conformance_receipt_refs: Vec<String>,
    caveats: Vec<String>,
}

fn consensus_engine_descriptor_value(input: &ConsensusEngineDescriptorInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.profile_id, "consensus engine profile")?;
    validate_non_empty(input.profile_version, "consensus engine profile version")?;
    validate_non_empty(input.implementation_id, "consensus engine implementation id")?;
    validate_refs(&input.required_evidence_refs, "consensus engine evidence ref")?;
    validate_refs(&input.conformance_receipt_refs, "consensus engine conformance ref")?;
    validate_caveats(&input.caveats)?;
    Ok(record("consensus-engine-descriptor-v1", vec![
        string(CONSENSUS_ENGINE_DESCRIPTOR_SCHEMA),
        record("profile", vec![string(input.profile_id)]),
        record("version", vec![string(input.profile_version)]),
        record("implementation", vec![string(input.implementation_id)]),
        record("enabled", vec![bool_value(input.enabled)]),
        record("read-consistency", vec![strings_sequence(&borrowed_strings(&input.supported_read_consistency_modes))]),
        record("capabilities", vec![strings_sequence(&borrowed_strings(&input.capabilities))]),
        record("currentness", vec![strings_sequence(&borrowed_strings(&input.currentness_evidence_classes))]),
        record("membership", vec![strings_sequence(&borrowed_strings(&input.membership_capabilities))]),
        record("production-status", vec![string(input.production_admission_status)]),
        record("evidence", vec![strings_sequence(&input.required_evidence_refs)]),
        record("conformance", vec![strings_sequence(&input.conformance_receipt_refs)]),
        record("caveats", vec![strings_sequence(&input.caveats)]),
    ]))
}

fn borrowed_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn parse_consensus_engine_descriptor(value: &IoValue) -> Result<ConsensusEngineDescriptor> {
    let fields = value
        .collect_simple_record("consensus-engine-descriptor-v1", Some(CONSENSUS_ENGINE_DESCRIPTOR_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-descriptor-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_DESCRIPTOR_SCHEMA, "consensus engine descriptor schema")?;
    let profile_id = record_string(&fields[1], "profile")?;
    let profile_version = record_string(&fields[2], "version")?;
    let implementation_id = record_string(&fields[3], "implementation")?;
    let is_enabled = record_bool(&fields[4], "enabled")?;
    let supported_read_consistency_modes = parse_string_sequence(&fields[5], "read-consistency")?;
    validate_read_consistency_support(&supported_read_consistency_modes)?;
    let capabilities = parse_string_sequence(&fields[6], "capabilities")?;
    let currentness_evidence_classes = parse_string_sequence(&fields[7], "currentness")?;
    let membership_capabilities = parse_string_sequence(&fields[8], "membership")?;
    let production_admission_status = record_string(&fields[9], "production-status")?;
    let required_evidence_refs = parse_ref_sequence(&fields[10], "evidence")?;
    let conformance_receipt_refs = parse_ref_sequence(&fields[11], "conformance")?;
    let caveats = parse_string_sequence(&fields[12], "caveats")?;
    validate_engine_descriptor_core(EngineDescriptorInput { profile_id: &profile_id, profile_version: &profile_version, implementation_id: &implementation_id, capabilities: &capabilities, currentness: &currentness_evidence_classes, membership: &membership_capabilities, caveats: &caveats })?;
    Ok(ConsensusEngineDescriptor {
        descriptor_ref: canonical_hash(value)?,
        profile_id,
        profile_version,
        implementation_id,
        enabled: is_enabled,
        supported_read_consistency_modes,
        capabilities,
        currentness_evidence_classes,
        membership_capabilities,
        production_admission_status,
        required_evidence_refs,
        conformance_receipt_refs,
        caveats,
        value: value.clone(),
    })
}

struct EngineDescriptorInput<'a> {
    profile_id: &'a str,
    profile_version: &'a str,
    implementation_id: &'a str,
    capabilities: &'a [String],
    currentness: &'a [String],
    membership: &'a [String],
    caveats: &'a [String],
}

fn validate_engine_descriptor_core(input: EngineDescriptorInput<'_>) -> Result<()> {
    let EngineDescriptorInput { profile_id, profile_version, implementation_id, capabilities, currentness, membership, caveats } = input;
    validate_non_empty(profile_id, "consensus engine profile")?;
    validate_non_empty(profile_version, "consensus engine profile version")?;
    validate_non_empty(implementation_id, "consensus engine implementation id")?;
    validate_string_items(capabilities, "consensus engine capability")?;
    validate_string_items(currentness, "consensus engine currentness class")?;
    validate_string_items(membership, "consensus engine membership capability")?;
    validate_caveats(caveats)
}

fn consensus_engine_registry_value(entries: &[ConsensusEngineDescriptor]) -> Result<IoValue> {
    ensure_count_at_most(entries.len(), MAX_RAFT_REFS, "consensus engine registry entries")?;
    if entries.is_empty() {
        return Err(MoltenError::invalid_harness("consensus engine registry requires entries"));
    }
    Ok(record("consensus-engine-registry-v1", vec![
        string(CONSENSUS_ENGINE_REGISTRY_SCHEMA),
        record("entries", vec![sequence(entries.iter().map(|entry| entry.value.clone()).collect())]),
        checks_value(&[("explicit-engine-registry", ENGINE_DECISION_PASS), ("fail-closed-resolution", ENGINE_DECISION_PASS)]),
    ]))
}

pub fn parse_consensus_engine_registry(value: &IoValue) -> Result<ConsensusEngineRegistry> {
    let fields = value
        .collect_simple_record("consensus-engine-registry-v1", Some(CONSENSUS_ENGINE_REGISTRY_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-registry-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_REGISTRY_SCHEMA, "consensus engine registry schema")?;
    let values = field_sequence(&fields[1], "entries")?;
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, "consensus engine registry entries")?;
    let mut entries = Vec::with_capacity(values.len());
    for value in values {
        entries.push(parse_consensus_engine_descriptor(&value_to_iovalue(&value))?);
    }
    require_unique_engine_keys(&entries)?;
    require_check(&parse_checks(&fields[2])?, "explicit-engine-registry", "consensus engine registry")?;
    Ok(ConsensusEngineRegistry {
        registry_ref: canonical_hash(value)?,
        entries,
        value: value.clone(),
    })
}

fn require_unique_engine_keys(entries: &[ConsensusEngineDescriptor]) -> Result<()> {
    let mut keys = std::collections::BTreeSet::new();
    for entry in entries {
        let key = engine_key(&entry.profile_id, &entry.profile_version);
        if !keys.insert(key.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate consensus engine registry entry {key}")));
        }
    }
    Ok(())
}

fn engine_key(profile_id: &str, profile_version: &str) -> String {
    format!("{profile_id}@{profile_version}")
}

// r[impl molten.consensus.engine_admission_policy]
// r[impl molten.testing.consensus_registry_negative_fixtures]
pub fn admit_consensus_engine(
    registry: &ConsensusEngineRegistry,
    input: &ConsensusEngineAdmissionInput,
) -> Result<ConsensusEngineAdmissionReceipt> {
    let descriptor = matching_engine_descriptor(registry, &input.algorithm_profile, &input.profile_version);
    let mut diagnostics = Vec::new();
    if descriptor.is_none() {
        if registry.entries.iter().any(|entry| entry.profile_id == input.algorithm_profile) {
            diagnostics.push(format!(
                "consensus engine profile version mismatch for {} version {}",
                input.algorithm_profile, input.profile_version
            ));
        } else {
            diagnostics.push(format!(
                "unsupported consensus engine profile {} version {}",
                input.algorithm_profile, input.profile_version
            ));
        }
    }
    if let Some(descriptor) = descriptor {
        collect_engine_admission_diagnostics(descriptor, input, &mut diagnostics)?;
    }
    ensure_count_at_most(diagnostics.len(), MAX_RAFT_DIAGNOSTICS, "consensus engine admission diagnostics")?;
    let decision = if diagnostics.is_empty() { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY };
    let value = consensus_engine_admission_receipt_value(input, descriptor, decision, &diagnostics)?;
    Ok(ConsensusEngineAdmissionReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        descriptor: descriptor.cloned(),
        diagnostics,
        value,
    })
}

fn matching_engine_descriptor<'a>(
    registry: &'a ConsensusEngineRegistry,
    profile_id: &str,
    profile_version: &str,
) -> Option<&'a ConsensusEngineDescriptor> {
    registry
        .entries
        .iter()
        .find(|entry| entry.profile_id == profile_id && entry.profile_version == profile_version)
}
