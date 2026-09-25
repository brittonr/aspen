
fn write_dispatch_queue_receipt(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
    phase: &str,
) -> Result<String> {
    let location_ref = local_ref(
        "node-control-outbox-path",
        &control_outbox_receipt_path(&request.request_ref)?.display(),
    )?;
    let diagnostics = Vec::new();
    let queue_receipt = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase,
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &diagnostics,
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&queue_receipt)?;
    write_preserves(root, &dispatch_receipt_path(&request.request_ref)?, &queue_receipt)?;
    import_artifact(root, &queue_receipt)?;
    Ok(queue_receipt_ref)
}

fn dispatch_status_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let status = status_local_node_with_request(root, request)?;
    write_preserves(
        root,
        &control_outbox_receipt_path(&request.request_ref)?,
        &status.control_receipt_value,
    )?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: status.control_receipt_ref,
        control_receipt_value: status.control_receipt_value,
        subreceipt_refs: vec![status.health_ref],
    })
}

fn dispatch_shutdown_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(root)?;
    let admission = admit_shutdown_request(root, request, &startup)?;
    let Some(plan) = admission.plan else {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root: root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &[],
            diagnostics: &admission.diagnostics,
        });
    };
    let stop = execute_shutdown_plan(root, request, &plan)?;
    write_preserves(
        root,
        &control_outbox_receipt_path(&request.request_ref)?,
        &stop.control_receipt_value,
    )?;
    Ok(ControlDispatch {
        operation: request.operation.clone(),
        request_ref: request.request_ref.clone(),
        control_receipt_ref: stop.control_receipt_ref,
        control_receipt_value: stop.control_receipt_value,
        subreceipt_refs: vec![stop.shutdown_ref],
    })
}

#[derive(Debug, Clone, Copy)]
struct ControlProvenanceInput<'a> {
    state_root: &'a crate::node_state::NodeStateRoot,
    request: &'a crate::node_runtime::ControlRequest,
    artifact_ref: &'a str,
    operation: &'a str,
    subreceipt_kind: &'a str,
}
