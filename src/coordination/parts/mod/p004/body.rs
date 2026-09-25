
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

struct FactInput<'a> {
    transition: &'a PrimitiveTransitionResult,
    manifest: &'a CoordinationServiceManifest,
    engine_manifest: &'a crate::raft_control_plane::RaftGroupManifest,
    engine_epoch: u64,
    request: &'a CoordinationRequest,
    token: Option<&'a FencingToken>,
}

fn fact_for(input: FactInput<'_>) -> Result<IoValue> {
    let FactInput { transition, manifest, engine_manifest, engine_epoch, request, token } = input;
    let base = if let Some(token) = token {
        status_fact_for_token(&transition.after_state, manifest, &request.service, &request.key, token)?
    } else {
        transition.status_fact.clone()
    };
    Ok(engine_status_fact(engine_manifest, engine_epoch, &base))
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
        read_consistency_mode: &request.read_consistency_mode,
        fact,
        state_ref,
        receipt_ref,
    })?;
    parse_coordination_status_assertion(&value)
}

fn pass_checks(transition: &PrimitiveTransitionResult) -> Vec<(&'static str, &'static str)> {
    let mut checks = vec![
        ("coordination-request-bound", "pass"),
        ("control-plane-command", "pass"),
        ("commit-receipt-bound", "pass"),
        ("idempotency-bound", "pass"),
        ("authority-policy-resource", "pass"),
        ("dataspace-reflection-after-commit", "pass"),
        ("normalized-consensus-evidence", "pass"),
        ("active-engine-epoch-bound", "pass"),
        ("primitive-transition-core", "pass"),
        ("transition-kind-advance", "pass"),
    ];
    checks.extend(transition.checks.iter().copied());
    checks
}

fn pass_receipt(input: PassReceiptInput<'_>) -> Result<CoordinationReceipt> {
    let PassReceiptInput {
        request,
        proposal_ref,
        token_ref,
        before_state_ref,
        state_ref,
        assertion_refs,
        output_refs,
        checks,
    } = input;
    let value = coordination_receipt_value(ReceiptValueInput {
        decision: "pass",
        service: &request.service,
        operation: &request.operation,
        read_consistency_mode: &request.read_consistency_mode,
        request_ref: &request.request_ref,
        raft_receipt_ref: Some(proposal_ref),
        token_ref,
        state_ref,
        transition: ReceiptTransitionInput {
            kind: TRANSITION_KIND_ADVANCE,
            before_state_ref,
            after_state_ref: Some(state_ref),
            preserved_state_ref: None,
            output_refs,
            control_plane_intent_ref: Some(proposal_ref),
            prior_receipt_ref: None,
        },
        dataspace_assertion_refs: assertion_refs,
        diagnostics: &[],
        checks,
    })?;
    parse_coordination_receipt(&value)
}

fn success_parts(input: PartsInput<'_>) -> Result<SuccessParts> {
    let token = materialize_token(input.transition.token.clone(), input.proposal_ref)?;
    let token_ref = token.as_ref().map(|item| item.token_ref.clone());
    let fact = fact_for(FactInput { transition: input.transition, manifest: input.manifest, engine_manifest: input.engine_manifest, engine_epoch: input.engine_epoch, request: input.request, token: token.as_ref() })?;
    let placeholder_receipt_ref = fixture_ref("coordination-mutation-placeholder");
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &placeholder_receipt_ref)?;
    let assertion_refs = vec![assertion.assertion_ref.clone()];
    let output_refs = transition_output_refs(std::slice::from_ref(&fact))?;
    let checks = pass_checks(input.transition);
    let receipt = pass_receipt(PassReceiptInput {
        request: input.request,
        proposal_ref: input.proposal_ref,
        token_ref: token_ref.as_deref(),
        before_state_ref: &input.before_snapshot.state_ref,
        state_ref: &input.snapshot.state_ref,
        assertion_refs: &assertion_refs,
        output_refs: &output_refs,
        checks: &checks,
    })?;
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, &receipt.receipt_ref)?;
    Ok(SuccessParts {
        token,
        receipt,
        assertion,
    })
}

fn success_values(input: ValuesInput<'_>) -> Result<Vec<IoValue>> {
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
    evidence_values.push(crate::raft_control_plane::normalized_raft_commit_receipt_value(
        &proposal.commit_receipt,
        crate::raft_control_plane::INITIAL_CONSENSUS_ENGINE_EPOCH,
    )?);
    evidence_values.extend(proposal.predicates.iter().map(|value| value.value.clone()));
    evidence_values.extend(evidence_values_for(EvidenceValuesInput {
        request,
        receipt,
        token,
        snapshot,
        assertions: std::slice::from_ref(assertion),
        read: None,
    }));
    Ok(evidence_values)
}

fn record_success(input: SuccessInput<'_>) -> Result<CoordinationApplyResult> {
    let SuccessInput {
        runtime,
        request,
        before_snapshot,
        transition,
        snapshot,
        proposal,
    } = input;
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    let parts = success_parts(PartsInput {
        transition: &transition,
        request: &request,
        manifest: &runtime.manifest,
        engine_manifest: &runtime.raft.manifest,
        engine_epoch: active_engine_epoch(runtime),
        before_snapshot: &before_snapshot,
        snapshot: &snapshot,
        proposal_ref: &proposal.commit_receipt.receipt_ref,
    })?;
    runtime.state = transition.after_state;
    let evidence_values = success_values(ValuesInput {
        proposal: &proposal,
        request: &request,
        receipt: &parts.receipt,
        token: parts.token.as_ref(),
        snapshot: &snapshot,
        assertion: &parts.assertion,
    })?;
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
    before_snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
) -> Result<CoordinationApplyResult> {
    let snapshot = snapshot_from_state(transition.state_for_receipt())?;
    let proposal = propose_change(ChangeInput {
        runtime,
        request: &request,
        snapshot: &snapshot,
    })?;
    if proposal.decision != "pass" {
        let diagnostics = vec!["control-plane commit denied for coordination mutation".to_string()];
        return deny_result(runtime, request, before_snapshot, diagnostics, &["control-plane-commit", "fail"]);
    }
    record_success(SuccessInput {
        runtime,
        request,
        before_snapshot,
        transition,
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
    let transition = primitive_denial_transition(runtime, &request, diagnostics)?;
    deny_transition_result(runtime, request, snapshot, transition, extra_check)
}

fn deny_transition_result(
    runtime: &mut CoordinationRuntime,
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
    extra_check: &[&'static str; 2],
) -> Result<CoordinationApplyResult> {
    finish_denial_transition(runtime, DenialTransitionInput { request, snapshot, transition, extra_check, record_operation: true })
}

struct DenialTransitionInput<'a> {
    request: CoordinationRequest,
    snapshot: CoordinationStateSnapshot,
    transition: PrimitiveTransitionResult,
    extra_check: &'a [&'static str; 2],
    record_operation: bool,
}
