//! Gate dispatch, operation finalization, and control receipts.

#[derive(Debug, Clone, Copy)]
pub(super) struct OperationFinalizeInput<'a> {
    pub(super) state_root: &'a crate::node_state::DirectoryRoot,
    pub(super) request: &'a crate::node_runtime::ControlRequest,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) subreceipt_refs: &'a [String],
    pub(super) diagnostics: &'a [String],
}

pub(super) fn dispatch_gate_request(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let startup = super::leases::current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(subject_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control gate requires target subject ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    let Some(gate_receipt_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control gate requires gate receipt payload ref".to_string());
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &diagnostics,
        });
    }
    let gate_value = match super::storage::read_ledger_artifact(state_root, gate_receipt_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control gate receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root,
                request,
                startup_receipt_ref: &startup.receipt_ref,
                subreceipt_refs: &[],
                diagnostics: &diagnostics,
            });
        }
    };
    let validation =
        crate::octet_gate::validate_octet_source_gate(&crate::octet_gate::OctetSourceGateValidationInput {
            consumer: "node-control-gate".to_string(),
            subject_ref: subject_ref.to_string(),
            receipt_value: Some(gate_value),
            source_scope: crate::octet_gate::default_source_scope("node-control-gate")?,
        })?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_operation_subreceipt_path(&request.request_ref, "octet-source-gate")?,
        &validation.value,
    )?;
    super::storage::import_artifact(state_root, &validation.value)?;
    diagnostics.extend(validation.diagnostics.iter().cloned());
    if validation.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control gate validation denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        subreceipt_refs: std::slice::from_ref(&validation.validation_ref),
        diagnostics: &diagnostics,
    })
}

pub(super) fn finalize_operation_dispatch(
    input: &OperationFinalizeInput<'_>,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let operation_receipt =
        super::operations::operation_receipt_value(&super::operations::OperationReceiptValueInput {
            decision,
            request: input.request,
            diagnostics: input.diagnostics,
        })?;
    let operation_receipt_ref = crate::preserves_rail::canonical_hash(&operation_receipt)?;
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_operation_receipt_path(&input.request.request_ref)?,
        &operation_receipt,
    )?;
    super::storage::import_artifact(input.state_root, &operation_receipt)?;
    let mut all_subreceipt_refs = Vec::with_capacity(input.subreceipt_refs.len() + 1);
    all_subreceipt_refs.extend(input.subreceipt_refs.iter().cloned());
    all_subreceipt_refs.push(operation_receipt_ref);
    let control_receipt = control_receipt_for_request(
        input.state_root,
        input.request,
        input.startup_receipt_ref,
        &all_subreceipt_refs,
        input.diagnostics,
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt)?;
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_outbox_receipt_path(&input.request.request_ref)?,
        &control_receipt,
    )?;
    super::storage::import_artifact(input.state_root, &control_receipt)?;
    Ok(super::outcomes::ControlDispatch {
        operation: input.request.operation.clone(),
        request_ref: input.request.request_ref.clone(),
        control_receipt_ref,
        control_receipt_value: control_receipt,
        subreceipt_refs: all_subreceipt_refs,
    })
}

pub(super) fn side_effect_preflight_diagnostics(request: &crate::node_runtime::ControlRequest) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(3);
    if request.authority_refs.is_empty() {
        diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        diagnostics.push("node control resource refs missing".to_string());
    }
    diagnostics
}

pub(super) fn control_receipt_for_request(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> crate::error::Result<preserves::IOValue> {
    let decision = if diagnostics.is_empty()
        && !request.authority_refs.is_empty()
        && !request.policy_refs.is_empty()
        && !request.resource_refs.is_empty()
    {
        "pass"
    } else {
        "deny"
    };
    let mut receipt_diagnostics = Vec::with_capacity(diagnostics.len() + 3);
    receipt_diagnostics.extend(diagnostics.iter().cloned());
    if request.authority_refs.is_empty() {
        receipt_diagnostics.push("node control authority refs missing".to_string());
    }
    if request.policy_refs.is_empty() {
        receipt_diagnostics.push("node control policy refs missing".to_string());
    }
    if request.resource_refs.is_empty() {
        receipt_diagnostics.push("node control resource refs missing".to_string());
    }
    let final_decision = if receipt_diagnostics.is_empty() {
        decision
    } else {
        "deny"
    };
    let authority_receipt_refs = if final_decision == "pass" {
        super::storage::capability_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    let resource_receipt_refs = if final_decision == "pass" {
        super::storage::resource_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    crate::node_runtime::control_receipt_value(&crate::node_runtime::ControlReceiptValueInput {
        decision: final_decision,
        request,
        startup_receipt_ref,
        authority_receipt_refs: &authority_receipt_refs,
        resource_receipt_refs: &resource_receipt_refs,
        subreceipt_refs,
        diagnostics: &receipt_diagnostics,
    })
}
