
fn propose_change(input: ChangeInput<'_>) -> Result<Proposal> {
    let ChangeInput {
        runtime,
        request,
        snapshot,
    } = input;
    let command = crate::raft_control_plane::control_registry_command_value(
        &crate::raft_control_plane::ControlRegistryCommandInput {
            operation: "set-coordination-state".to_string(),
            namespace: coordination_namespace(&request.service),
            name: request.key.clone(),
            target_ref: Some(snapshot.state_ref.clone()),
        },
    )?;
    let evidence_refs = vec![
        request.request_ref.clone(),
        request.operation_id_ref.clone(),
        snapshot.state_ref.clone(),
    ];
    let envelope =
        crate::raft_control_plane::raft_command_envelope_value(&crate::raft_control_plane::RaftCommandEnvelopeInput {
            group_ref: runtime.raft.manifest.manifest_ref.clone(),
            client_session: request.client_session.clone(),
            sequence: runtime.next_sequence,
            command,
            authority_refs: request.authority_refs.clone(),
            policy_refs: request.policy_refs.clone(),
            resource_refs: request.resource_refs.clone(),
            evidence_refs,
        })?;
    crate::raft_control_plane::propose_control_registry_command(&mut runtime.raft, &envelope)
}

fn fact_for(
    prepared: &PreparedMutation,
    manifest: &CoordinationServiceManifest,
    request: &CoordinationRequest,
    token: Option<&FencingToken>,
) -> Result<IoValue> {
    if let Some(token) = token {
        status_fact_for_token(&prepared.state, manifest, &request.service, &request.key, token)
    } else {
        Ok(prepared.status_fact.clone())
    }
}

fn status_assertion_for(
    request: &CoordinationRequest,
    fact: &IoValue,
    state_ref: &str,
    receipt_ref: &str,
) -> Result<CoordinationStatusAssertion> {
    let value = coordination_status_assertion_value(StatusAssertionInput {
        service: &request.service,
        key: &request.key,
        fact,
        state_ref,
        receipt_ref,
    })?;
    parse_coordination_status_assertion(&value)
}

fn pass_checks(prepared: &PreparedMutation) -> Vec<(&'static str, &'static str)> {
    let mut checks = vec![
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("commit-receipt-bound", "pass"),
        ("idempotency-bound", "pass"),
        ("authority-policy-resource", "pass"),
        ("dataspace-reflection-after-commit", "pass"),
    ];
    checks.extend(prepared.checks.iter().copied());
    checks
}

fn pass_receipt(input: PassReceiptInput<'_>) -> Result<CoordinationReceipt> {
    let PassReceiptInput {
        request,
        proposal_ref,
        token_ref,
        state_ref,
        assertion_refs,
        checks,
    } = input;
    let value = coordination_receipt_value(ReceiptValueInput {
        decision: "pass",
        service: &request.service,
        operation: &request.operation,
        request_ref: &request.request_ref,
        raft_receipt_ref: Some(proposal_ref),
        token_ref,
        state_ref,
        dataspace_assertion_refs: assertion_refs,
        diagnostics: &[],
        checks,
    })?;
    parse_coordination_receipt(&value)
}

fn success_parts(input: PartsInput<'_>) -> Result<SuccessParts> {
    let token = materialize_token(input.prepared.token.clone(), input.proposal_ref)?;
    let token_ref = token.as_ref().map(|item| item.token_ref.clone());
    let fact = fact_for(input.prepared, input.manifest, input.request, token.as_ref())?;
    let placeholder_receipt_ref = fixture_ref("coordination-mutation-placeholder");
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &placeholder_receipt_ref)?;
    let assertion_refs = vec![assertion.assertion_ref.clone()];
    let checks = pass_checks(input.prepared);
    let receipt = pass_receipt(PassReceiptInput {
        request: input.request,
        proposal_ref: input.proposal_ref,
        token_ref: token_ref.as_deref(),
        state_ref: &input.snapshot.state_ref,
        assertion_refs: &assertion_refs,
        checks: &checks,
    })?;
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &receipt.receipt_ref)?;
    Ok(SuccessParts {
        token,
        receipt,
        assertion,
    })
}

fn success_values(input: ValuesInput<'_>) -> Vec<IoValue> {
    let ValuesInput {
        proposal,
        request,
        receipt,
        token,
        snapshot,
        assertion,
    } = input;
    let mut evidence_values = vec![
        proposal.envelope.value.clone(),
        proposal.commit_receipt.value.clone(),
        proposal.registry_receipt.value.clone(),
    ];
    if let Some(log_entry) = &proposal.log_entry {
        evidence_values.push(log_entry.value.clone());
    }
    evidence_values.extend(proposal.predicates.iter().map(|value| value.value.clone()));
    evidence_values.extend(evidence_values_for(EvidenceValuesInput {
        request,
        receipt,
        token,
        snapshot,
        assertions: std::slice::from_ref(assertion),
        read: None,
    }));
    evidence_values
}

fn record_success(input: SuccessInput<'_>) -> Result<CoordinationApplyResult> {
    let SuccessInput {
        runtime,
        request,
        prepared,
        snapshot,
        proposal,
    } = input;
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    let parts = success_parts(PartsInput {
        prepared: &prepared,
        request: &request,
        manifest: &runtime.manifest,
        snapshot: &snapshot,
        proposal_ref: &proposal.commit_receipt.receipt_ref,
    })?;
    runtime.state = prepared.state;
    let evidence_values = success_values(ValuesInput {
        proposal: &proposal,
        request: &request,
        receipt: &parts.receipt,
        token: parts.token.as_ref(),
        snapshot: &snapshot,
        assertion: &parts.assertion,
    });
    if let Some(token) = &parts.token {
        runtime.tokens.push(token.clone());
    }
    runtime.receipts.push(parts.receipt.clone());
    runtime.assertions.push(parts.assertion.clone());
    let result = CoordinationApplyResult {
        receipt: parts.receipt.clone(),
        request: request.clone(),
        token: parts.token,
        state_snapshot: snapshot,
        assertions: vec![parts.assertion],
        raft_commit_ref: Some(proposal.commit_receipt.receipt_ref),
        raft_read_receipt: None,
        evidence_values,
    };
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    Ok(result)
}

fn commit_prepared_mutation(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    prepared: PreparedMutation,
) -> Result<CoordinationApplyResult> {
    let snapshot = snapshot_from_state(&prepared.state)?;
    let proposal = propose_change(ChangeInput {
        runtime,
        request: &request,
        snapshot: &snapshot,
    })?;
    if proposal.decision != "pass" {
        let diagnostics = vec!["control-plane commit denied for coordination mutation".to_string()];
        return deny_result(runtime, request, snapshot, diagnostics, &["control-plane-commit", "fail"]);
    }
    record_success(SuccessInput {
        runtime,
        request,
        prepared,
        snapshot,
        proposal,
    })
}

fn deny_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    diagnostics: Vec<String>,
    extra_check: &[&'static str; 2],
) -> Result<CoordinationApplyResult> {
    let checks = [
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("deny-before-side-effects", "pass"),
        (extra_check[0], extra_check[1]),
    ];
    let receipt_value = coordination_receipt_value(ReceiptValueInput {
        decision: "deny",
        service: &request.service,
        operation: &request.operation,
        request_ref: &request.request_ref,
        raft_receipt_ref: None,
        token_ref: None,
        state_ref: &snapshot.state_ref,
        dataspace_assertion_refs: &[],
        diagnostics: &diagnostics,
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
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    Ok(result)
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
