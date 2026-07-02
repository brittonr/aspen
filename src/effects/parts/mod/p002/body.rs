
pub fn parse_effect_binding_receipt(value: &IoValue) -> Result<EffectBindingReceipt> {
    let fields = simple_record(value, "effect-binding-receipt-v1", 9)?;
    require_schema(&fields[0], EFFECT_BINDING_RECEIPT_SCHEMA, "effect binding receipt schema")?;
    let decision = required_record_string(&fields[1], "decision", "effect binding decision")?;
    validate_decision(&decision)?;
    let handler_profile = value_to_iovalue(&fields[3]);
    let handler_profile = simple_record(&handler_profile, "handler-profile", 2)?;
    let effect = value_to_iovalue(&fields[5]);
    let effect = simple_record(&effect, "effect", 2)?;
    let effect_id = required_string(&effect[0], "effect binding effect id")?;
    let operation = required_string(&effect[1], "effect binding operation")?;
    validate_effect_id(&effect_id)?;
    validate_operation(&operation)?;
    let profile = required_string(&handler_profile[1], "effect binding handler profile")?;
    validate_handler_profile(&profile)?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "deny-undeclared-effects", "effect binding receipt")?;
    Ok(EffectBindingReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        manifest_ref: required_record_ref(&fields[2], "manifest", "effect binding manifest ref")?,
        handler_profile_ref: required_ref(&handler_profile[0], "effect binding handler profile ref")?,
        request_ref: required_record_ref(&fields[4], "request", "effect binding request ref")?,
        effect_id,
        operation,
        handler_profile: profile,
        diagnostics: parse_string_sequence_record_unvalidated(&fields[6], "diagnostics")?,
        evidence_refs: parse_ref_sequence_record(&fields[7], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn admit_effect_request(
    manifest_value: &IoValue,
    handler_profile_value: &IoValue,
    request_value: &IoValue,
    evidence_refs: &[String],
) -> Result<EffectBindingReceipt> {
    let manifest = parse_effect_manifest(manifest_value)?;
    let handler_profile = parse_handler_profile(handler_profile_value)?;
    let request = parse_effect_request(request_value)?;
    let mut diagnostics = Vec::new();
    if request.artifact_ref != manifest.artifact_ref {
        diagnostics.push("request artifact does not match manifest artifact".to_string());
    }
    if request.handler_profile != handler_profile.profile {
        diagnostics.push("request handler profile does not match admitted profile".to_string());
    }
    if !manifest
        .declared_effects
        .iter()
        .any(|effect| effect.effect_id == request.effect_id && effect.operation == request.operation)
    {
        diagnostics.push("effect id or operation is not declared by artifact manifest".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = effect_binding_receipt_value(&EffectBindingReceiptInput {
        decision: decision.to_string(),
        manifest_ref: manifest.manifest_ref,
        handler_profile_ref: handler_profile.profile_ref,
        request_ref: request.request_ref,
        effect_id: request.effect_id,
        operation: request.operation,
        handler_profile: request.handler_profile,
        diagnostics,
        evidence_refs: evidence_refs.to_vec(),
    })?;
    parse_effect_binding_receipt(&receipt_value)
}

pub fn handler_binding_value(input: &HandlerBindingInput) -> Result<IoValue> {
    validate_non_empty(&input.profile, "handler profile")?;
    validate_non_empty(&input.adapter_kind, "handler adapter kind")?;
    require_ref(&input.adapter_ref, "handler adapter ref")?;
    if let Some(executor_preflight_ref) = input.executor_preflight_ref.as_deref() {
        require_ref(executor_preflight_ref, "handler executor preflight ref")?;
    }
    require_ref(&input.policy_ref, "handler policy ref")?;
    require_ref(&input.capability_context_ref, "handler capability context ref")?;
    if let Some(context_ref) = input.context_ref.as_deref() {
        require_ref(context_ref, "handler authority context ref")?;
    }
    validate_scope(&input.scope)?;
    validate_refs(&input.resource_refs, "handler resource ref")?;
    validate_operations(&input.operations)?;
    validate_refs(&input.evidence_refs, "handler evidence ref")?;
    Ok(record("handler-binding-v1", vec![
        string(EFFECT_HANDLER_BINDING_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        scope_value(&input.scope),
        record("implementation", vec![
            string(&input.adapter_kind),
            string(&input.adapter_ref),
            optional_ref_value(input.executor_preflight_ref.as_deref()),
        ]),
        record("policy", vec![
            string(&input.policy_ref),
            string(&input.capability_context_ref),
            optional_ref_value(input.context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        operations_record(&input.operations),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "deny-ambient-effects",
            "handler-binding-available",
            "policy-capability-resource-binding",
            "bluefin-non-normative-prior-art",
        ]),
    ]))
}

pub fn effect_handle_value(input: &EffectHandleInput) -> Result<IoValue> {
    validate_non_empty(&input.kind, "effect handle kind")?;
    validate_scope(&input.scope)?;
    require_ref(&input.handler_binding_ref, "effect handle handler binding ref")?;
    validate_operations(&input.operations)?;
    require_ref(&input.capability_context_ref, "effect handle capability context ref")?;
    if let Some(context_ref) = input.context_ref.as_deref() {
        require_ref(context_ref, "effect handle authority context ref")?;
    }
    validate_refs(&input.resource_refs, "effect handle resource ref")?;
    if let (Some(not_before), Some(expires_at)) = (input.not_before, input.expires_at)
        && not_before > expires_at
    {
        return Err(MoltenError::invalid_harness("effect handle validity not-before exceeds expiry"));
    }
    validate_refs(&input.revocation_refs, "effect handle revocation ref")?;
    validate_transfer(&input.transfer)?;
    if let Some(parent_handle_ref) = input.parent_handle_ref.as_deref() {
        require_ref(parent_handle_ref, "effect handle parent ref")?;
    }
    validate_refs(&input.evidence_refs, "effect handle evidence ref")?;
    Ok(record("effect-handle-v1", vec![
        string(EFFECT_HANDLE_SCHEMA),
        record("kind", vec![string(&input.kind)]),
        scope_value(&input.scope),
        record("handler", vec![string(&input.handler_binding_ref)]),
        operations_record(&input.operations),
        record("authority", vec![
            string(&input.capability_context_ref),
            optional_ref_value(input.context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        record("validity", vec![
            optional_u64_value(input.not_before),
            optional_u64_value(input.expires_at),
            sequence(input.revocation_refs.iter().map(string).collect()),
        ]),
        record("transfer", vec![string(&input.transfer)]),
        record("parent", vec![optional_ref_value(input.parent_handle_ref.as_deref())]),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "handle-not-authority",
            "handler-scoped-handle",
            "operation-set-binding",
            "scope-lifetime-binding",
        ]),
    ]))
}

pub fn parse_handler_binding(value: &IoValue) -> Result<HandlerBinding> {
    let binding = simple_record(value, "handler-binding-v1", 9)?;
    require_schema(&binding[0], EFFECT_HANDLER_BINDING_SCHEMA, "handler binding schema")?;
    let implementation = value_to_iovalue(&binding[3]);
    let implementation = simple_record(&implementation, "implementation", 3)?;
    let policy = value_to_iovalue(&binding[4]);
    let policy = simple_record(&policy, "policy", 3)?;
    let checks = parse_checks(&binding[8])?;
    require_check(&checks, "deny-ambient-effects", "handler binding")?;
    require_check(&checks, "handler-binding-available", "handler binding")?;
    Ok(HandlerBinding {
        binding_ref: canonical_hash(value)?,
        profile: required_record_string(&binding[1], "profile", "handler binding profile")?,
        scope: parse_scope(&binding[2])?,
        adapter_kind: required_string(&implementation[0], "handler adapter kind")?,
        adapter_ref: required_ref(&implementation[1], "handler adapter ref")?,
        executor_preflight_ref: parse_optional_ref_value(&implementation[2])?,
        policy_ref: required_ref(&policy[0], "handler policy ref")?,
        capability_context_ref: required_ref(&policy[1], "handler capability context ref")?,
        context_ref: parse_optional_ref_value(&policy[2])?,
        resource_refs: parse_ref_sequence_record(&binding[5], "resources")?,
        operations: parse_string_sequence_record(&binding[6], "operations")?,
        evidence_refs: parse_ref_sequence_record(&binding[7], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn parse_effect_handle(value: &IoValue) -> Result<EffectHandle> {
    let handle = simple_record(value, "effect-handle-v1", 12)?;
    require_schema(&handle[0], EFFECT_HANDLE_SCHEMA, "effect handle schema")?;
    let authority = value_to_iovalue(&handle[5]);
    let authority = simple_record(&authority, "authority", 2)?;
    let validity = value_to_iovalue(&handle[7]);
    let validity = simple_record(&validity, "validity", 3)?;
    let revocation_values = required_sequence(&validity[2], "effect handle revocations")?;
    let mut revocation_refs = Vec::with_capacity(revocation_values.len());
    for revocation in revocation_values.iter() {
        revocation_refs.push(required_ref(revocation, "effect handle revocation ref")?);
    }
    let checks = parse_checks(&handle[11])?;
    require_check(&checks, "handle-not-authority", "effect handle")?;
    require_check(&checks, "handler-scoped-handle", "effect handle")?;
    let transfer = required_record_string(&handle[8], "transfer", "effect handle transfer")?;
    validate_transfer(&transfer)?;
    let not_before = parse_optional_u64_value(&validity[0])?;
    let expires_at = parse_optional_u64_value(&validity[1])?;
    if let (Some(not_before), Some(expires_at)) = (not_before, expires_at)
        && not_before > expires_at
    {
        return Err(MoltenError::invalid_harness("effect handle validity not-before exceeds expiry"));
    }
    Ok(EffectHandle {
        handle_ref: canonical_hash(value)?,
        kind: required_record_string(&handle[1], "kind", "effect handle kind")?,
        scope: parse_scope(&handle[2])?,
        handler_binding_ref: required_record_ref(&handle[3], "handler", "effect handle handler ref")?,
        operations: parse_string_sequence_record(&handle[4], "operations")?,
        capability_context_ref: required_ref(&authority[0], "effect handle capability context ref")?,
        context_ref: parse_optional_ref_value(&authority[1])?,
        resource_refs: parse_ref_sequence_record(&handle[6], "resources")?,
        not_before,
        expires_at,
        revocation_refs,
        transfer,
        parent_handle_ref: parse_optional_ref_record(&handle[9], "parent")?,
        evidence_refs: parse_ref_sequence_record(&handle[10], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn validate_handle_for_request(
    handler_value: &IoValue,
    handle_value: &IoValue,
    request: &EffectHandleRequest<'_>,
) -> Result<EffectHandleValidation> {
    let handler = parse_handler_binding(handler_value)?;
    let handle = parse_effect_handle(handle_value)?;
    validate_parsed_handle_for_request(&handler, &handle, request)
}

pub fn compound_handler_profile_value(input: &CompoundHandlerProfileInput) -> Result<IoValue> {
    validate_non_empty(&input.profile, "compound handler profile")?;
    validate_scope(&input.scope)?;
    validate_refs(&input.handler_binding_refs, "compound handler binding ref")?;
    validate_refs(&input.child_handle_refs, "compound child handle ref")?;
    validate_unique_refs(&input.handler_binding_refs, "compound handler binding ref")?;
    validate_unique_refs(&input.child_handle_refs, "compound child handle ref")?;
    if input.child_handle_refs.is_empty() {
        return Err(MoltenError::invalid_harness("compound handler profile must expose at least one child handle"));
    }
    require_ref(&input.policy_ref, "compound handler policy ref")?;
    require_ref(&input.capability_context_ref, "compound handler capability context ref")?;
    if let Some(context_ref) = input.context_ref.as_deref() {
        require_ref(context_ref, "compound handler authority context ref")?;
    }
    validate_refs(&input.resource_refs, "compound handler resource ref")?;
    validate_refs(&input.evidence_refs, "compound handler evidence ref")?;
    Ok(record("compound-handler-profile-v1", vec![
        string(EFFECT_COMPOUND_HANDLER_SCHEMA),
        record("profile", vec![string(&input.profile)]),
        scope_value(&input.scope),
        refs_record("handler-bindings", &input.handler_binding_refs),
        refs_record("child-handles", &input.child_handle_refs),
        record("policy", vec![
            string(&input.policy_ref),
            string(&input.capability_context_ref),
            optional_ref_value(input.context_ref.as_deref()),
        ]),
        refs_record("resources", &input.resource_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "compound-handler-profile",
            "child-handle-ref-binding",
            "shared-policy-capability-resource",
            "no-ambient-effects",
        ]),
    ]))
}
