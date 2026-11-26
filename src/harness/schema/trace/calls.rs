use preserves::ValueImpl;

pub fn actor_input_value(
    suite: &super::model::Suite,
    step: &crate::harness::nucleus::CoreStep,
    context: super::observed::HostcallEvidenceContext<'_>,
) -> crate::error::Result<preserves::IOValue> {
    let actor = step.primary_actor();
    let kind = super::intake::actor_kind_for_primary_actor(suite, actor)?;
    Ok(crate::preserves_rail::record("actor-input-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ACTOR_INPUT_SCHEMA),
        crate::preserves_rail::record("actor", vec![
            crate::preserves_rail::string(actor),
            crate::preserves_rail::string(kind.as_str()),
        ]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(context.sequence)]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(context.step_ref)]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(context.policy_ref)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(context.capability_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(context.budget_ref)]),
        crate::preserves_rail::record("input", vec![super::events::step_value(step)]),
        hostcall_checks_value(&["canonical-preserves", "actor-registry-binding", "executor-boundary"]),
    ]))
}

pub fn hostcall_request_value(
    suite: &super::model::Suite,
    step: &crate::harness::nucleus::CoreStep,
    context: super::observed::HostcallEvidenceContext<'_>,
    decision: &crate::runtime::AdmissionDecision,
) -> crate::error::Result<preserves::IOValue> {
    let request = crate::harness::nucleus::AdmissionRequest::from_step(step);
    let effect_refs = super::handles::hostcall_effect_refs(suite, step, context, decision.is_allowed())?;
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
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_HOSTCALL_REQUEST_SCHEMA),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(context.sequence)]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(context.step_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(request.action.as_str())]),
        super::events::admission_request_value(&request),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(context.policy_ref)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(context.capability_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(context.budget_ref)]),
        hostcall_checks_value(&checks),
        crate::preserves_rail::record("handler-binding-ref", vec![crate::preserves_rail::string(
            &effect_refs.handler_binding_ref,
        )]),
        crate::preserves_rail::record("handle-ref", vec![crate::preserves_rail::string(&effect_refs.handle_ref)]),
    ];
    if let Some(effect_manifest_ref) = &effect_refs.effect_manifest_ref {
        fields.push(crate::preserves_rail::record("effect-manifest-ref", vec![crate::preserves_rail::string(
            effect_manifest_ref,
        )]));
    }
    if let Some(handler_profile_ref) = &effect_refs.handler_profile_ref {
        fields.push(crate::preserves_rail::record("handler-profile-ref", vec![crate::preserves_rail::string(
            handler_profile_ref,
        )]));
    }
    if let Some(effect_request_ref) = &effect_refs.effect_request_ref {
        fields.push(crate::preserves_rail::record("effect-request-ref", vec![crate::preserves_rail::string(
            effect_request_ref,
        )]));
    }
    if let Some(effect_binding_receipt_ref) = &effect_refs.effect_binding_receipt_ref {
        fields.push(crate::preserves_rail::record("effect-binding-receipt-ref", vec![crate::preserves_rail::string(
            effect_binding_receipt_ref,
        )]));
    }
    Ok(crate::preserves_rail::record("hostcall-request-v1", fields))
}

pub fn validate_hostcall_effect_binding_request(
    hostcall_request: &preserves::IOValue,
    operation: &str,
) -> crate::error::Result<()> {
    let request = hostcall_request.collect_simple_record("hostcall-request-v1", Some(15)).ok_or_else(|| {
        crate::error::Failure::invalid_harness("executor hostcall gate requires bound effect request evidence")
    })?;
    let request_operation =
        super::fields::required_record_string(&request[3], "operation", "hostcall request operation")?;
    if request_operation != operation {
        return Err(crate::error::Failure::invalid_harness(format!(
            "executor hostcall gate operation mismatch: got {request_operation}, expected {operation}"
        )));
    }
    for (field, label) in [
        (&request[11], "effect-manifest-ref"),
        (&request[12], "handler-profile-ref"),
        (&request[13], "effect-request-ref"),
        (&request[14], "effect-binding-receipt-ref"),
    ] {
        let content_ref = super::fields::required_record_string(field, label, label)?;
        crate::preserves_rail::validate_content_ref(&content_ref)?;
    }
    Ok(())
}

pub fn hostcall_decision_value(
    context: super::observed::HostcallEvidenceContext<'_>,
    admission_event: &preserves::IOValue,
    authority: &super::observed::AdmissionAuthorityEvidence,
    decision: &crate::runtime::AdmissionDecision,
) -> crate::error::Result<preserves::IOValue> {
    let authority_value = super::events::admission_authority_value(authority);
    Ok(crate::preserves_rail::record("hostcall-decision-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_HOSTCALL_DECISION_SCHEMA),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(context.sequence)]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(context.step_ref)]),
        crate::preserves_rail::record("decision", vec![
            crate::preserves_rail::string(decision.status()),
            crate::preserves_rail::string(decision.reason()),
        ]),
        crate::preserves_rail::record("admission-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(admission_event)?,
        )]),
        crate::preserves_rail::record("authority-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&authority_value)?,
        )]),
        crate::preserves_rail::record("policy-ref", vec![crate::preserves_rail::string(context.policy_ref)]),
        crate::preserves_rail::record("capability-ref", vec![crate::preserves_rail::string(context.capability_ref)]),
        crate::preserves_rail::record("budget-ref", vec![crate::preserves_rail::string(context.budget_ref)]),
        hostcall_checks_value(&["admission-binding", "authority-binding", "budget-binding"]),
    ]))
}

pub fn actor_output_value(
    step: &crate::harness::nucleus::CoreStep,
    context: super::observed::HostcallEvidenceContext<'_>,
    decision: &crate::runtime::AdmissionDecision,
    runtime_events: &[preserves::IOValue],
) -> crate::error::Result<preserves::IOValue> {
    let runtime_events_value = crate::preserves_rail::sequence(runtime_events.to_vec());
    Ok(crate::preserves_rail::record("actor-output-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::RUNTIME_ACTOR_OUTPUT_SCHEMA),
        crate::preserves_rail::record("actor", vec![crate::preserves_rail::string(step.primary_actor())]),
        crate::preserves_rail::record("sequence", vec![crate::preserves_rail::u64_value(context.sequence)]),
        crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(context.step_ref)]),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(decision.status())]),
        crate::preserves_rail::record("events-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&runtime_events_value)?,
        )]),
        crate::preserves_rail::record("events", vec![crate::preserves_rail::u64_value(runtime_events.len() as u64)]),
        hostcall_checks_value(&["staged-output", "deterministic-trace"]),
    ]))
}

pub(super) fn hostcall_checks_value(checks: &[&str]) -> preserves::IOValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        checks
            .iter()
            .map(|name| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(*name),
                    crate::preserves_rail::string("pass"),
                ])
            })
            .collect(),
    )])
}
