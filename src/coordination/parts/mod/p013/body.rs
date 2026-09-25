
// r[impl molten.coordination_state_machine_proof.transition_receipt_binding]
fn finish_denial_transition(runtime: &mut CoordinationRuntime, input: DenialTransitionInput<'_>) -> Result<CoordinationApplyResult> {
    let DenialTransitionInput { request, snapshot, transition, extra_check, record_operation } = input;
    let mut checks = vec![
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("deny-before-side-effects", "pass"),
        ("primitive-transition-core", "pass"),
        ("preserved-state-bound", "pass"),
    ];
    checks.extend(transition.checks.iter().copied());
    checks.push((extra_check[0], extra_check[1]));
    let output_refs = transition_output_refs(&transition.output_facts)?;
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision: "deny",
        service: &request.service,
        operation: &request.operation,
        read_consistency_mode: &request.read_consistency_mode,
        request_ref: &request.request_ref,
        raft_receipt_ref: None,
        token_ref: None,
        state_ref: &snapshot.state_ref,
        transition: ReceiptTransitionInput {
            kind: &transition.kind,
            before_state_ref: &snapshot.state_ref,
            after_state_ref: None,
            preserved_state_ref: Some(&snapshot.state_ref),
            output_refs: &output_refs,
            control_plane_intent_ref: None,
            prior_receipt_ref: None,
        },
        dataspace_assertion_refs: &[],
        diagnostics: &transition.diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_coordination_receipt(&receipt_value)?;
    let evidence_values = evidence_values_for(EvidenceValuesInput {
        request: &request,
        receipt: &receipt,
        token: None,
        snapshot: &snapshot,
        assertions: &[],
        read: None,
    });
    let result = CoordinationApplyResult {
        receipt: receipt.clone(),
        request: request.clone(),
        token: None,
        state_snapshot: snapshot,
        assertions: Vec::new(),
        raft_commit_ref: None,
        raft_read_receipt: None,
        evidence_values,
    };
    runtime.receipts.push(receipt);
    if record_operation {
        runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    }
    Ok(result)
}

// r[impl molten.coordination_state_machine_proof.replay_transition_kind]
fn replay_or_conflicting_duplicate(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    existing: CoordinationApplyResult,
) -> Result<CoordinationApplyResult> {
    if request.request_ref == existing.request.request_ref {
        return duplicate_replay_result(runtime, request, snapshot, existing);
    }
    let diagnostic = format!(
        "conflicting duplicate operation id {} previously bound request {}",
        request.operation_id_ref, existing.request.request_ref
    );
    let transition = PrimitiveTransitionResult {
        kind: TRANSITION_KIND_CONFLICTING_DUPLICATE.to_string(),
        decision: "deny".to_string(),
        before_state: runtime.state.clone(),
        after_state: runtime.state.clone(),
        token: None,
        status_fact: status_fact_for(&runtime.state, &runtime.manifest, &request.service, &request.key)?,
        output_facts: vec![existing.receipt.value.clone()],
        diagnostics: vec![diagnostic],
        checks: vec![("duplicate-conflict-denied", "pass")],
        shell_intents: vec![SHELL_INTENT_EMIT_RECEIPT.to_string()],
    };
    finish_denial_transition(runtime, DenialTransitionInput { request, snapshot, transition, extra_check: &["conflicting-duplicate-operation", "fail"], record_operation: false })
}

// r[impl molten.coordination_state_machine_proof.replay_transition_kind]
fn duplicate_replay_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    existing: CoordinationApplyResult,
) -> Result<CoordinationApplyResult> {
    let output_refs = duplicate_output_refs(&existing)?;
    let diagnostics = vec![format!(
        "duplicate operation replay returned prior receipt {}",
        existing.receipt.receipt_ref
    )];
    let checks = [
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("idempotency-bound", "pass"),
        ("duplicate-replay-no-advance", "pass"),
        ("preserved-state-bound", "pass"),
        ("primitive-transition-core", "pass"),
        (SHELL_INTENT_REPLAY_OUTPUT, "pass"),
    ];
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision: &existing.receipt.decision,
        service: &request.service,
        operation: &request.operation,
        read_consistency_mode: &request.read_consistency_mode,
        request_ref: &request.request_ref,
        raft_receipt_ref: None,
        token_ref: existing.receipt.token_ref.as_deref(),
        state_ref: &snapshot.state_ref,
        transition: ReceiptTransitionInput {
            kind: TRANSITION_KIND_DUPLICATE_REPLAY,
            before_state_ref: &snapshot.state_ref,
            after_state_ref: None,
            preserved_state_ref: Some(&snapshot.state_ref),
            output_refs: &output_refs,
            control_plane_intent_ref: None,
            prior_receipt_ref: Some(&existing.receipt.receipt_ref),
        },
        dataspace_assertion_refs: &[],
        diagnostics: &diagnostics,
        checks: &checks,
    })?;
    let receipt = parse_coordination_receipt(&receipt_value)?;
    let mut evidence_values = evidence_values_for(EvidenceValuesInput {
        request: &request,
        receipt: &receipt,
        token: existing.token.as_ref(),
        snapshot: &snapshot,
        assertions: &[],
        read: None,
    });
    evidence_values.push(existing.receipt.value.clone());
    evidence_values.extend(existing.assertions.iter().map(|assertion| assertion.value.clone()));
    let result = CoordinationApplyResult {
        receipt: receipt.clone(),
        request,
        token: existing.token,
        state_snapshot: snapshot,
        assertions: Vec::new(),
        raft_commit_ref: None,
        raft_read_receipt: None,
        evidence_values,
    };
    runtime.receipts.push(receipt);
    Ok(result)
}

fn duplicate_output_refs(existing: &CoordinationApplyResult) -> Result<Vec<String>> {
    let mut refs = vec![existing.receipt.receipt_ref.clone()];
    if let Some(token) = &existing.token {
        refs.push_limited(token.token_ref.clone(), MAX_COORDINATION_REFS, "coordination duplicate output refs")?;
    }
    for assertion in &existing.assertions {
        refs.push_limited(
            assertion.assertion_ref.clone(),
            MAX_COORDINATION_REFS,
            "coordination duplicate output refs",
        )?;
    }
    Ok(refs)
}

// r[impl molten.coordination_state_machine_proof.primitive_transition_cores]
fn primitive_transition(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PrimitiveTransitionResult> {
    match prepare_mutation(runtime, request) {
        Ok(prepared) => Ok(PrimitiveTransitionResult {
            kind: TRANSITION_KIND_ADVANCE.to_string(),
            decision: "pass".to_string(),
            before_state: runtime.state.clone(),
            after_state: prepared.state,
            token: prepared.token,
            status_fact: prepared.status_fact.clone(),
            output_facts: vec![prepared.status_fact],
            diagnostics: Vec::new(),
            checks: prepared.checks,
            shell_intents: vec![SHELL_INTENT_COMMIT.to_string(), SHELL_INTENT_ASSERT_STATUS.to_string()],
        }),
        Err(error) => primitive_denial_transition(runtime, request, vec![error.to_string()]),
    }
}

fn primitive_denial_transition(
    runtime: &CoordinationRuntime,
    request: &CoordinationRequest,
    diagnostics: Vec<String>,
) -> Result<PrimitiveTransitionResult> {
    let status_fact = status_fact_for(&runtime.state, &runtime.manifest, &request.service, &request.key)?;
    Ok(PrimitiveTransitionResult {
        kind: TRANSITION_KIND_DENY_PRESERVE.to_string(),
        decision: "deny".to_string(),
        before_state: runtime.state.clone(),
        after_state: runtime.state.clone(),
        token: None,
        status_fact: status_fact.clone(),
        output_facts: vec![status_fact],
        diagnostics,
        checks: vec![("semantic-preserved-state", "pass")],
        shell_intents: vec![SHELL_INTENT_EMIT_RECEIPT.to_string()],
    })
}

fn prepare_mutation(runtime: &CoordinationRuntime, request: &CoordinationRequest) -> Result<PreparedMutation> {
    match (request.service.as_str(), request.operation.as_str()) {
        (SERVICE_LOCK, OP_ACQUIRE) => prepare_lock_acquire(runtime, request),
        (SERVICE_LOCK, OP_RELEASE) => prepare_lock_release(runtime, request),
        (SERVICE_QUEUE, OP_ENQUEUE) => prepare_queue_enqueue(runtime, request),
        (SERVICE_QUEUE, OP_DEQUEUE) => prepare_queue_dequeue(runtime, request),
        (SERVICE_SEMAPHORE, OP_ACQUIRE) => prepare_semaphore_acquire(runtime, request),
        (SERVICE_SEMAPHORE, OP_RELEASE) => prepare_semaphore_release(runtime, request),
        (SERVICE_RATE_LIMIT, OP_ACQUIRE) => prepare_rate_acquire(runtime, request),
        (SERVICE_ELECTION, OP_ELECT) => prepare_election(runtime, request),
        (SERVICE_BARRIER, OP_ARRIVE) => prepare_barrier(runtime, request),
        (SERVICE_REGISTRY, OP_REGISTER) => prepare_registry_register(runtime, request),
        (SERVICE_REGISTRY, OP_UNREGISTER) => prepare_registry_unregister(runtime, request),
        _ => Err(MoltenError::invalid_harness("unsupported coordination mutation")),
    }
}
