//! Install dispatch with its provenance gate.

struct InstallRefs {
    schema_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

struct InstallFinishInput<'a> {
    state_root: &'a crate::node_state::DirectoryRoot,
    request: &'a crate::node_runtime::ControlRequest,
    startup_receipt_ref: &'a str,
    payload_ref: &'a str,
    payload_value: preserves::IOValue,
    provenance: crate::provenance::Evaluation,
    diagnostics: Vec<String>,
}

fn finish_install_dispatch(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    subreceipt_refs: &[String],
    diagnostics: &[String],
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    super::gating::finalize_operation_dispatch(&super::gating::OperationFinalizeInput {
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
) -> crate::error::Result<InstallRefs> {
    let schema_refs = match request.target_ref.as_ref() {
        Some(target_ref) => vec![target_ref.clone()],
        None => vec![super::storage::local_ref(
            "node-control-install-schema",
            &request.request_ref,
        )?],
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

fn finish_install(input: InstallFinishInput<'_>) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let mut diagnostics = input.diagnostics;
    let provenance_receipt_refs = [input.provenance.receipt_ref.clone()];
    let refs = install_refs(input.request, input.payload_ref, &provenance_receipt_refs[0])?;
    let artifact_root = input.state_root.artifact_store()?;
    let install =
        match crate::artifacts::install_artifact_with_root(&artifact_root, &crate::artifacts::ArtifactInstallInput {
            kind: "node-control-artifact".to_string(),
            payload: input.payload_value,
            schema_refs: refs.schema_refs,
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: input.request.policy_refs.clone(),
            evidence_refs: refs.evidence_refs,
            installer_ref: input.request.request_ref.clone(),
            capability_refs: input.request.authority_refs.clone(),
        }) {
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
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_operation_subreceipt_path(&input.request.request_ref, "artifact-install")?,
        &install.receipt_value,
    )?;
    super::storage::import_artifact(input.state_root, &install.receipt_value)?;
    if install.decision == "pass" {
        super::storage::import_artifact(input.state_root, &install.artifact.value)?;
    } else if install.missing_dependencies.is_empty() {
        diagnostics.push("node control artifact install denied".to_string());
    } else {
        diagnostics
            .extend(install.missing_dependencies.iter().map(|reference| format!("missing dependency {reference}")));
    }
    let subreceipt_refs = [provenance_receipt_refs[0].clone(), install_receipt_ref];
    finish_install_dispatch(input.state_root, input.request, input.startup_receipt_ref, &subreceipt_refs, &diagnostics)
}

pub(super) fn dispatch_install_request(
    state_root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::ControlDispatch> {
    let startup = super::leases::current_startup_receipt(state_root)?;
    let mut diagnostics = super::gating::side_effect_preflight_diagnostics(request);
    let Some(payload_ref) = request.payload_ref.as_deref() else {
        diagnostics.push("node control install requires payload ref".to_string());
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    };
    if !diagnostics.is_empty() {
        return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
    }
    let payload_value = match super::storage::read_ledger_artifact(state_root, payload_ref) {
        Ok(value) => value,
        Err(error) => {
            diagnostics.push(format!("node control install payload not found in node ledger: {error}"));
            return finish_install_dispatch(state_root, request, &startup.receipt_ref, &[], &diagnostics);
        }
    };
    let provenance = super::handlers::evaluate_control_provenance(&super::handlers::ControlProvenanceInput {
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
