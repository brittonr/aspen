
fn evaluate_control_provenance(input: &ControlProvenanceInput<'_>) -> Result<crate::provenance::Evaluation> {
    let mut provenance_diagnostics = Vec::with_capacity(input.request.evidence_refs.len().saturating_add(1));
    if input.request.evidence_refs.is_empty() {
        provenance_diagnostics.push("node control provenance evidence refs missing".to_string());
    }
    let mut provenance_values = Vec::with_capacity(input.request.evidence_refs.len());
    let mut build_verification_values = Vec::with_capacity(input.request.evidence_refs.len());
    for evidence_ref in &input.request.evidence_refs {
        match read_ledger_artifact(input.state_root, evidence_ref) {
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
    write_preserves(
        input.state_root,
        &control_operation_subreceipt_path(&input.request.request_ref, input.subreceipt_kind)?,
        &evaluation.receipt_value,
    )?;
    import_artifact(input.state_root, &evaluation.receipt_value)?;
    Ok(evaluation)
}

struct InstallRefs {
    schema_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct InstallFinishInput<'a> {
    state_root: &'a crate::node_state::NodeStateRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    payload_ref: &'a str,
    payload_value: IoValue,
    provenance: crate::provenance::Evaluation,
    diagnostics: Vec<String>,
}

fn finish_install_dispatch(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> Result<ControlDispatch> {
    finalize_operation_dispatch(&OperationFinalizeInput {
        state_root,
        request,
        startup_receipt_ref,
        subreceipt_refs,
        diagnostics,
    })
}

fn install_refs(
    request: &crate::node_runtime::ControlRequest,
    payload_ref: &str,
    provenance_receipt_ref: &str,
) -> Result<InstallRefs> {
    let schema_refs = match request.target_ref.as_ref() {
        Some(target_ref) => vec![target_ref.clone()],
        None => vec![local_ref("node-control-install-schema", &request.request_ref)?],
    };
    let extra_evidence_refs = if request.target_ref.is_some() { 3 } else { 2 };
    let mut evidence_refs =
        Vec::with_capacity(request.resource_refs.len() + request.evidence_refs.len() + extra_evidence_refs);
    evidence_refs.extend(request.resource_refs.iter().cloned());
    evidence_refs.extend(request.evidence_refs.iter().cloned());
    evidence_refs.push(provenance_receipt_ref.to_string());
    evidence_refs.push(payload_ref.to_string());
    if let Some(target_ref) = request.target_ref.as_ref() {
        evidence_refs.push(target_ref.clone());
    }
    Ok(InstallRefs {
        schema_refs,
        evidence_refs,
    })
}

fn finish_install(input: InstallFinishInput<'_>) -> Result<ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let refs = install_refs(input.request, input.payload_ref, &provenance_receipt_refs[0])?;
    let artifact_root = input.state_root.artifact_store()?;
    let install = match crate::artifacts::install_artifact_with_root(
        &artifact_root,
        &crate::artifacts::ArtifactInstallInput {
            kind: "node-control-artifact".to_string(),
            payload: input.payload_value,
            schema_refs: refs.schema_refs,
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: input.request.policy_refs.clone(),
            evidence_refs: refs.evidence_refs,
            installer_ref: input.request.request_ref.clone(),
            capability_refs: input.request.authority_refs.clone(),
        },
    ) {
        Ok(install) => install,
        Err(error) => {
            diagnostics.push(format!("node control artifact install failed: {error}"));
            return finish_install_dispatch(
                input.state_root,
                input.request,
                input.startup_receipt_ref,
                &provenance_receipt_refs,
                &diagnostics,
            );
        }
    };
    let install_receipt_ref = crate::preserves_rail::canonical_hash(&install.receipt_value)?;
    write_preserves(
        input.state_root,
        &control_operation_subreceipt_path(&input.request.request_ref, "artifact-install")?,
        &install.receipt_value,
    )?;
    import_artifact(input.state_root, &install.receipt_value)?;
    if install.decision == "pass" {
        import_artifact(input.state_root, &install.artifact.value)?;
    } else if install.missing_dependencies.is_empty() {
        diagnostics.push("node control artifact install denied".to_string());
    } else {
        diagnostics
            .extend(install.missing_dependencies.iter().map(|reference| format!("missing dependency {reference}")));
    }
    let subreceipt_refs = [provenance_receipt_refs[0].clone(), install_receipt_ref];
    finish_install_dispatch(input.state_root, input.request, input.startup_receipt_ref, &subreceipt_refs, &diagnostics)
}

fn dispatch_install_request(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<ControlDispatch> {
    let startup = current_startup_receipt(state_root)?;
    let mut diagnostics = side_effect_preflight_diagnostics(request);
    let Some(payload_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control install requires payload ref".to_string());
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    };
    if !diagnostics.is_empty() {
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    }
    let payload_value = match read_ledger_artifact(state_root, payload_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control install payload not found in node ledger: {error}"));
            return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
        }
    };
    let provenance = evaluate_control_provenance(&ControlProvenanceInput {
        state_root,
        request,
        artifact_ref: payload_ref,
        operation: "install",
        subreceipt_kind: "artifact-provenance",
    })?;
    let provenance_receipt_refs = [provenance.receipt_ref.clone()];
    diagnostics.extend(provenance.diagnostics.iter().cloned());
    if provenance.decision != "pass" {
        return finish_install_dispatch(
            state_root,
            request,
            &startup.receipt_ref,
            &provenance_receipt_refs,
            &diagnostics,
        );
    }
    finish_install(InstallFinishInput {
        state_root,
        request,
        startup_receipt_ref: &startup.receipt_ref,
        payload_ref,
        payload_value,
        provenance,
        diagnostics,
    })
}

struct PreparedRun {
    admission_ref: String,
    job_ref: String,
    execution_request_value: IoValue,
}

struct RunStart {
    diagnostics: Vec<String>,
    prepared: PreparedRun,
}

struct CompleteRunInput<'a> {
    state_root: &'a crate::node_state::NodeStateRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    prepared: PreparedRun,
    provenance: crate::provenance::Evaluation,
    diagnostics: Vec<String>,
}

type RunStartResult = std::result::Result<RunStart, Box<ControlDispatch>>;

struct RunDenyInput<'a> {
    state_root: &'a crate::node_state::NodeStateRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    diagnostics: Vec<String>,
}

fn deny_run_start(input: RunDenyInput<'_>) -> Result<RunStartResult> {
    let dispatch = finalize_operation_dispatch(&OperationFinalizeInput {
        state_root: input.state_root,
        request: input.request,
        startup_receipt_ref: input.startup_receipt_ref,
        subreceipt_refs: &[],
        diagnostics: &input.diagnostics,
    })?;
    Ok(Err(Box::new(dispatch)))
}

fn prepare_run(
    state_root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
) -> Result<RunStartResult> {
    let mut diagnostics = side_effect_preflight_diagnostics(request);
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
    let execution_request_value = match read_ledger_artifact(state_root, execution_request_ref) {
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
