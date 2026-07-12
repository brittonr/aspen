
fn complete_run(input: CompleteRunInput<'_>) -> Result<ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let admission_receipt_value = match read_ledger_artifact(input.state_root, &input.prepared.admission_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run admission receipt not found in node ledger: {error}"));
            return finalize_operation_dispatch(&OperationFinalizeInput {
                state_root: input.state_root,
                request: input.request,
                startup_receipt_ref: input.startup_receipt_ref,
                subreceipt_refs: &provenance_receipt_refs,
                diagnostics: &diagnostics,
            });
        }
    };
    let execution = crate::job_dag::execution_loopback_with_node_state(
        input.state_root,
        &admission_receipt_value,
        &input.prepared.execution_request_value,
    )?;
    write_preserves(
        input.state_root,
        &control_operation_subreceipt_path(&input.request.request_ref, "job-execution")?,
        &execution.receipt_value,
    )?;
    import_artifact(input.state_root, &execution.receipt_value)?;
    let mut subreceipt_refs = Vec::with_capacity(3);
    subreceipt_refs.push(input.provenance.receipt_ref);
    subreceipt_refs.push(execution.receipt_ref.clone());
    if let Some(run) = execution.run.as_ref() {
        let run_ref = crate::preserves_rail::canonical_hash(&run.receipt_value)?;
        write_preserves(
            input.state_root,
            &control_operation_subreceipt_path(&input.request.request_ref, "job-run")?,
            &run.receipt_value,
        )?;
        import_artifact(input.state_root, &run.receipt_value)?;
        subreceipt_refs.push(run_ref);
    }
    diagnostics.extend(execution.diagnostics.iter().cloned());
    if execution.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control run execution denied".to_string());
    }
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &subreceipt_refs,
        diagnostics: &diagnostics,
    })
}

fn dispatch_run_request(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let start = match prepare_run(state_root, request, &startup.receipt_ref)? {
        Ok(start) => start,
        Err(dispatch) => return Ok(*dispatch),
    };
    let mut diagnostics = start.diagnostics;
    let provenance = evaluate_control_provenance(&ControlProvenanceInput {
        state_root,
        request,
        artifact_ref: &start.prepared.job_ref,
        operation: "run",
        subreceipt_kind: "job-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finalize_operation_dispatch(&OperationFinalizeInput {
            state_root,
            request,
            startup_receipt_ref: &startup.receipt_ref,
            subreceipt_refs: &provenance_receipt_refs,
            diagnostics: &diagnostics,
        });
    }
    complete_run(CompleteRunInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        prepared: start.prepared,
        provenance,
        diagnostics,
    })
}

fn dispatch_gate_request(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
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
    let gate_value = match read_ledger_artifact(state_root, gate_receipt_ref) {
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
    write_preserves(
        state_root,
        &control_operation_subreceipt_path(&request.request_ref, "octet-source-gate")?,
        &validation.value,
    )?;
    import_artifact(state_root, &validation.value)?;
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

fn finalize_operation_dispatch(input: &OperationFinalizeInput<'_>) -> Result<ControlDispatch> {
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    let operation_receipt = operation_receipt_value(&OperationReceiptValueInput {
        decision,
        request: input.request,
        diagnostics: input.diagnostics,
    })?;
    let operation_receipt_ref = crate::preserves_rail::canonical_hash(&operation_receipt)?;
    write_preserves(
        input.state_root,
        &control_operation_receipt_path(&input.request.request_ref)?,
        &operation_receipt,
    )?;
    import_artifact(input.state_root, &operation_receipt)?;
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
    write_preserves(
        input.state_root,
        &control_outbox_receipt_path(&input.request.request_ref)?,
        &control_receipt,
    )?;
    import_artifact(input.state_root, &control_receipt)?;
    Ok(ControlDispatch {
        operation: input.request.operation.clone(),
        request_ref: input.request.request_ref.clone(),
        control_receipt_ref,
        control_receipt_value: control_receipt,
        subreceipt_refs: all_subreceipt_refs,
    })
}

fn side_effect_preflight_diagnostics(request: &crate::node_runtime::ControlRequest) -> Vec<String> {
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

fn read_ledger_artifact<Root: NodeStateAuthority + ?Sized>(source: &Root, artifact_ref: &str) -> Result<IoValue> {
    let root = source.acquire_node_state_root()?;
    let ledger = root.ledger_store()?;
    crate::ledger::read_artifact_with_root(&ledger, artifact_ref)
}

fn control_receipt_for_request(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<IoValue> {
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
        capability_receipt_refs(state_root)?
    } else {
        Vec::new()
    };
    let resource_receipt_refs = if final_decision == "pass" {
        resource_receipt_refs(state_root)?
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
