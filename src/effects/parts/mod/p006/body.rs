
const HANDLER_PROFILE_ADMISSION_RECEIPT_SCHEMA: &str = "molten.effects.handler-profile-admission-receipt.v1";
const EFFECT_PROFILE_REPLAY_BINDING_SCHEMA: &str = "molten.effects.profile-replay-binding.v1";
const HANDLER_PROFILE_ADMISSION_RECEIPT_FIELD_COUNT: usize = 12;
const EFFECT_PROFILE_REPLAY_BINDING_FIELD_COUNT: usize = 11;

const EFFECT_PROFILE_INTEGRATION_REPLAY: &str = "replay";
const EFFECT_PROFILE_INTEGRATION_TRANSCRIPT: &str = "transcript";
const EFFECT_PROFILE_INTEGRATION_EVAL_CACHE: &str = "eval-cache";
const EFFECT_PROFILE_INTEGRATION_JOB_DAG: &str = "job-dag";
const EFFECT_PROFILE_INTEGRATION_REMOTE_EXECUTION: &str = "remote-execution";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerProfileAdmissionInput {
    pub manifest: IoValue,
    pub handler_profile: IoValue,
    pub supported_effects: Vec<DeclaredEffect>,
    pub determinism_class: String,
    pub replay_class: String,
    pub current_policy_ref: String,
    pub current_capability_context_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandlerProfileAdmissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub handler_profile_ref: String,
    pub handler_profile: String,
    pub supported_effect_count: usize,
    pub determinism_class: String,
    pub replay_class: String,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProfileReplayBindingInput {
    pub integration_kind: String,
    pub subject_ref: String,
    pub effect_manifest_ref: String,
    pub handler_profile_ref: String,
    pub profile_admission_ref: String,
    pub expected_manifest_ref: Option<String>,
    pub expected_handler_profile_ref: Option<String>,
    pub compatibility_ref: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProfileReplayBinding {
    pub binding_ref: String,
    pub decision: String,
    pub integration_kind: String,
    pub subject_ref: String,
    pub effect_manifest_ref: String,
    pub handler_profile_ref: String,
    pub profile_admission_ref: String,
    pub compatibility_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

struct HandlerProfileAdmissionValueInput<'a> {
    decision: &'a str,
    manifest_ref: &'a str,
    handler_profile_ref: &'a str,
    handler_profile: &'a str,
    supported_effects: &'a [DeclaredEffect],
    determinism_class: &'a str,
    replay_class: &'a str,
    policy_ref: &'a str,
    capability_context_ref: &'a str,
    resource_refs: &'a [String],
    diagnostics: &'a [String],
    evidence_refs: &'a [String],
}

struct EffectProfileReplayBindingValueInput<'a> {
    decision: &'a str,
    integration_kind: &'a str,
    subject_ref: &'a str,
    effect_manifest_ref: &'a str,
    handler_profile_ref: &'a str,
    profile_admission_ref: &'a str,
    compatibility_ref: Option<&'a str>,
    diagnostics: &'a [String],
    evidence_refs: &'a [String],
}

pub fn admit_handler_profile_for_manifest(
    input: HandlerProfileAdmissionInput,
) -> Result<HandlerProfileAdmissionReceipt> {
    let manifest = parse_effect_manifest(&input.manifest)?;
    let profile = parse_handler_profile(&input.handler_profile)?;
    validate_declared_effects(&input.supported_effects)?;
    validate_effect_determinism_class(&input.determinism_class)?;
    validate_effect_replay_class(&input.replay_class)?;
    require_ref(&input.current_policy_ref, "handler profile admission current policy ref")?;
    require_ref(
        &input.current_capability_context_ref,
        "handler profile admission current capability context ref",
    )?;
    validate_refs(&input.evidence_refs, "handler profile admission evidence ref")?;
    let mut diagnostics = handler_profile_admission_diagnostics(&manifest, &profile, &input);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = handler_profile_admission_receipt_value(&HandlerProfileAdmissionValueInput {
        decision,
        manifest_ref: &manifest.manifest_ref,
        handler_profile_ref: &profile.profile_ref,
        handler_profile: &profile.profile,
        supported_effects: &input.supported_effects,
        determinism_class: &input.determinism_class,
        replay_class: &input.replay_class,
        policy_ref: &input.current_policy_ref,
        capability_context_ref: &input.current_capability_context_ref,
        resource_refs: &profile.resource_refs,
        diagnostics: &diagnostics,
        evidence_refs: &input.evidence_refs,
    })?;
    diagnostics.shrink_to_fit();
    Ok(HandlerProfileAdmissionReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        manifest_ref: manifest.manifest_ref,
        handler_profile_ref: profile.profile_ref,
        handler_profile: profile.profile,
        supported_effect_count: input.supported_effects.len(),
        determinism_class: input.determinism_class,
        replay_class: input.replay_class,
        policy_ref: input.current_policy_ref,
        capability_context_ref: input.current_capability_context_ref,
        resource_refs: profile.resource_refs,
        diagnostics,
        evidence_refs: input.evidence_refs,
        checks: handler_profile_admission_check_names(),
        value,
    })
}

pub fn parse_handler_profile_admission_receipt(value: &IoValue) -> Result<HandlerProfileAdmissionReceipt> {
    let fields = simple_record(
        value,
        "handler-profile-admission-receipt-v1",
        HANDLER_PROFILE_ADMISSION_RECEIPT_FIELD_COUNT,
    )?;
    require_schema(
        &fields[0],
        HANDLER_PROFILE_ADMISSION_RECEIPT_SCHEMA,
        "handler profile admission receipt schema",
    )?;
    let decision = required_record_string(&fields[1], "decision", "handler profile admission decision")?;
    validate_decision(&decision)?;
    let profile_record = value_to_iovalue(&fields[3]);
    let profile_record = simple_record(&profile_record, "handler-profile", 2)?;
    let policy_record = value_to_iovalue(&fields[7]);
    let policy_record = simple_record(&policy_record, "policy", 2)?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "handler-profile-admission-receipt", "handler profile admission receipt")?;
    Ok(HandlerProfileAdmissionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        manifest_ref: required_record_ref(&fields[2], "manifest", "handler profile admission manifest ref")?,
        handler_profile_ref: required_ref(&profile_record[0], "handler profile admission profile ref")?,
        handler_profile: required_string(&profile_record[1], "handler profile admission profile")?,
        supported_effect_count: parse_supported_effect_count(&fields[4])?,
        determinism_class: required_record_string(&fields[5], "determinism", "handler profile determinism class")?,
        replay_class: required_record_string(&fields[6], "replay", "handler profile replay class")?,
        policy_ref: required_ref(&policy_record[0], "handler profile admission policy ref")?,
        capability_context_ref: required_ref(&policy_record[1], "handler profile admission capability context ref")?,
        resource_refs: parse_ref_sequence_record(&fields[8], "resources")?,
        diagnostics: parse_string_sequence_record_unvalidated(&fields[9], "diagnostics")?,
        evidence_refs: parse_ref_sequence_record(&fields[10], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn bind_effect_profile_replay_evidence(
    input: EffectProfileReplayBindingInput,
) -> Result<EffectProfileReplayBinding> {
    validate_effect_profile_integration_kind(&input.integration_kind)?;
    require_ref(&input.subject_ref, "effect profile binding subject ref")?;
    require_ref(&input.effect_manifest_ref, "effect profile binding manifest ref")?;
    require_ref(&input.handler_profile_ref, "effect profile binding handler profile ref")?;
    require_ref(&input.profile_admission_ref, "effect profile binding admission ref")?;
    if let Some(expected_manifest_ref) = input.expected_manifest_ref.as_deref() {
        require_ref(expected_manifest_ref, "effect profile binding expected manifest ref")?;
    }
    if let Some(expected_handler_profile_ref) = input.expected_handler_profile_ref.as_deref() {
        require_ref(
            expected_handler_profile_ref,
            "effect profile binding expected handler profile ref",
        )?;
    }
    if let Some(compatibility_ref) = input.compatibility_ref.as_deref() {
        require_ref(compatibility_ref, "effect profile binding compatibility ref")?;
    }
    validate_refs(&input.evidence_refs, "effect profile binding evidence ref")?;
    let diagnostics = effect_profile_replay_binding_diagnostics(&input);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let value = effect_profile_replay_binding_value(&EffectProfileReplayBindingValueInput {
        decision,
        integration_kind: &input.integration_kind,
        subject_ref: &input.subject_ref,
        effect_manifest_ref: &input.effect_manifest_ref,
        handler_profile_ref: &input.handler_profile_ref,
        profile_admission_ref: &input.profile_admission_ref,
        compatibility_ref: input.compatibility_ref.as_deref(),
        diagnostics: &diagnostics,
        evidence_refs: &input.evidence_refs,
    })?;
    Ok(EffectProfileReplayBinding {
        binding_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        integration_kind: input.integration_kind,
        subject_ref: input.subject_ref,
        effect_manifest_ref: input.effect_manifest_ref,
        handler_profile_ref: input.handler_profile_ref,
        profile_admission_ref: input.profile_admission_ref,
        compatibility_ref: input.compatibility_ref,
        diagnostics,
        evidence_refs: input.evidence_refs,
        checks: effect_profile_replay_binding_check_names(),
        value,
    })
}

pub fn parse_effect_profile_replay_binding(value: &IoValue) -> Result<EffectProfileReplayBinding> {
    let fields = simple_record(
        value,
        "effect-profile-replay-binding-v1",
        EFFECT_PROFILE_REPLAY_BINDING_FIELD_COUNT,
    )?;
    require_schema(&fields[0], EFFECT_PROFILE_REPLAY_BINDING_SCHEMA, "effect profile replay binding schema")?;
    let decision = required_record_string(&fields[1], "decision", "effect profile replay binding decision")?;
    validate_decision(&decision)?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "effect-manifest-ref-bound", "effect profile replay binding")?;
    Ok(EffectProfileReplayBinding {
        binding_ref: canonical_hash(value)?,
        decision,
        integration_kind: required_record_string(&fields[2], "integration", "effect profile integration kind")?,
        subject_ref: required_record_ref(&fields[3], "subject", "effect profile binding subject ref")?,
        effect_manifest_ref: required_record_ref(&fields[4], "effect-manifest", "effect profile binding manifest ref")?,
        handler_profile_ref: required_record_ref(&fields[5], "handler-profile", "effect profile binding profile ref")?,
        profile_admission_ref: required_record_ref(&fields[6], "profile-admission", "effect profile binding admission ref")?,
        compatibility_ref: parse_optional_ref_record(&fields[7], "compatibility")?,
        diagnostics: parse_string_sequence_record_unvalidated(&fields[8], "diagnostics")?,
        evidence_refs: parse_ref_sequence_record(&fields[9], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn unison_effect_compatibility_claim_diagnostics(metadata: &str) -> Vec<String> {
    let lower = metadata.to_ascii_lowercase();
    let denied = [
        "unison-compatible",
        "unison runtime compatibility",
        "compatible with unison runtime",
    ];
    denied
        .into_iter()
        .filter(|marker| lower.contains(marker))
        .map(|_| "Molten effect manifests treat Unison abilities as prior art only".to_string())
        .collect()
}
