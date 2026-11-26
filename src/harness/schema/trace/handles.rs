#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostcallEffectRefs {
    pub(super) handler_binding_ref: String,
    pub(super) handle_ref: String,
    pub(super) effect_manifest_ref: Option<String>,
    pub(super) handler_profile_ref: Option<String>,
    pub(super) effect_request_ref: Option<String>,
    pub(super) effect_binding_receipt_ref: Option<String>,
}

struct CallBase {
    actor_id: String,
    actor_kind: &'static str,
    actor_ref: String,
    operation: &'static str,
    session_ref: String,
    scope: crate::effects::EffectScope,
    allowed_hostcalls: Vec<String>,
    preflight_ref: String,
    resource_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct CallBinding {
    value: preserves::IOValue,
    value_ref: String,
    handle: preserves::IOValue,
    handle_ref: String,
}

#[derive(Default)]
struct RequestRefs {
    effect_manifest_ref: Option<String>,
    handler_profile_ref: Option<String>,
    effect_request_ref: Option<String>,
    effect_binding_receipt_ref: Option<String>,
}

pub(super) fn hostcall_effect_refs(
    suite: &super::model::Suite,
    step: &crate::harness::nucleus::CoreStep,
    context: super::observed::HostcallEvidenceContext<'_>,
    bind_effect_request: bool,
) -> crate::error::Result<HostcallEffectRefs> {
    let base = call_base(suite, step, context)?;
    let binding = call_binding(&base, context)?;
    let refs = if bind_effect_request {
        request_refs(&base, &binding, context)?
    } else {
        RequestRefs::default()
    };
    validate_call_handle(&base, &binding, context)?;
    Ok(HostcallEffectRefs {
        handler_binding_ref: binding.value_ref,
        handle_ref: binding.handle_ref,
        effect_manifest_ref: refs.effect_manifest_ref,
        handler_profile_ref: refs.handler_profile_ref,
        effect_request_ref: refs.effect_request_ref,
        effect_binding_receipt_ref: refs.effect_binding_receipt_ref,
    })
}

fn call_base(
    suite: &super::model::Suite,
    step: &crate::harness::nucleus::CoreStep,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<CallBase> {
    let actor = super::intake::actor_decl_for_primary_actor(suite, step.primary_actor())?;
    let operation = crate::harness::nucleus::AdmissionRequest::from_step(step).action.as_str();
    let actor_ref = actor_identity_ref(&actor.id)?;
    let session_ref = hostcall_session_ref(context)?;
    let scope = crate::effects::EffectScope {
        run_ref: context.suite_ref.to_string(),
        session_ref: session_ref.clone(),
        actor_ref: Some(actor_ref.clone()),
        turn_ref: Some(context.step_ref.to_string()),
    };
    let allowed_hostcalls = super::readiness::allowed_hostcalls_for_actor(suite, actor);
    let executor_preflight = super::readiness::executor_preflight_value(actor, &allowed_hostcalls)?;
    let preflight_ref = crate::preserves_rail::canonical_hash(&executor_preflight)?;
    Ok(CallBase {
        actor_id: actor.id.clone(),
        actor_kind: actor.kind.as_str(),
        actor_ref,
        operation,
        session_ref,
        scope,
        allowed_hostcalls,
        preflight_ref: preflight_ref.clone(),
        resource_refs: vec![context.budget_ref.to_string()],
        evidence_refs: vec![preflight_ref],
    })
}

fn call_binding(
    base: &CallBase,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<CallBinding> {
    let adapter_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("hostcall-adapter-surface", vec![
            crate::preserves_rail::string(&base.actor_id),
            crate::preserves_rail::string(base.actor_kind),
            crate::preserves_rail::string(&base.preflight_ref),
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
    let value_ref = crate::preserves_rail::canonical_hash(&value)?;
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
    let handle_ref = crate::preserves_rail::canonical_hash(&handle)?;
    Ok(CallBinding {
        value,
        value_ref,
        handle,
        handle_ref,
    })
}

fn request_refs(
    base: &CallBase,
    binding: &CallBinding,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<RequestRefs> {
    let effect_id = format!("hostcall.{}", base.operation);
    let effect_manifest = call_effect_manifest(base, context)?;
    let effect_manifest_ref = crate::preserves_rail::canonical_hash(&effect_manifest)?;
    let handler_profile = call_handler_profile(base, binding, context)?;
    let handler_profile_ref = crate::preserves_rail::canonical_hash(&handler_profile)?;
    let effect_request = call_effect_request(base, binding, context, effect_id)?;
    let effect_request_ref = crate::preserves_rail::canonical_hash(&effect_request)?;
    let effect_binding_receipt_ref =
        call_binding_receipt_ref(base, binding, &effect_manifest, &handler_profile, &effect_request)?;
    Ok(RequestRefs {
        effect_manifest_ref: Some(effect_manifest_ref),
        handler_profile_ref: Some(handler_profile_ref),
        effect_request_ref: Some(effect_request_ref),
        effect_binding_receipt_ref: Some(effect_binding_receipt_ref),
    })
}

fn call_effect_manifest(
    base: &CallBase,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<preserves::IOValue> {
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
                resource_class: "hostcall".to_string(),
                capability_refs: vec![context.capability_ref.to_string()],
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
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<preserves::IOValue> {
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
    context: super::observed::HostcallEvidenceContext<'_>,
    effect_id: String,
) -> crate::error::Result<preserves::IOValue> {
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
    manifest: &preserves::IOValue,
    profile: &preserves::IOValue,
    request: &preserves::IOValue,
) -> crate::error::Result<String> {
    let effect_binding = crate::effects::admit_effect_request(manifest, profile, request, &[
        binding.value_ref.clone(),
        binding.handle_ref.clone(),
    ])?;
    if effect_binding.decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "hostcall effect manifest denied operation {}: {:?}",
            base.operation, effect_binding.diagnostics
        )));
    }
    Ok(effect_binding.receipt_ref.clone())
}

fn validate_call_handle(
    base: &CallBase,
    binding: &CallBinding,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<()> {
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
        return Err(crate::error::Failure::invalid_harness("hostcall effect handle validation ref mismatch"));
    }
    Ok(())
}

fn actor_identity_ref(actor_id: &str) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("actor-identity-v1", vec![
        crate::preserves_rail::string(actor_id),
    ]))
}

fn hostcall_session_ref(context: super::observed::HostcallEvidenceContext<'_>) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("hostcall-session-v1", vec![
        crate::preserves_rail::string(context.suite_ref),
        crate::preserves_rail::string(context.policy_ref),
        crate::preserves_rail::string(context.capability_ref),
        crate::preserves_rail::string(context.budget_ref),
    ]))
}
