
fn propose_change(input: ChangeInput<'_>) -> Result<Proposal> {
    let ChangeInput {
        runtime,
        request,
        intent,
    } = input;
    let command = crate::raft_control_plane::control_registry_command_value(
        &crate::raft_control_plane::ControlRegistryCommandInput {
            operation: "set-coordination-state".to_string(),
            namespace: coordination_namespace(&request.service),
            name: request.key.clone(),
            target_ref: Some(intent.intent_ref.clone()),
        },
    )?;
    let evidence_refs = vec![
        request.request_ref.clone(),
        request.operation_id_ref.clone(),
        intent.intent_ref.clone(),
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

fn commit_intent_for(
    request: &CoordinationRequest,
    before_snapshot: &CoordinationStateSnapshot,
    transition: &PrimitiveTransitionResult,
) -> Result<CommitIntent> {
    let time_sample = transition
        .time_sample
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("passing coordination transition carries no clock sample"))?;
    let unbound_snapshot = snapshot_from_state(&transition.after_state)?;
    let value = coordination_commit_intent_value(CommitIntentValueInput {
        request_ref: &request.request_ref,
        before_state_ref: &before_snapshot.state_ref,
        unbound_after_state_ref: &unbound_snapshot.state_ref,
        token_claim: transition.token_claim.as_ref(),
        time_sample,
    })?;
    Ok(CommitIntent {
        intent_ref: canonical_hash(&value)?,
        value,
        unbound_snapshot,
    })
}

fn status_assertion_for(
    request: &CoordinationRequest,
    fact: &IoValue,
    state_ref: &str,
    control_plane_receipt_ref: &str,
) -> Result<CoordinationStatusAssertion> {
    let value = coordination_status_assertion_value(StatusAssertionInput {
        service: &request.service,
        key: &request.key,
        read_consistency_mode: &request.read_consistency_mode,
        fact,
        state_ref,
        control_plane_receipt_ref,
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
        ("status-assertion-ref-bound", "pass"),
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

// r[impl molten.coordination.truthful_tokens_assertions_and_expiry]
/// The status assertion binds the control-plane commit receipt, so the coordination receipt lists the
/// exact ref of the assertion that is emitted.
fn success_parts(input: PartsInput<'_>) -> Result<SuccessParts> {
    let token_ref = input.token.as_ref().map(|item| item.token_ref.clone());
    let base = if input.token.is_some() {
        status_fact_for(input.state, input.manifest, &input.request.service, &input.request.key)?
    } else {
        input.transition.status_fact.clone()
    };
    let fact = engine_status_fact(input.engine_manifest, input.engine_epoch, &base);
    let assertion = status_assertion_for(input.request, &fact, &input.snapshot.state_ref, input.proposal_ref)?;
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
    Ok(SuccessParts {
        token: input.token,
        receipt,
        assertion,
    })
}

fn success_values(input: ValuesInput<'_>) -> Result<Vec<IoValue>> {
    let ValuesInput {
        proposal,
        request,
        intent,
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
    evidence_values.push(intent.value.clone());
    if intent.unbound_snapshot.state_ref != snapshot.state_ref {
        evidence_values.push(intent.unbound_snapshot.value.clone());
    }
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
        intent,
        proposal,
    } = input;
    runtime.next_sequence = runtime
        .next_sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("coordination raft sequence overflow"))?;
    let commit_receipt_ref = proposal.commit_receipt.receipt_ref.clone();
    let mut state = transition.after_state.clone();
    let token = transition
        .token_claim
        .as_ref()
        .map(|claim| bind_token_claim(&mut state, claim, &commit_receipt_ref))
        .transpose()?;
    let snapshot = snapshot_from_state(&state)?;
    let parts = success_parts(PartsInput {
        transition: &transition,
        request: &request,
        manifest: &runtime.manifest,
        engine_manifest: &runtime.raft.manifest,
        engine_epoch: active_engine_epoch(runtime),
        before_snapshot: &before_snapshot,
        state: &state,
        snapshot: &snapshot,
        proposal_ref: &commit_receipt_ref,
        token,
    })?;
    runtime.state = state;
    let evidence_values = success_values(ValuesInput {
        proposal: &proposal,
        request: &request,
        intent: &intent,
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
        time_sample: transition.time_sample.clone(),
    };
    runtime.applied_operations.insert(request.operation_id_ref.clone(), result.clone());
    Ok(result)
}
