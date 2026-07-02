
fn call_binding(base: &CallBase, context: HostcallEvidenceContext<'_>) -> Result<CallBinding> {
    let adapter_ref = canonical_hash(&record("hostcall-adapter-surface", vec![
        string(&base.actor_id),
        string(base.actor_kind),
        string(&base.preflight_ref),
    ]))?;
    let value = crate::effects::handler_binding_value(&crate::effects::HandlerBindingInput {
        profile: "local-hostcall".to_string(),
        scope: base.scope.clone(),
        adapter_kind: "hostcall".to_string(),
        adapter_ref,
        executor_preflight_ref: Some(base.preflight_ref.clone()),
        policy_ref: context.policy_ref.to_string(),
        capability_context_ref: context.capability_ref.to_string(),
        context_ref: None,
        resource_refs: base.resource_refs.clone(),
        operations: base.allowed_hostcalls.clone(),
        evidence_refs: base.evidence_refs.clone(),
    })?;
    let value_ref = canonical_hash(&value)?;
    let handle = crate::effects::effect_handle_value(&crate::effects::EffectHandleInput {
        kind: "hostcall".to_string(),
        scope: base.scope.clone(),
        handler_binding_ref: value_ref.clone(),
        operations: vec![base.operation.to_string()],
        capability_context_ref: context.capability_ref.to_string(),
        context_ref: None,
        resource_refs: base.resource_refs.clone(),
        not_before: Some(0),
        expires_at: None,
        revocation_refs: Vec::new(),
        transfer: crate::effects::TRANSFER_LOCAL_ONLY.to_string(),
        parent_handle_ref: None,
        evidence_refs: base.evidence_refs.clone(),
    })?;
    let handle_ref = canonical_hash(&handle)?;
    Ok(CallBinding {
        value,
        value_ref,
        handle,
        handle_ref,
    })
}

fn request_refs(base: &CallBase, binding: &CallBinding, context: HostcallEvidenceContext<'_>) -> Result<RequestRefs> {
    let effect_id = format!("hostcall.{}", base.operation);
    let effect_manifest = call_effect_manifest(base, context)?;
    let effect_manifest_ref = canonical_hash(&effect_manifest)?;
    let handler_profile = call_handler_profile(base, binding, context)?;
    let handler_profile_ref = canonical_hash(&handler_profile)?;
    let effect_request = call_effect_request(base, binding, context, effect_id)?;
    let effect_request_ref = canonical_hash(&effect_request)?;
    let effect_binding_receipt_ref =
        call_binding_receipt_ref(base, binding, &effect_manifest, &handler_profile, &effect_request)?;
    Ok(RequestRefs {
        effect_manifest_ref: Some(effect_manifest_ref),
        handler_profile_ref: Some(handler_profile_ref),
        effect_request_ref: Some(effect_request_ref),
        effect_binding_receipt_ref: Some(effect_binding_receipt_ref),
    })
}

fn call_effect_manifest(base: &CallBase, context: HostcallEvidenceContext<'_>) -> Result<IoValue> {
    crate::effects::effect_manifest_value(&crate::effects::EffectManifestInput {
        artifact_kind: base.actor_kind.to_string(),
        artifact_ref: base.actor_ref.clone(),
        executor_kind: base.actor_kind.to_string(),
        declared_effects: base
            .allowed_hostcalls
            .as_slice()
            .iter()
            .map(|hostcall| crate::effects::DeclaredEffect {
                effect_id: format!("hostcall.{hostcall}"),
                operation: hostcall.clone(),
                input_schema_ref: context.step_ref.to_string(),
                output_schema_ref: context.step_ref.to_string(),
                evidence_refs: vec![base.preflight_ref.clone()],
            })
            .collect(),
        policy_refs: vec![context.policy_ref.to_string()],
        evidence_refs: vec![base.preflight_ref.clone()],
    })
}

fn call_handler_profile(
    base: &CallBase,
    binding: &CallBinding,
    context: HostcallEvidenceContext<'_>,
) -> Result<IoValue> {
    crate::effects::handler_profile_value(&crate::effects::HandlerProfileInput {
        profile: crate::effects::HANDLER_PROFILE_LOCAL.to_string(),
        handler_binding_refs: vec![binding.value_ref.clone()],
        policy_ref: context.policy_ref.to_string(),
        capability_context_ref: context.capability_ref.to_string(),
        resource_refs: base.resource_refs.clone(),
        evidence_refs: vec![base.preflight_ref.clone()],
    })
}

fn call_effect_request(
    base: &CallBase,
    binding: &CallBinding,
    context: HostcallEvidenceContext<'_>,
    effect_id: String,
) -> Result<IoValue> {
    crate::effects::effect_request_value(&crate::effects::EffectRequestInput {
        artifact_ref: base.actor_ref.clone(),
        effect_id,
        operation: base.operation.to_string(),
        handler_profile: crate::effects::HANDLER_PROFILE_LOCAL.to_string(),
        input_ref: context.step_ref.to_string(),
        capability_refs: vec![context.capability_ref.to_string()],
        evidence_refs: vec![binding.value_ref.clone(), binding.handle_ref.clone()],
    })
}

fn call_binding_receipt_ref(
    base: &CallBase,
    binding: &CallBinding,
    manifest: &IoValue,
    profile: &IoValue,
    request: &IoValue,
) -> Result<String> {
    let effect_binding = crate::effects::admit_effect_request(manifest, profile, request, &[
        binding.value_ref.clone(),
        binding.handle_ref.clone(),
    ])?;
    if effect_binding.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "hostcall effect manifest denied operation {}: {:?}",
            base.operation, effect_binding.diagnostics
        )));
    }
    Ok(effect_binding.receipt_ref.clone())
}

fn validate_call_handle(base: &CallBase, binding: &CallBinding, context: HostcallEvidenceContext<'_>) -> Result<()> {
    let validation = crate::effects::validate_handle_for_request(
        &binding.value,
        &binding.handle,
        &crate::effects::EffectHandleRequest {
            kind: "hostcall",
            operation: base.operation,
            run_ref: context.suite_ref,
            session_ref: &base.session_ref,
            actor_ref: Some(&base.actor_ref),
            turn_ref: Some(context.step_ref),
            policy_ref: context.policy_ref,
            capability_context_ref: context.capability_ref,
            context_ref: None,
            resource_refs: &base.resource_refs,
            logical_time: context.sequence,
            remote_use: false,
            revoked_refs: &[],
        },
    )?;
    if validation.handler_binding_ref != binding.value_ref || validation.handle_ref != binding.handle_ref {
        return Err(MoltenError::invalid_harness("hostcall effect handle validation ref mismatch"));
    }
    Ok(())
}

fn actor_identity_ref(actor_id: &str) -> Result<String> {
    canonical_hash(&record("actor-identity-v1", vec![string(actor_id)]))
}

fn hostcall_session_ref(context: HostcallEvidenceContext<'_>) -> Result<String> {
    canonical_hash(&record("hostcall-session-v1", vec![
        string(context.suite_ref),
        string(context.policy_ref),
        string(context.capability_ref),
        string(context.budget_ref),
    ]))
}

pub(crate) fn validate_hostcall_effect_binding_request(hostcall_request: &IoValue, operation: &str) -> Result<()> {
    let request = hostcall_request
        .collect_simple_record("hostcall-request-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("executor hostcall gate requires bound effect request evidence"))?;
    let request_operation = required_record_string(&request[3], "operation", "hostcall request operation")?;
    if request_operation != operation {
        return Err(MoltenError::invalid_harness(format!(
            "executor hostcall gate operation mismatch: got {request_operation}, expected {operation}"
        )));
    }
    for (field, label) in [
        (&request[11], "effect-manifest-ref"),
        (&request[12], "handler-profile-ref"),
        (&request[13], "effect-request-ref"),
        (&request[14], "effect-binding-receipt-ref"),
    ] {
        let content_ref = required_record_string(field, label, label)?;
        validate_content_ref(&content_ref)?;
    }
    Ok(())
}

pub fn hostcall_decision_value(
    context: HostcallEvidenceContext<'_>,
    admission_event: &IoValue,
    authority: &AdmissionAuthorityEvidence,
    decision: &crate::runtime::AdmissionDecision,
) -> Result<IoValue> {
    let authority_value = admission_authority_value(authority);
    Ok(record("hostcall-decision-v1", vec![
        string(crate::preserves_rail::RUNTIME_HOSTCALL_DECISION_SCHEMA),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
        record("admission-ref", vec![string(canonical_hash(admission_event)?)]),
        record("authority-ref", vec![string(canonical_hash(&authority_value)?)]),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        hostcall_checks_value(&["admission-binding", "authority-binding", "budget-binding"]),
    ]))
}

pub fn actor_output_value(
    step: &super::core::CoreStep,
    context: HostcallEvidenceContext<'_>,
    decision: &crate::runtime::AdmissionDecision,
    runtime_events: &[IoValue],
) -> Result<IoValue> {
    let runtime_events_value = sequence(runtime_events.to_vec());
    Ok(record("actor-output-v1", vec![
        string(crate::preserves_rail::RUNTIME_ACTOR_OUTPUT_SCHEMA),
        record("actor", vec![string(step.primary_actor())]),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("decision", vec![string(decision.status())]),
        record("events-ref", vec![string(canonical_hash(&runtime_events_value)?)]),
        record("events", vec![u64_value(runtime_events.len() as u64)]),
        hostcall_checks_value(&["staged-output", "deterministic-trace"]),
    ]))
}

pub(crate) struct SteelExecutionReceiptInput<'a> {
    pub actor_id: &'a str,
    pub source_ref: &'a str,
    pub callable: &'a str,
    pub operation: &'a str,
    pub input_ref: &'a str,
    pub output_ref: &'a str,
    pub hostcalls: &'a [String],
    pub resource_limits: SteelResourceReceiptInput,
}

pub(crate) struct SteelResourceReceiptInput {
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
    pub source_bytes: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub hostcall_limit: u64,
    pub hostcall_count: u64,
}
