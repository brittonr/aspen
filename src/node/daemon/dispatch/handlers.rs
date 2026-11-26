//! Status and shutdown dispatch and queue receipts.

pub(super) fn prior_dispatch_for_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<Option<super::outcomes::ControlDispatch>> {
    let receipt_path = super::paths::control_outbox_receipt_path(&request.request_ref)?;
    if !root.try_exists(&receipt_path)? {
        return Ok(None);
    }
    let archived_path = super::paths::control_outbox_request_path(&request.request_ref)?;
    if root.try_exists(&archived_path)? {
        let archived_value = super::storage::read_preserves(root, &archived_path)?;
        let archived_ref = crate::preserves_rail::canonical_hash(&archived_value)?;
        if archived_ref != request.request_ref {
            return Err(crate::error::Failure::invalid_harness(
                "node control duplicate request conflicts with archived request evidence",
            ));
        }
    }
    let control_receipt_value = super::storage::read_preserves(root, &receipt_path)?;
    let control = crate::node_runtime::parse_control_receipt(&control_receipt_value)?;
    if control.request_ref != request.request_ref {
        return Err(crate::error::Failure::invalid_harness(
            "node control duplicate receipt conflicts with request ref",
        ));
    }
    Ok(Some(super::outcomes::ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: control.receipt_ref,
        control_receipt_value: control.value,
        subreceipt_refs: control.subreceipt_refs,
    }))
}

pub(super) fn write_dispatch_queue_receipt(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    phase: &str,
) -> crate::error::Result<String> {
    let location_ref = super::storage::local_ref(
        "node-control-outbox-path",
        &super::paths::control_outbox_receipt_path(&request.request_ref)?.display(),
    )?;
    let diagnostics = Vec::new();
    let queue_receipt = super::operations::queue_receipt_value(&super::operations::QueueReceiptValueInput {
        decision: "pass",
        phase,
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &diagnostics,
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&queue_receipt)?;
    super::storage::write_preserves(root, &super::paths::dispatch_receipt_path(&request.request_ref)?, &queue_receipt)?;
    super::storage::import_artifact(root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

pub(super) fn dispatch_status_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let status = super::running::status_local_node_with_request(root, request)?;
    super::storage::write_preserves(
        root,
        &super::paths::control_outbox_receipt_path(&request.request_ref)?,
        &status.control_receipt_value,
    )?;
    Ok(super::outcomes::ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

pub(super) fn dispatch_shutdown_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let startup = super::leases::current_startup_receipt(root)?;
    let admission = super::stopping::admit_shutdown_request(root, request, &startup)?;
    let Some(plan) = admission.plan else {
        return super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
            state_root: root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &admission.diagnostics,
        });
    };
    let stop = super::stopping::execute_shutdown_plan(root, request, &plan)?;
    super::storage::write_preserves(
        root,
        &super::paths::control_outbox_receipt_path(&request.request_ref)?,
        &stop.control_receipt_value,
    )?;
    Ok(super::outcomes::ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ControlProvenanceInput<'a> {
    pub(super) state_root: &'a crate::node_state::DirectoryRoot,
    pub(super) request: &'a crate::node_runtime::ControlRequest,
    pub(super) artifact_ref: &'a str,
    pub(super) operation: &'a str,
    pub(super) subreceipt_kind: &'a str,
}

pub(super) fn evaluate_control_provenance(
    input: &ControlProvenanceInput<'_>,
) -> crate::error::Result<crate::provenance::Evaluation> {
    let mut provenance_diagnostics = Vec::with_capacity(input.request.evidence_refs.len().saturating_add(1));
    if input.request.evidence_refs.is_empty() {
        provenance_diagnostics.push("node control provenance evidence refs missing".to_string());
    }
    let mut provenance_values = Vec::with_capacity(input.request.evidence_refs.len());
    let mut build_verification_values = Vec::with_capacity(input.request.evidence_refs.len());
    for evidence_ref in &input.request.evidence_refs {
        match super::storage::read_ledger_artifact(input.state_root, evidence_ref) {
            Ok(value) => {
                if crate::provenance::parse_build_verification_receipt(&value).is_ok() {
                    build_verification_values.push(value);
                } else {
                    provenance_values.push(value);
                }
            }
            Err(error) => provenance_diagnostics
                .push(format!("node control provenance evidence {evidence_ref} not found in node ledger: {error}")),
        }
    }
    let evaluation = crate::provenance::evaluate(&crate::provenance::EvaluationInput {
        operation: input.operation,
        profile: "node-control",
        artifact_ref: input.artifact_ref,
        provenance_values: &provenance_values,
        build_verification_values: &build_verification_values,
        prior_diagnostics: &provenance_diagnostics,
    })?;
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_operation_subreceipt_path(&input.request.request_ref, input.subreceipt_kind)?,
        &evaluation.receipt_value,
    )?;
    super::storage::import_artifact(input.state_root, &evaluation.receipt_value)?;
    Ok(evaluation)
}
