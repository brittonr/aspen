
pub fn event_value(event: &super::core::CoreEvent) -> IoValue {
    match event {
        super::core::CoreEvent::MessageDelivered { from, to, body } => {
            record("message-delivered", vec![string(from), string(to), body.as_iovalue().clone()])
        }
        super::core::CoreEvent::ObserveRegistered { actor, pattern } => {
            record("observe-registered", vec![string(actor), pattern.as_iovalue().clone()])
        }
        super::core::CoreEvent::AssertionObserved { observer, owner, value } => {
            record("assertion-observed", vec![string(observer), string(owner), value.as_iovalue().clone()])
        }
        super::core::CoreEvent::AssertionCommitted { actor, value } => {
            record("assertion-committed", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreEvent::AssertionRetracted { actor, value } => {
            record("assertion-retracted", vec![string(actor), value.as_iovalue().clone()])
        }
        super::core::CoreEvent::AssertionRetractionObserved { observer, owner, value } => {
            record("assertion-retraction-observed", vec![string(observer), string(owner), value.as_iovalue().clone()])
        }
        super::core::CoreEvent::EffectRequest {
            effect,
            actor,
            sequence,
            upper,
        } => {
            let mut fields = vec![string(effect_name(effect)), string(actor), u64_value(*sequence)];
            if let Some(upper) = upper {
                fields.push(u64_value(*upper));
            }
            record("effect-request", fields)
        }
        super::core::CoreEvent::EffectResponse {
            effect,
            actor,
            sequence,
            upper,
            value,
        } => {
            let mut fields = vec![string(effect_name(effect)), string(actor), u64_value(*sequence)];
            if let Some(upper) = upper {
                fields.push(u64_value(*upper));
            }
            fields.push(u64_value(*value));
            record("effect-response", fields)
        }
        super::core::CoreEvent::AdmissionDecision { request, decision } => {
            admission_decision_event_value(request, decision)
        }
        super::core::CoreEvent::TurnRolledBack { actor, reason } => {
            record("turn-rolled-back", vec![string(actor), string(reason)])
        }
    }
}

fn admission_decision_event_value(
    request: &super::core::AdmissionRequest,
    decision: &crate::runtime::AdmissionDecision,
) -> IoValue {
    record("admission-decision-v1", vec![
        string(crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
    ])
}

pub fn admission_decision_event_value_with_authority(
    request: &super::core::AdmissionRequest,
    authority: &AdmissionAuthorityEvidence,
    decision: &crate::runtime::AdmissionDecision,
) -> IoValue {
    record("admission-decision-v1", vec![
        string(crate::preserves_rail::RUNTIME_ADMISSION_DECISION_SCHEMA),
        admission_request_value(request),
        admission_authority_value(authority),
        record("decision", vec![string(decision.status()), string(decision.reason())]),
    ])
}

fn admission_authority_value(authority: &AdmissionAuthorityEvidence) -> IoValue {
    record("authority", vec![
        record("source", vec![string(&authority.source)]),
        record("capability-ref", vec![string(&authority.capability_ref)]),
        record("authorized", vec![bool_value(authority.authorized)]),
        optional_string_value(authority.grant_ref.as_deref()),
        record("request-ref", vec![string(&authority.request_ref)]),
        record("ucan-proofset-ref", vec![string(&authority.proofset_ref)]),
        authority_ref_sequence("ucan-verification-receipt-refs", &authority.ucan_verification_receipt_refs),
        authority_ref_sequence("derived-grant-refs", &authority.derived_grant_refs),
        record("basalt-enforcement-receipt-ref", vec![string(&authority.basalt_enforcement_receipt_ref)]),
        record("basalt-enforcement-receipt", vec![authority.basalt_enforcement_receipt_value.clone()]),
    ])
}

fn authority_ref_sequence(label: &'static str, refs: &[String]) -> IoValue {
    record(label, vec![sequence(refs.iter().map(string).collect())])
}

fn admission_request_value(request: &super::core::AdmissionRequest) -> IoValue {
    record("request", vec![
        string(&request.actor),
        string(request.action.as_str()),
        optional_string_value(request.target.as_deref()),
        optional_runtime_value(request.value.as_ref()),
        optional_u64_value(request.upper),
    ])
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_runtime_value(value: Option<&super::core::RuntimeValue>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![value.as_iovalue().clone()]))
}

fn optional_u64_value(value: Option<u64>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![u64_value(value)]))
}

pub fn actor_input_value(
    suite: &Suite,
    step: &super::core::CoreStep,
    context: HostcallEvidenceContext<'_>,
) -> Result<IoValue> {
    let actor = step.primary_actor();
    let kind = actor_kind_for_primary_actor(suite, actor)?;
    Ok(record("actor-input-v1", vec![
        string(crate::preserves_rail::RUNTIME_ACTOR_INPUT_SCHEMA),
        record("actor", vec![string(actor), string(kind.as_str())]),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        record("input", vec![step_value(step)]),
        hostcall_checks_value(&["canonical-preserves", "actor-registry-binding", "executor-boundary"]),
    ]))
}

pub fn hostcall_request_value(
    suite: &Suite,
    step: &super::core::CoreStep,
    context: HostcallEvidenceContext<'_>,
    decision: &crate::runtime::AdmissionDecision,
) -> Result<IoValue> {
    let request = super::core::AdmissionRequest::from_step(step);
    let effect_refs = hostcall_effect_refs(suite, step, context, decision.is_allowed())?;
    let checks = if decision.is_allowed() {
        vec![
            "no-ambient-executor-io",
            "policy-capability-budget-context",
            "handler-binding-available",
            "effect-handle-binding",
            "handle-not-authority",
            "effect-manifest-bound",
            "deny-undeclared-effects",
        ]
    } else {
        vec![
            "no-ambient-executor-io",
            "policy-capability-budget-context",
            "handler-binding-available",
            "effect-handle-binding",
            "handle-not-authority",
        ]
    };
    let mut fields = vec![
        string(crate::preserves_rail::RUNTIME_HOSTCALL_REQUEST_SCHEMA),
        record("sequence", vec![u64_value(context.sequence)]),
        record("step-ref", vec![string(context.step_ref)]),
        record("operation", vec![string(request.action.as_str())]),
        admission_request_value(&request),
        record("policy-ref", vec![string(context.policy_ref)]),
        record("capability-ref", vec![string(context.capability_ref)]),
        record("budget-ref", vec![string(context.budget_ref)]),
        hostcall_checks_value(&checks),
        record("handler-binding-ref", vec![string(&effect_refs.handler_binding_ref)]),
        record("handle-ref", vec![string(&effect_refs.handle_ref)]),
    ];
    if let Some(effect_manifest_ref) = &effect_refs.effect_manifest_ref {
        fields.push(record("effect-manifest-ref", vec![string(effect_manifest_ref)]));
    }
    if let Some(handler_profile_ref) = &effect_refs.handler_profile_ref {
        fields.push(record("handler-profile-ref", vec![string(handler_profile_ref)]));
    }
    if let Some(effect_request_ref) = &effect_refs.effect_request_ref {
        fields.push(record("effect-request-ref", vec![string(effect_request_ref)]));
    }
    if let Some(effect_binding_receipt_ref) = &effect_refs.effect_binding_receipt_ref {
        fields.push(record("effect-binding-receipt-ref", vec![string(effect_binding_receipt_ref)]));
    }
    Ok(record("hostcall-request-v1", fields))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostcallEffectRefs {
    handler_binding_ref: String,
    handle_ref: String,
    effect_manifest_ref: Option<String>,
    handler_profile_ref: Option<String>,
    effect_request_ref: Option<String>,
    effect_binding_receipt_ref: Option<String>,
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
    value: IoValue,
    value_ref: String,
    handle: IoValue,
    handle_ref: String,
}

#[derive(Default)]
struct RequestRefs {
    effect_manifest_ref: Option<String>,
    handler_profile_ref: Option<String>,
    effect_request_ref: Option<String>,
    effect_binding_receipt_ref: Option<String>,
}

fn hostcall_effect_refs(
    suite: &Suite,
    step: &super::core::CoreStep,
    context: HostcallEvidenceContext<'_>,
    bind_effect_request: bool,
) -> Result<HostcallEffectRefs> {
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

fn call_base(suite: &Suite, step: &super::core::CoreStep, context: HostcallEvidenceContext<'_>) -> Result<CallBase> {
    let actor = actor_decl_for_primary_actor(suite, step.primary_actor())?;
    let operation = super::core::AdmissionRequest::from_step(step).action.as_str();
    let actor_ref = actor_identity_ref(&actor.id)?;
    let session_ref = hostcall_session_ref(context)?;
    let scope = crate::effects::EffectScope {
        run_ref: context.suite_ref.to_string(),
        session_ref: session_ref.clone(),
        actor_ref: Some(actor_ref.clone()),
        turn_ref: Some(context.step_ref.to_string()),
    };
    let allowed_hostcalls = allowed_hostcalls_for_actor(suite, actor);
    let executor_preflight = executor_preflight_value(actor, &allowed_hostcalls)?;
    let preflight_ref = canonical_hash(&executor_preflight)?;
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
