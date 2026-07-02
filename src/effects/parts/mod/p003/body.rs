
pub fn parse_compound_handler_profile(value: &IoValue) -> Result<CompoundHandlerProfile> {
    let profile = simple_record(value, "compound-handler-profile-v1", 9)?;
    require_schema(&profile[0], EFFECT_COMPOUND_HANDLER_SCHEMA, "compound handler schema")?;
    let policy = value_to_iovalue(&profile[5]);
    let policy = simple_record(&policy, "policy", 3)?;
    let handler_binding_refs = parse_ref_sequence_record(&profile[3], "handler-bindings")?;
    let child_handle_refs = parse_ref_sequence_record(&profile[4], "child-handles")?;
    validate_unique_refs(&handler_binding_refs, "compound handler binding ref")?;
    validate_unique_refs(&child_handle_refs, "compound child handle ref")?;
    if child_handle_refs.is_empty() {
        return Err(MoltenError::invalid_harness("compound handler profile must expose at least one child handle"));
    }
    let checks = parse_checks(&profile[8])?;
    require_check(&checks, "compound-handler-profile", "compound handler profile")?;
    require_check(&checks, "child-handle-ref-binding", "compound handler profile")?;
    Ok(CompoundHandlerProfile {
        profile_ref: canonical_hash(value)?,
        profile: required_record_string(&profile[1], "profile", "compound handler profile")?,
        scope: parse_scope(&profile[2])?,
        handler_binding_refs,
        child_handle_refs,
        policy_ref: required_ref(&policy[0], "compound handler policy ref")?,
        capability_context_ref: required_ref(&policy[1], "compound handler capability context ref")?,
        context_ref: parse_optional_ref_value(&policy[2])?,
        resource_refs: parse_ref_sequence_record(&profile[6], "resources")?,
        evidence_refs: parse_ref_sequence_record(&profile[7], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn dynamic_operation_record_value(input: &DynamicOperationRecordInput) -> Result<IoValue> {
    validate_operation(&input.operation)?;
    require_ref(&input.adapter_ref, "dynamic operation adapter ref")?;
    require_ref(&input.callable_ref, "dynamic operation callable ref")?;
    require_ref(&input.request_ref, "dynamic operation request ref")?;
    require_ref(&input.response_ref, "dynamic operation response ref")?;
    require_ref(&input.policy_ref, "dynamic operation policy ref")?;
    require_ref(&input.capability_context_ref, "dynamic operation capability context ref")?;
    validate_refs(&input.resource_refs, "dynamic operation resource ref")?;
    validate_refs(&input.evidence_refs, "dynamic operation evidence ref")?;
    Ok(record("dynamic-operation-v1", vec![
        string(EFFECT_DYNAMIC_OPERATION_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("adapter", vec![string(&input.adapter_ref)]),
        record("callable", vec![string(&input.callable_ref)]),
        record("request", vec![string(&input.request_ref)]),
        record("response", vec![string(&input.response_ref)]),
        record("policy", vec![string(&input.policy_ref), string(&input.capability_context_ref)]),
        refs_record("resources", &input.resource_refs),
        refs_record("evidence", &input.evidence_refs),
        checks_value(&[
            "reviewed-dynamic-operation",
            "canonical-request-response",
            "policy-capability-resource-binding",
        ]),
    ]))
}

pub fn parse_dynamic_operation_record(value: &IoValue) -> Result<DynamicOperationRecord> {
    let operation = simple_record(value, "dynamic-operation-v1", 10)?;
    require_schema(&operation[0], EFFECT_DYNAMIC_OPERATION_SCHEMA, "dynamic operation schema")?;
    let policy = value_to_iovalue(&operation[6]);
    let policy = simple_record(&policy, "policy", 2)?;
    let checks = parse_checks(&operation[9])?;
    require_check(&checks, "reviewed-dynamic-operation", "dynamic operation")?;
    require_check(&checks, "canonical-request-response", "dynamic operation")?;
    Ok(DynamicOperationRecord {
        record_ref: canonical_hash(value)?,
        operation: required_record_string(&operation[1], "operation", "dynamic operation name")?,
        adapter_ref: required_record_ref(&operation[2], "adapter", "dynamic operation adapter ref")?,
        callable_ref: required_record_ref(&operation[3], "callable", "dynamic operation callable ref")?,
        request_ref: required_record_ref(&operation[4], "request", "dynamic operation request ref")?,
        response_ref: required_record_ref(&operation[5], "response", "dynamic operation response ref")?,
        policy_ref: required_ref(&policy[0], "dynamic operation policy ref")?,
        capability_context_ref: required_ref(&policy[1], "dynamic operation capability context ref")?,
        resource_refs: parse_ref_sequence_record(&operation[7], "resources")?,
        evidence_refs: parse_ref_sequence_record(&operation[8], "evidence")?,
        checks,
        value: value.clone(),
    })
}

pub fn attenuated_handle_value(parent_handle_value: &IoValue, input: &HandleAttenuationInput) -> Result<IoValue> {
    let parent = parse_effect_handle(parent_handle_value)?;
    validate_scope_narrows(&parent.scope, &input.scope)?;
    validate_operation_subset(&parent.operations, &input.operations)?;
    validate_transfer_attenuation(&parent.transfer, &input.transfer)?;
    if let (Some(parent_expiry), Some(child_expiry)) = (parent.expires_at, input.expires_at)
        && child_expiry > parent_expiry
    {
        return Err(MoltenError::invalid_harness("attenuated effect handle expiry exceeds parent expiry"));
    }
    if parent.expires_at.is_some() && input.expires_at.is_none() {
        return Err(MoltenError::invalid_harness("attenuated effect handle cannot remove parent expiry"));
    }
    validate_refs(&input.evidence_refs, "attenuated handle evidence ref")?;
    effect_handle_value(&EffectHandleInput {
        kind: parent.kind,
        scope: input.scope.clone(),
        handler_binding_ref: parent.handler_binding_ref,
        operations: input.operations.clone(),
        capability_context_ref: parent.capability_context_ref,
        context_ref: parent.context_ref,
        resource_refs: parent.resource_refs,
        not_before: parent.not_before,
        expires_at: input.expires_at.or(parent.expires_at),
        revocation_refs: parent.revocation_refs,
        transfer: input.transfer.clone(),
        parent_handle_ref: Some(parent.handle_ref),
        evidence_refs: input.evidence_refs.clone(),
    })
}

pub fn handle_cleanup_receipt_value(
    handle_ref: &str,
    action: &str,
    live_usable: bool,
    preserve_artifact: bool,
    evidence_refs: &[String],
) -> Result<IoValue> {
    require_ref(handle_ref, "cleanup handle ref")?;
    validate_non_empty(action, "cleanup action")?;
    validate_refs(evidence_refs, "cleanup evidence ref")?;
    Ok(record("handle-cleanup-v1", vec![
        string(EFFECT_HANDLE_CLEANUP_SCHEMA),
        record("handle", vec![string(handle_ref)]),
        record("action", vec![string(action)]),
        record("live-usable", vec![crate::preserves_rail::bool_value(live_usable)]),
        record("preserve-artifact", vec![crate::preserves_rail::bool_value(preserve_artifact)]),
        refs_record("evidence", evidence_refs),
        checks_value(&[
            "live-usability-cleanup",
            "historical-artifact-preserved",
            "replay-evidence-retained",
        ]),
    ]))
}

pub fn parse_handle_cleanup_receipt(value: &IoValue) -> Result<HandleCleanupReceipt> {
    let receipt = simple_record(value, "handle-cleanup-v1", 7)?;
    require_schema(&receipt[0], EFFECT_HANDLE_CLEANUP_SCHEMA, "handle cleanup schema")?;
    let checks = parse_checks(&receipt[6])?;
    require_check(&checks, "live-usability-cleanup", "handle cleanup")?;
    require_check(&checks, "historical-artifact-preserved", "handle cleanup")?;
    let should_preserve_artifact = required_record_bool(&receipt[4], "preserve-artifact", "cleanup preserve artifact")?;
    if !should_preserve_artifact {
        return Err(MoltenError::invalid_harness("handle cleanup must preserve historical artifacts for replay"));
    }
    Ok(HandleCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        handle_ref: required_record_ref(&receipt[1], "handle", "cleanup handle ref")?,
        action: required_record_string(&receipt[2], "action", "cleanup action")?,
        live_usable: required_record_bool(&receipt[3], "live-usable", "cleanup live usability")?,
        preserve_artifact: should_preserve_artifact,
        evidence_refs: parse_ref_sequence_record(&receipt[5], "evidence")?,
        checks,
        value: value.clone(),
    })
}

fn validate_parsed_handle_for_request(
    handler: &HandlerBinding,
    handle: &EffectHandle,
    request: &EffectHandleRequest<'_>,
) -> Result<EffectHandleValidation> {
    require_binding_match(handler, handle, request)?;
    require_context_match(handler, handle, request)?;
    require_lifetime_match(handle, request)?;
    Ok(EffectHandleValidation {
        handler_binding_ref: handler.binding_ref.clone(),
        handle_ref: handle.handle_ref.clone(),
        checks: vec![
            "handler-binding-available".to_string(),
            "effect-handle-binding".to_string(),
            "handle-not-authority".to_string(),
            "operation-authorization-binding".to_string(),
            "scope-lifetime-binding".to_string(),
        ],
    })
}

fn require_binding_match(
    handler: &HandlerBinding,
    handle: &EffectHandle,
    request: &EffectHandleRequest<'_>,
) -> Result<()> {
    if handle.handler_binding_ref != handler.binding_ref {
        return Err(MoltenError::invalid_harness("effect handle does not bind the supplied handler binding"));
    }
    if handle.kind != request.kind {
        return Err(MoltenError::invalid_harness(format!(
            "effect handle kind mismatch: got {}, expected {}",
            handle.kind, request.kind
        )));
    }
    require_operation(&handler.operations, request.operation, "handler binding")?;
    require_operation(&handle.operations, request.operation, "effect handle")?;
    require_scope_match(&handler.scope, request, "handler binding")?;
    require_scope_match(&handle.scope, request, "effect handle")
}

fn require_context_match(
    handler: &HandlerBinding,
    handle: &EffectHandle,
    request: &EffectHandleRequest<'_>,
) -> Result<()> {
    if handler.policy_ref != request.policy_ref {
        return Err(MoltenError::invalid_harness("handler binding policy ref does not match request"));
    }
    if handler.capability_context_ref != request.capability_context_ref
        || handle.capability_context_ref != request.capability_context_ref
    {
        return Err(MoltenError::invalid_harness("effect handle capability context ref does not match request"));
    }
    if handler.context_ref.as_deref() != request.context_ref || handle.context_ref.as_deref() != request.context_ref {
        return Err(MoltenError::invalid_harness("effect handle authority context ref does not match request"));
    }
    if handler.resource_refs != request.resource_refs || handle.resource_refs != request.resource_refs {
        return Err(MoltenError::invalid_harness("effect handle resource refs do not match request"));
    }
    Ok(())
}

fn require_lifetime_match(handle: &EffectHandle, request: &EffectHandleRequest<'_>) -> Result<()> {
    if handle.not_before.is_some_and(|not_before| request.logical_time < not_before) {
        return Err(MoltenError::invalid_harness("effect handle used before not-before bound"));
    }
    if handle.expires_at.is_some_and(|expires_at| request.logical_time >= expires_at) {
        return Err(MoltenError::invalid_harness("effect handle expired before request"));
    }
    if request
        .revoked_refs
        .iter()
        .any(|revoked| handle.revocation_refs.iter().any(|handle_revoked| handle_revoked == revoked))
    {
        return Err(MoltenError::invalid_harness("effect handle revoked before request"));
    }
    if request.remote_use && handle.transfer == TRANSFER_LOCAL_ONLY {
        return Err(MoltenError::invalid_harness("local-only effect handle cannot be used remotely"));
    }
    if request.remote_use && handle.transfer == TRANSFER_REMOTE_PROXY {
        if handle.evidence_refs.len() < 3 {
            return Err(MoltenError::invalid_harness(
                "remote-proxy effect handle missing peer/node/revocation evidence refs",
            ));
        }
        if handle.resource_refs.is_empty() {
            return Err(MoltenError::invalid_harness("remote-proxy effect handle missing resource limits"));
        }
        if handle.expires_at.is_none() {
            return Err(MoltenError::invalid_harness("remote-proxy effect handle missing bounded expiry"));
        }
    }
    Ok(())
}

fn scope_value(scope: &EffectScope) -> IoValue {
    record("scope", vec![
        record("run", vec![string(&scope.run_ref)]),
        record("session", vec![string(&scope.session_ref)]),
        record("actor", vec![optional_ref_value(scope.actor_ref.as_deref())]),
        record("turn", vec![optional_ref_value(scope.turn_ref.as_deref())]),
    ])
}

fn parse_scope(value: &Value<IoValue>) -> Result<EffectScope> {
    let value = value_to_iovalue(value);
    let scope = simple_record(&value, "scope", 4)?;
    Ok(EffectScope {
        run_ref: required_record_ref(&scope[0], "run", "effect scope run ref")?,
        session_ref: required_record_ref(&scope[1], "session", "effect scope session ref")?,
        actor_ref: parse_optional_ref_record(&scope[2], "actor")?,
        turn_ref: parse_optional_ref_record(&scope[3], "turn")?,
    })
}

fn validate_scope(scope: &EffectScope) -> Result<()> {
    require_ref(&scope.run_ref, "effect scope run ref")?;
    require_ref(&scope.session_ref, "effect scope session ref")?;
    if let Some(actor_ref) = scope.actor_ref.as_deref() {
        require_ref(actor_ref, "effect scope actor ref")?;
    }
    if let Some(turn_ref) = scope.turn_ref.as_deref() {
        require_ref(turn_ref, "effect scope turn ref")?;
    }
    Ok(())
}
