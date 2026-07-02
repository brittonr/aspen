
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundHandlerProfileInput {
    pub profile: String,
    pub scope: EffectScope,
    pub handler_binding_refs: Vec<String>,
    pub child_handle_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundHandlerProfile {
    pub profile_ref: String,
    pub profile: String,
    pub scope: EffectScope,
    pub handler_binding_refs: Vec<String>,
    pub child_handle_refs: Vec<String>,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub context_ref: Option<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicOperationRecordInput {
    pub operation: String,
    pub adapter_ref: String,
    pub callable_ref: String,
    pub request_ref: String,
    pub response_ref: String,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicOperationRecord {
    pub record_ref: String,
    pub operation: String,
    pub adapter_ref: String,
    pub callable_ref: String,
    pub request_ref: String,
    pub response_ref: String,
    pub policy_ref: String,
    pub capability_context_ref: String,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleAttenuationInput {
    pub scope: EffectScope,
    pub operations: Vec<String>,
    pub expires_at: Option<u64>,
    pub transfer: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleCleanupReceipt {
    pub receipt_ref: String,
    pub handle_ref: String,
    pub action: String,
    pub live_usable: bool,
    pub preserve_artifact: bool,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub value: IoValue,
}

pub fn effect_manifest_value(input: &EffectManifestInput) -> Result<IoValue> {
    validate_non_empty(&input.artifact_kind, "effect manifest artifact kind")?;
    require_ref(&input.artifact_ref, "effect manifest artifact ref")?;
    validate_executor_kind(&input.executor_kind)?;
    validate_declared_effects(&input.declared_effects)?;
    validate_refs(&input.policy_refs, "effect manifest policy ref")?;
    validate_refs(&input.evidence_refs, "effect manifest evidence ref")?;
    Ok(record("effect-manifest-v1", vec![
        string(EFFECT_MANIFEST_SCHEMA),
        record("artifact", vec![string(&input.artifact_kind), string(&input.artifact_ref)]),
        record("executor", vec![string(&input.executor_kind)]),
        record("effects", vec![sequence(
            input.declared_effects.iter().map(declared_effect_value).collect(),
        )]),
        refs_record("policy", &input.policy_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "declared-effect-ids",
            "artifact-effect-manifest-link",
            "no-unison-runtime-compatibility",
            "deny-undeclared-effects",
        ]),
    ]))
}

pub fn parse_effect_manifest(value: &IoValue) -> Result<EffectManifest> {
    let fields = simple_record(value, "effect-manifest-v1", 7)?;
    require_schema(&fields[0], EFFECT_MANIFEST_SCHEMA, "effect manifest schema")?;
    let artifact = value_to_iovalue(&fields[1]);
    let artifact = simple_record(&artifact, "artifact", 2)?;
    let executor = value_to_iovalue(&fields[2]);
    let executor = simple_record(&executor, "executor", 1)?;
    let artifact_kind = required_string(&artifact[0], "effect manifest artifact kind")?;
    let artifact_ref = required_ref(&artifact[1], "effect manifest artifact ref")?;
    let executor_kind = required_string(&executor[0], "effect manifest executor kind")?;
    validate_executor_kind(&executor_kind)?;
    let declared_effects = parse_declared_effects(&fields[3])?;
    let policy_refs = parse_ref_sequence_record(&fields[4], "policy")?;
    let evidence_refs = parse_ref_sequence_record(&fields[5], "evidence")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "declared-effect-ids", "effect manifest")?;
    require_check(&checks, "artifact-effect-manifest-link", "effect manifest")?;
    Ok(EffectManifest {
        manifest_ref: canonical_hash(value)?,
        artifact_kind,
        artifact_ref,
        executor_kind,
        declared_effects,
        policy_refs,
        evidence_refs,
        checks,
        value: value.clone(),
    })
}

pub fn handler_profile_value(input: &HandlerProfileInput) -> Result<IoValue> {
    validate_handler_profile(&input.profile)?;
    validate_refs(&input.handler_binding_refs, "handler profile binding ref")?;
    validate_unique_refs(&input.handler_binding_refs, "handler profile binding ref")?;
    require_ref(&input.policy_ref, "handler profile policy ref")?;
    require_ref(&input.capability_context_ref, "handler profile capability context ref")?;
    validate_refs(&input.resource_refs, "handler profile resource ref")?;
    validate_refs(&input.evidence_refs, "handler profile evidence ref")?;
    Ok(record("handler-profile-v1", vec![
        string(EFFECT_HANDLER_PROFILE_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        refs_record("handler-bindings", &input.handler_binding_refs),
        record("policy", vec![string(&input.policy_ref), string(&input.capability_context_ref)]),
        refs_record("resources", &input.resource_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "handler-profile-admitted",
            "policy-capability-resource-binding",
            "deny-ambient-effects",
        ]),
    ]))
}

pub fn parse_handler_profile(value: &IoValue) -> Result<HandlerProfile> {
    let fields = simple_record(value, "handler-profile-v1", 7)?;
    require_schema(&fields[0], EFFECT_HANDLER_PROFILE_SCHEMA, "handler profile schema")?;
    let profile = required_record_string(&fields[1], "profile", "handler profile")?;
    validate_handler_profile(&profile)?;
    let policy = value_to_iovalue(&fields[3]);
    let policy = simple_record(&policy, "policy", 2)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "handler-profile-admitted", "handler profile")?;
    Ok(HandlerProfile {
        profile_ref: canonical_hash(value)?,
        profile,
        handler_binding_refs: parse_ref_sequence_record(&fields[2], "handler-bindings")?,
        policy_ref: required_ref(&policy[0], "handler profile policy ref")?,
        capability_context_ref: required_ref(&policy[1], "handler profile capability context ref")?,
        resource_refs: parse_ref_sequence_record(&fields[4], "resources")?,
        evidence_refs: parse_ref_sequence_record(&fields[5], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn effect_request_value(input: &EffectRequestInput) -> Result<IoValue> {
    require_ref(&input.artifact_ref, "effect request artifact ref")?;
    validate_effect_id(&input.effect_id)?;
    validate_operation(&input.operation)?;
    validate_handler_profile(&input.handler_profile)?;
    require_ref(&input.input_ref, "effect request input ref")?;
    validate_refs(&input.capability_refs, "effect request capability ref")?;
    validate_refs(&input.evidence_refs, "effect request evidence ref")?;
    Ok(record("effect-request-v1", vec![
        string(EFFECT_REQUEST_SCHEMA),
        record("artifact", vec![string(&input.artifact_ref)]),
        record("effect", vec![string(&input.effect_id), string(&input.operation)]),
        record("handler-profile", vec![string(&input.handler_profile)]),
        record("input", vec![string(&input.input_ref)]),
        refs_record("capabilities", &input.capability_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "canonical-effect-request",
            "manifest-effect-id-bound",
            "handler-profile-bound",
        ]),
    ]))
}

pub fn parse_effect_request(value: &IoValue) -> Result<EffectRequest> {
    let fields = simple_record(value, "effect-request-v1", 8)?;
    require_schema(&fields[0], EFFECT_REQUEST_SCHEMA, "effect request schema")?;
    let effect = value_to_iovalue(&fields[2]);
    let effect = simple_record(&effect, "effect", 2)?;
    let effect_id = required_string(&effect[0], "effect request effect id")?;
    let operation = required_string(&effect[1], "effect request operation")?;
    validate_effect_id(&effect_id)?;
    validate_operation(&operation)?;
    let handler_profile = required_record_string(&fields[3], "handler-profile", "effect request handler profile")?;
    validate_handler_profile(&handler_profile)?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-effect-request", "effect request")?;
    Ok(EffectRequest {
        request_ref: canonical_hash(value)?,
        artifact_ref: required_record_ref(&fields[1], "artifact", "effect request artifact ref")?,
        effect_id,
        operation,
        handler_profile,
        input_ref: required_record_ref(&fields[4], "input", "effect request input ref")?,
        capability_refs: parse_ref_sequence_record(&fields[5], "capabilities")?,
        evidence_refs: parse_ref_sequence_record(&fields[6], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn effect_response_value(input: &EffectResponseInput) -> Result<IoValue> {
    validate_decision(&input.decision)?;
    require_ref(&input.request_ref, "effect response request ref")?;
    if let Some(output_ref) = input.output_ref.as_deref() {
        require_ref(output_ref, "effect response output ref")?;
    }
    validate_refs(&input.evidence_refs, "effect response evidence ref")?;
    Ok(record("effect-response-v1", vec![
        string(EFFECT_RESPONSE_SCHEMA),
        record("request", vec![string(&input.request_ref)]),
        record("decision", vec![string(&input.decision)]),
        record("output", vec![optional_ref_value(input.output_ref.as_deref())]),
        diagnostics_record(&input.diagnostics),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&["canonical-effect-response", "request-ref-bound", "decision-recorded"]),
    ]))
}

pub fn parse_effect_response(value: &IoValue) -> Result<EffectResponse> {
    let fields = simple_record(value, "effect-response-v1", 7)?;
    require_schema(&fields[0], EFFECT_RESPONSE_SCHEMA, "effect response schema")?;
    let decision = required_record_string(&fields[2], "decision", "effect response decision")?;
    validate_decision(&decision)?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "canonical-effect-response", "effect response")?;
    Ok(EffectResponse {
        response_ref: canonical_hash(value)?,
        request_ref: required_record_ref(&fields[1], "request", "effect response request ref")?,
        decision,
        output_ref: parse_optional_ref_record(&fields[3], "output")?,
        diagnostics: parse_string_sequence_record_unvalidated(&fields[4], "diagnostics")?,
        evidence_refs: parse_ref_sequence_record(&fields[5], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn effect_binding_receipt_value(input: &EffectBindingReceiptInput) -> Result<IoValue> {
    validate_decision(&input.decision)?;
    require_ref(&input.manifest_ref, "effect binding manifest ref")?;
    require_ref(&input.handler_profile_ref, "effect binding profile ref")?;
    require_ref(&input.request_ref, "effect binding request ref")?;
    validate_effect_id(&input.effect_id)?;
    validate_operation(&input.operation)?;
    validate_handler_profile(&input.handler_profile)?;
    validate_refs(&input.evidence_refs, "effect binding evidence ref")?;
    Ok(record("effect-binding-receipt-v1", vec![
        string(EFFECT_BINDING_RECEIPT_SCHEMA),
        record("decision", vec![string(&input.decision)]),
        record("manifest", vec![string(&input.manifest_ref)]),
        record("handler-profile", vec![string(&input.handler_profile_ref), string(&input.handler_profile)]),
        record("request", vec![string(&input.request_ref)]),
        record("effect", vec![string(&input.effect_id), string(&input.operation)]),
        diagnostics_record(&input.diagnostics),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "effect-manifest-bound",
            "handler-profile-bound",
            "deny-undeclared-effects",
            "content-addressing-is-not-authority",
        ]),
    ]))
}
