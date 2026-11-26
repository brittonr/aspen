//! Run dispatch preparation and completion.

struct PreparedRun {
    admission_ref: String,
    job_ref: String,
    execution_request_value: preserves::IOValue,
}

struct RunStart {
    diagnostics: Vec<String>,
    prepared: PreparedRun,
}

struct CompleteRunInput<'a> {
    state_root: &'a crate::node_state::DirectoryRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    prepared: PreparedRun,
    provenance: crate::provenance::Evaluation,
    diagnostics: Vec<String>,
}

type RunStartResult = std::result::Result<RunStart, Box<super::outcomes::ControlDispatch>>;

struct RunDenyInput<'a> {
    state_root: &'a crate::node_state::DirectoryRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    diagnostics: Vec<String>,
}

fn deny_run_start(input: RunDenyInput<'_>) -> crate::error::Result<RunStartResult> {
    let dispatch = super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &[],
        diagnostics: &input.diagnostics,
    })?;
    Ok(Err(Box::new(dispatch)))
}

fn prepare_run(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
) -> crate::error::Result<RunStartResult> {
    let mut diagnostics = super::gating::side_effect_preflight_diagnostics(request);
    let Some(execution_request_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control run requires execution request payload ref".to_string());
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    };
    let Some(admission_ref) = request.target_ref.as_deref() else {
        diagnostics.push("node control run requires admission receipt target ref".to_string());
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    };
    if !diagnostics.is_empty() {
        return deny_run_start(RunDenyInput {
            state_root,
            request,
            startup_receipt_ref,
            diagnostics,
        });
    }
    let execution_request_value = match super::storage::read_ledger_artifact(state_root, execution_request_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control run execution request not found in node ledger: {error}"));
            return deny_run_start(RunDenyInput {
                state_root,
                request,
                startup_receipt_ref,
                diagnostics,
            });
        }
    };
    let execution_request = match crate::job_dag::parse_job_execution_request_value(&execution_request_value) {
        Ok(execution_request) => execution_request,
        Err(error) => {
            diagnostics.push(format!("node control run execution request malformed: {error}"));
            return deny_run_start(RunDenyInput {
                state_root,
                request,
                startup_receipt_ref,
                diagnostics,
            });
        }
    };
    Ok(Ok(RunStart {
        diagnostics,
        prepared: PreparedRun {
            admission_ref: admission_ref.to_string(),
            job_ref: execution_request.job_ref,
            execution_request_value,
        },
    }))
}

fn complete_run(input: CompleteRunInput<'_>) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let admission_receipt_value =
        match super::storage::read_ledger_artifact(input.state_root, &input.prepared.admission_ref) {
            Ok(value) => value,
            Err(error) => {
                diagnostics.push(format!("node control run admission receipt not found in node ledger: {error}"));
                return super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
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
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_operation_subreceipt_path(&input.request.request_ref, "job-execution")?,
        &execution.receipt_value,
    )?;
    super::storage::import_artifact(input.state_root, &execution.receipt_value)?;
    let mut subreceipt_refs = Vec::with_capacity(3);
    subreceipt_refs.push(input.provenance.receipt_ref);
    subreceipt_refs.push(execution.receipt_ref.clone());
    if let Some(run) = execution.run.as_ref() {
        let run_ref = crate::preserves_rail::canonical_hash(&run.receipt_value)?;
        super::storage::write_preserves(
            input.state_root,
            &super::paths::control_operation_subreceipt_path(&input.request.request_ref, "job-run")?,
            &run.receipt_value,
        )?;
        super::storage::import_artifact(input.state_root, &run.receipt_value)?;
        subreceipt_refs.push(run_ref);
    }
    diagnostics.extend(execution.diagnostics.iter().cloned());
    if execution.decision != "pass" && diagnostics.is_empty() {
        diagnostics.push("node control run execution denied".to_string());
    }
    super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &subreceipt_refs,
        diagnostics: &diagnostics,
    })
}

pub(super) fn dispatch_run_request(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let startup = super::leases::current_startup_receipt(state_root)?;
    let start = match prepare_run(state_root, request, &startup.receipt_ref)? {
        Ok(start) => start,
        Err(dispatch) => return Ok(*dispatch),
    };
    let mut diagnostics = start.diagnostics;
    let provenance = super::handlers::evaluate_control_provenance(&super::handlers::ControlProvenanceInput {
        state_root,
        request,
        artifact_ref: &start.prepared.job_ref,
        operation: "run",
        subreceipt_kind: "job-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
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
