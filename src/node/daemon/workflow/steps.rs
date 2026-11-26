//! The gate, import, transfer, and finish steps of a bundle apply.

pub(super) fn apply_gate_check(
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
    verified: &super::artifacts::ControlLiveWorkflowBundleVerify,
) -> crate::error::Result<super::applying::Check> {
    let mut diagnostics = Vec::new();
    let receipt_ref = match input.receipt_value {
        Some(value) => match super::manifests::parse_control_live_workflow_bundle_gate_receipt(value) {
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
    Ok(super::applying::Check {
        receipt_ref,
        diagnostics,
    })
}

pub(super) fn apply_import_step(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> crate::error::Result<super::applying::ImportStep> {
    let import_input = super::applying::live_workflow_bundle_import_input_from_apply(input);
    let imported = super::intake::import_control_live_workflow_bundle_with_root(state_root, &import_input)?;
    if imported.decision == "pass" {
        Ok(super::applying::ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: imported.imported_refs,
            diagnostics: Vec::new(),
        })
    } else {
        Ok(super::applying::ImportStep {
            receipt_ref: Some(imported.receipt_ref),
            imported_refs: Vec::new(),
            diagnostics: imported.diagnostics,
        })
    }
}

pub(super) async fn apply_transfer_step(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> crate::error::Result<super::applying::TransferStep> {
    let Some(request_value) = input.request_value else {
        return Ok(super::applying::TransferStep::default());
    };
    let bundle = super::manifests::parse_control_live_workflow_bundle(input.bundle_value)?;
    let authority = super::issuance::parse_control_authority_grant(&bundle.authority_grant_value)?;
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
    let send_input = super::offers::ControlLiveSendInput {
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
        let sent = super::sending::send_control_live_ingress_with_root(&send_input, Some(state_root)).await?;
        let send_receipt = super::receipts::parse_control_live_send_receipt(&sent.send_receipt_value)?;
        let diagnostics = if send_receipt.decision == "pass" {
            Vec::new()
        } else {
            send_receipt.diagnostics
        };
        Ok(super::applying::TransferStep {
            envelope_ref: Some(sent.envelope_ref),
            operation_ref: Some(sent.operation_ref),
            send_receipt_ref: Some(sent.send_receipt_ref),
            send_receipt_value: Some(sent.send_receipt_value),
            diagnostics,
        })
    } else {
        let preflight = super::vetting::preflight_control_live_send_with_root(&send_input, Some(state_root))?;
        let diagnostics = if preflight.decision == "pass" {
            Vec::new()
        } else {
            preflight.diagnostics
        };
        Ok(super::applying::TransferStep {
            envelope_ref: Some(preflight.envelope_ref),
            operation_ref: Some(preflight.operation_ref),
            diagnostics,
            ..super::applying::TransferStep::default()
        })
    }
}

pub(super) fn finish_apply(
    state_root: &crate::node_state::DirectoryRoot,
    input: super::applying::FinishInput<'_>,
) -> crate::error::Result<super::artifacts::ControlLiveWorkflowBundleApply> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let mode = if input.input.should_send {
        "send"
    } else if input.input.request_value.is_some() {
        "dry-run"
    } else {
        "import"
    };
    let receipt_value = super::gates::live_workflow_bundle_apply_receipt_value(
        &super::gates::LiveWorkflowBundleApplyReceiptValueInput {
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
            topology_profile_ref: super::tuning::selected_apply_topology_profile_ref(input.input),
            transport_profile_ref: super::tuning::selected_apply_transport_profile_ref(input.input),
            effective_max_attempts: super::tuning::effective_live_apply_max_attempts(input.input),
            effective_join_timeout_ms: super::tuning::effective_live_apply_join_timeout_ms(input.input),
            expected: &input.expected,
            diagnostics: &input.diagnostics,
        },
    )?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::artifacts::ControlLiveWorkflowBundleApply {
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
