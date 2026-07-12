
fn apply_gate_check(
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
    verified: &ControlLiveWorkflowBundleVerify,
) -> Result<Check> {
    let mut diagnostics = Vec::new();
    let receipt_ref = match input.receipt_value {
        Some(value) => match parse_control_live_workflow_bundle_gate_receipt(value) {
            Ok(receipt) => {
                if receipt.decision != "pass" {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate receipt {} decision {}",
                        receipt.receipt_ref, receipt.decision
                    ));
                }
                if receipt.bundle_ref != verified.bundle_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate bundle {} does not match {}",
                        receipt.bundle_ref, verified.bundle_ref
                    ));
                }
                if receipt.recomputed_verify_receipt_ref != verified.receipt_ref {
                    diagnostics.push(format!(
                        "node control live workflow bundle apply gate recomputed verify {} does not match current {}",
                        receipt.recomputed_verify_receipt_ref, verified.receipt_ref
                    ));
                }
                Some(receipt.receipt_ref)
            }
            Err(error) => {
                let receipt_ref = crate::preserves_rail::canonical_hash(value)?;
                diagnostics.push(format!("node control live workflow bundle apply gate receipt parse failed: {error}"));
                Some(receipt_ref)
            }
        },
        None => {
            if input.is_gate_receipt_required {
                diagnostics.push("node control live workflow bundle apply requires a current gate receipt".to_string());
            }
            None
        }
    };
    Ok(Check {
        receipt_ref,
        diagnostics,
    })
}

fn apply_import_step(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
) -> Result<ImportStep> {
    let import_input = live_workflow_bundle_import_input_from_apply(input);
    let imported = import_control_live_workflow_bundle_with_root(state_root, &import_input)?;
    if imported.decision == "pass" {
        Ok(ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: imported.imported_refs,
            diagnostics: Vec::new(),
        })
    } else {
        Ok(ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: Vec::new(),
            diagnostics: imported.diagnostics,
        })
    }
}

async fn apply_transfer_step(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
) -> Result<TransferStep> {
    let Some(request_value) = input.request_value else {
        return Ok(TransferStep::default());
    };
    let bundle = parse_control_live_workflow_bundle(input.bundle_value)?;
    let authority = parse_control_authority_grant(&bundle.authority_grant_value)?;
    let from_peer = input.from_peer.unwrap_or(&authority.peer_id);
    let peer_bootstrap_refs = if input.peer_bootstrap_refs.is_empty() {
        vec![bundle.peer_admission_ref.clone()]
    } else {
        input.peer_bootstrap_refs.to_vec()
    };
    let authority_refs = if input.authority_refs.is_empty() {
        vec![bundle.authority_grant_ref.clone()]
    } else {
        input.authority_refs.to_vec()
    };
    let send_input = ControlLiveSendInput {
        state_root: Some(input.state_root),
        request_value,
        receiver_ticket_value: &bundle.ticket_value,
        from_peer,
        sequence: input.sequence,
        expected_operation_ref: input.expected_operation_ref,
        expected_receiver_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        topology_profile: input.topology_profile,
        transport_profile: input.transport_profile,
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: &peer_bootstrap_refs,
        authority_refs: &authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
        join_timeout_ms: input.join_timeout_ms,
    };
    if input.should_send {
        let sent = send_control_live_ingress_with_root(&send_input, Some(state_root)).await?;
        let send_receipt = parse_control_live_send_receipt(&sent.send_receipt_value)?;
        let diagnostics = if send_receipt.decision == "pass" {
            Vec::new()
        } else {
            send_receipt.diagnostics
        };
        Ok(TransferStep {
            envelope_ref: Some(sent.envelope_ref),
            operation_ref: Some(sent.operation_ref),
            send_receipt_ref: Some(sent.send_receipt_ref),
            send_receipt_value: Some(sent.send_receipt_value),
            diagnostics,
        })
    } else {
        let preflight = preflight_control_live_send_with_root(&send_input, Some(state_root))?;
        let diagnostics = if preflight.decision == "pass" {
            Vec::new()
        } else {
            preflight.diagnostics
        };
        Ok(TransferStep {
            envelope_ref: Some(preflight.envelope_ref),
            operation_ref: Some(preflight.operation_ref),
            diagnostics,
            ..TransferStep::default()
        })
    }
}

fn finish_apply(
    state_root: &crate::node_state::NodeStateRoot,
    input: FinishInput<'_>,
) -> Result<ControlLiveWorkflowBundleApply> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let mode = if input.input.should_send {
        "send"
    } else if input.input.request_value.is_some() {
        "dry-run"
    } else {
        "import"
    };
    let receipt_value = live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
        decision,
        bundle_ref: &input.verified.bundle_ref,
        gate_receipt_ref: input.gate_receipt_ref.as_deref(),
        recomputed_verify_receipt_ref: &input.verified.receipt_ref,
        import_receipt_ref: input.import_receipt_ref.as_deref(),
        imported_refs: &input.imported_refs,
        mode,
        envelope_ref: input.envelope_ref.as_deref(),
        operation_ref: input.operation_ref.as_deref(),
        send_receipt_ref: input.send_receipt_ref.as_deref(),
        topology_profile_ref: selected_apply_topology_profile_ref(input.input),
        transport_profile_ref: selected_apply_transport_profile_ref(input.input),
        effective_max_attempts: effective_live_apply_max_attempts(input.input),
        effective_join_timeout_ms: effective_live_apply_join_timeout_ms(input.input),
        expected: &input.expected,
        diagnostics: &input.diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlLiveWorkflowBundleApply {
        bundle_ref: input.verified.bundle_ref,
        gate_receipt_ref: input.gate_receipt_ref,
        recomputed_verify_receipt_ref: input.verified.receipt_ref,
        import_receipt_ref: input.import_receipt_ref,
        imported_refs: input.imported_refs,
        envelope_ref: input.envelope_ref,
        operation_ref: input.operation_ref,
        send_receipt_ref: input.send_receipt_ref,
        send_receipt_value: input.send_receipt_value,
        diagnostics: input.diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}

pub async fn apply_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleApplyInput<'_>,
) -> Result<ControlLiveWorkflowBundleApply> {
    validate_live_workflow_bundle_apply_input(input)?;
    let state_root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    ensure_state_layout(&state_root)?;
    let verify_input = live_workflow_bundle_verify_input_from_apply(input);
    let verified = verify_control_live_workflow_bundle(&verify_input)?;
    let expected = live_workflow_bundle_expected_input_from_verify(&verify_input);
    let Check {
        receipt_ref: gate_receipt_ref,
        diagnostics: gate_diagnostics,
    } = apply_gate_check(input, &verified)?;
    let mut diagnostics = verified.diagnostics.clone();
    diagnostics.extend(gate_diagnostics);
    if input.should_send && input.request_value.is_none() {
        diagnostics.push("node control live workflow bundle apply send requested without a request".to_string());
    }
    let ImportStep {
        receipt_ref: import_receipt_ref,
        imported_refs,
        diagnostics: import_diagnostics,
    } = if diagnostics.is_empty() {
        apply_import_step(&state_root, input)?
    } else {
        ImportStep::default()
    };
    diagnostics.extend(import_diagnostics);
    let TransferStep {
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics: transfer_diagnostics,
    } = if diagnostics.is_empty() {
        apply_transfer_step(&state_root, input).await?
    } else {
        TransferStep::default()
    };
    diagnostics.extend(transfer_diagnostics);
    finish_apply(&state_root, FinishInput {
        input,
        verified,
        expected,
        gate_receipt_ref,
        import_receipt_ref,
        imported_refs,
        envelope_ref,
        operation_ref,
        send_receipt_ref,
        send_receipt_value,
        diagnostics,
    })
}

pub fn reconcile_control_live_workflow_bundle(
    input: &ControlLiveWorkflowBundleReconcileInput<'_>,
) -> Result<ControlLiveWorkflowBundleReconcile> {
    validate_live_workflow_bundle_reconcile_input(input)?;
    let apply = parse_control_live_workflow_bundle_apply_receipt(input.apply_receipt_value)?;
    let send = input.send_receipt_value.map(parse_control_live_send_receipt).transpose()?;
    let ingress = input.ingress_receipt_value.map(parse_control_ingress_receipt).transpose()?;
    let queue = input.queue_receipt_value.map(parse_control_queue_receipt).transpose()?;
    let control = input.control_receipt_value.map(crate::node_runtime::parse_control_receipt).transpose()?;
    let artifacts = ReconcileArtifacts {
        apply: &apply,
        send: send.as_ref(),
        ingress: ingress.as_ref(),
        queue: queue.as_ref(),
        control: control.as_ref(),
    };
    let mut diagnostics = live_workflow_bundle_reconcile_diagnostics(input, &artifacts)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let bindings = live_workflow_bundle_reconcile_bindings(&artifacts);
    let receipt_value = live_workflow_bundle_reconcile_receipt_value(&LiveWorkflowBundleReconcileReceiptValueInput {
        decision,
        apply_receipt_ref: &apply.receipt_ref,
        bundle_ref: &apply.bundle_ref,
        send_receipt_ref: bindings.send_receipt_ref,
        ingress_receipt_ref: bindings.ingress_receipt_ref,
        queue_receipt_ref: bindings.queue_receipt_ref,
        control_receipt_ref: bindings.control_receipt_ref,
        envelope_ref: bindings.envelope_ref,
        operation_ref: bindings.operation_ref,
        request_ref: bindings.request_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ControlLiveWorkflowBundleReconcile {
        bundle_ref: apply.bundle_ref.clone(),
        apply_receipt_ref: apply.receipt_ref.clone(),
        send_receipt_ref: bindings.send_receipt_ref.map(ToString::to_string),
        ingress_receipt_ref: bindings.ingress_receipt_ref.map(ToString::to_string),
        queue_receipt_ref: bindings.queue_receipt_ref.map(ToString::to_string),
        control_receipt_ref: bindings.control_receipt_ref.map(ToString::to_string),
        envelope_ref: bindings.envelope_ref.map(ToString::to_string),
        operation_ref: bindings.operation_ref.map(ToString::to_string),
        request_ref: bindings.request_ref.map(ToString::to_string),
        diagnostics,
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
    })
}
