
fn handler_profile_admission_receipt_value(input: &HandlerProfileAdmissionValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    require_ref(input.manifest_ref, "handler profile admission manifest ref")?;
    require_ref(input.handler_profile_ref, "handler profile admission profile ref")?;
    validate_handler_profile(input.handler_profile)?;
    validate_declared_effects(input.supported_effects)?;
    validate_effect_determinism_class(input.determinism_class)?;
    validate_effect_replay_class(input.replay_class)?;
    require_ref(input.policy_ref, "handler profile admission policy ref")?;
    require_ref(input.capability_context_ref, "handler profile admission capability context ref")?;
    validate_refs(input.resource_refs, "handler profile admission resource ref")?;
    validate_refs(input.evidence_refs, "handler profile admission evidence ref")?;
    Ok(record("handler-profile-admission-receipt-v1", vec![
        string(HANDLER_PROFILE_ADMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("handler-profile", vec![string(input.handler_profile_ref), string(input.handler_profile)]),
        record("supported-effects", vec![sequence(
            input.supported_effects.iter().map(declared_effect_value).collect(),
        )]),
        record("determinism", vec![string(input.determinism_class)]),
        record("replay", vec![string(input.replay_class)]),
        record("policy", vec![string(input.policy_ref), string(input.capability_context_ref)]),
        refs_record("resources", input.resource_refs),
        diagnostics_record(input.diagnostics),
        refs_record("evidence", input.evidence_refs),
        handler_profile_admission_checks_value(),
    ]))
}

fn effect_profile_replay_binding_value(input: &EffectProfileReplayBindingValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_effect_profile_integration_kind(input.integration_kind)?;
    require_ref(input.subject_ref, "effect profile replay subject ref")?;
    require_ref(input.effect_manifest_ref, "effect profile replay manifest ref")?;
    require_ref(input.handler_profile_ref, "effect profile replay handler profile ref")?;
    require_ref(input.profile_admission_ref, "effect profile replay admission ref")?;
    if let Some(compatibility_ref) = input.compatibility_ref {
        require_ref(compatibility_ref, "effect profile replay compatibility ref")?;
    }
    validate_refs(input.evidence_refs, "effect profile replay evidence ref")?;
    Ok(record("effect-profile-replay-binding-v1", vec![
        string(EFFECT_PROFILE_REPLAY_BINDING_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("integration", vec![string(input.integration_kind)]),
        record("subject", vec![string(input.subject_ref)]),
        record("effect-manifest", vec![string(input.effect_manifest_ref)]),
        record("handler-profile", vec![string(input.handler_profile_ref)]),
        record("profile-admission", vec![string(input.profile_admission_ref)]),
        record("compatibility", vec![optional_ref_value(input.compatibility_ref)]),
        diagnostics_record(input.diagnostics),
        refs_record("evidence", input.evidence_refs),
        effect_profile_replay_binding_checks_value(),
    ]))
}

fn handler_profile_admission_diagnostics(
    manifest: &EffectManifest,
    profile: &HandlerProfile,
    input: &HandlerProfileAdmissionInput,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if profile.policy_ref != input.current_policy_ref {
        diagnostics.push("handler profile admission policy ref is stale".to_string());
    }
    if profile.capability_context_ref != input.current_capability_context_ref {
        diagnostics.push("handler profile admission capability context is stale or revoked".to_string());
    }
    if profile.handler_binding_refs.is_empty() {
        diagnostics.push("handler profile admission requires handler binding refs".to_string());
    }
    if profile.resource_refs.is_empty() {
        diagnostics.push("handler profile admission requires resource bounds".to_string());
    }
    if input.evidence_refs.is_empty() {
        diagnostics.push("handler profile admission requires evidence refs".to_string());
    }
    diagnostics.extend(effect_support_diagnostics(manifest, &input.supported_effects));
    diagnostics
}

fn effect_support_diagnostics(manifest: &EffectManifest, supported_effects: &[DeclaredEffect]) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(manifest.declared_effects.len());
    for declared in &manifest.declared_effects {
        let mut is_saw_effect_operation = false;
        let mut is_saw_exact = false;
        for supported in supported_effects {
            if supported.effect_id == declared.effect_id && supported.operation == declared.operation {
                is_saw_effect_operation = true;
                if effect_support_matches(declared, supported) {
                    is_saw_exact = true;
                }
            }
        }
        if !is_saw_effect_operation {
            diagnostics.push(format!(
                "handler profile does not support declared effect {} operation {}",
                declared.effect_id, declared.operation
            ));
        } else if !is_saw_exact {
            diagnostics.push(format!(
                "handler profile schema/resource/capability mismatch for effect {} operation {}",
                declared.effect_id, declared.operation
            ));
        }
    }
    diagnostics
}

fn effect_support_matches(declared: &DeclaredEffect, supported: &DeclaredEffect) -> bool {
    declared.input_schema_ref == supported.input_schema_ref
        && declared.output_schema_ref == supported.output_schema_ref
        && declared.resource_class == supported.resource_class
        && declared.capability_refs == supported.capability_refs
}

fn declared_effect_for_request<'a>(manifest: &'a EffectManifest, request: &EffectRequest) -> Option<&'a DeclaredEffect> {
    manifest
        .declared_effects
        .iter()
        .find(|effect| effect.effect_id == request.effect_id && effect.operation == request.operation)
}

fn missing_capability_diagnostics(effect: &DeclaredEffect, request: &EffectRequest) -> Vec<String> {
    effect
        .capability_refs
        .iter()
        .filter(|required| !request.capability_refs.iter().any(|candidate| candidate == *required))
        .map(|_| {
            format!(
                "effect request missing required capability for effect {} operation {}",
                effect.effect_id, effect.operation
            )
        })
        .collect()
}

fn effect_profile_replay_binding_diagnostics(input: &EffectProfileReplayBindingInput) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let is_compatible = input.compatibility_ref.is_some();
    if let Some(expected_manifest_ref) = input.expected_manifest_ref.as_deref()
        && expected_manifest_ref != input.effect_manifest_ref
        && !is_compatible
    {
        diagnostics.push("effect profile binding manifest ref changed without compatibility evidence".to_string());
    }
    if let Some(expected_handler_profile_ref) = input.expected_handler_profile_ref.as_deref()
        && expected_handler_profile_ref != input.handler_profile_ref
        && !is_compatible
    {
        diagnostics.push("effect profile binding handler profile ref changed without compatibility evidence".to_string());
    }
    diagnostics
}

fn parse_supported_effect_count(value: &Value<IoValue>) -> Result<usize> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, "supported-effects", 1)?;
    let sequence = required_sequence(&record[0], "supported effects")?;
    for effect in sequence.iter() {
        let effect = value_to_iovalue(effect);
        declared_effect_fields(&effect)?;
    }
    Ok(sequence.len())
}

fn handler_profile_admission_checks_value() -> IoValue {
    checks_value(&[
        "handler-profile-admission-receipt",
        "effect-manifest-bound",
        "handler-profile-bound",
        "operation-schema-bound",
        "resource-bounds-bound",
        "capability-context-bound",
        "unison-prior-art-only",
    ])
}

fn handler_profile_admission_check_names() -> Vec<String> {
    vec![
        "handler-profile-admission-receipt".to_string(),
        "effect-manifest-bound".to_string(),
        "handler-profile-bound".to_string(),
        "operation-schema-bound".to_string(),
        "resource-bounds-bound".to_string(),
        "capability-context-bound".to_string(),
        "unison-prior-art-only".to_string(),
    ]
}

fn effect_profile_replay_binding_checks_value() -> IoValue {
    checks_value(&[
        "effect-manifest-ref-bound",
        "handler-profile-ref-bound",
        "profile-admission-ref-bound",
        "profile-change-denies-without-compatibility",
    ])
}

fn effect_profile_replay_binding_check_names() -> Vec<String> {
    vec![
        "effect-manifest-ref-bound".to_string(),
        "handler-profile-ref-bound".to_string(),
        "profile-admission-ref-bound".to_string(),
        "profile-change-denies-without-compatibility".to_string(),
    ]
}

fn validate_effect_determinism_class(value: &str) -> Result<()> {
    match value {
        EFFECT_DETERMINISM_DETERMINISTIC | EFFECT_DETERMINISM_NONDETERMINISTIC => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect determinism class {value}"))),
    }
}

fn validate_effect_replay_class(value: &str) -> Result<()> {
    match value {
        EFFECT_REPLAY_CLASS_RECORDED | EFFECT_REPLAY_CLASS_RECORD_REQUIRED | EFFECT_REPLAY_CLASS_COMPATIBLE => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect replay class {value}"))),
    }
}

fn validate_effect_profile_integration_kind(value: &str) -> Result<()> {
    match value {
        EFFECT_PROFILE_INTEGRATION_REPLAY
        | EFFECT_PROFILE_INTEGRATION_TRANSCRIPT
        | EFFECT_PROFILE_INTEGRATION_EVAL_CACHE
        | EFFECT_PROFILE_INTEGRATION_JOB_DAG
        | EFFECT_PROFILE_INTEGRATION_REMOTE_EXECUTION => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported effect profile integration kind {value}"))),
    }
}
