
fn status_local_node_with_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Status> {
    let startup_value = read_preserves(root, &fixed_node_path(STARTUP_FILE)?)?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let observations = read_node_run_observations(root, &startup.receipt_ref)?;
    let control = completed_node_run_control(root, &observations)?;
    let lifecycle = observations.classify("shutdown", control.as_ref(), false);
    let shutdown_ref = match lifecycle.lifecycle {
        crate::node_runtime::NodeRunLifecycle::Active => None,
        crate::node_runtime::NodeRunLifecycle::Stopped => observations.shutdown_receipt.map(|shutdown| shutdown.receipt_ref),
        crate::node_runtime::NodeRunLifecycle::Unresolved => {
            return Err(MoltenError::invalid_harness("node status has unresolved current-run observations"));
        }
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = crate::node_runtime::node_health_receipt_value(&crate::node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &index_receipt_refs(root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health_value)?;
    write_preserves(root, &fixed_node_path(HEALTH_FILE)?, &health_value).map_err(|error| match error {
        MoltenError::NodeStateWriteOpenDenied { path, errno } if path == HEALTH_FILE => {
            if let [binding_ref] = startup.version_refs.as_slice() {
                MoltenError::NodeStatusHealthOpenDenied {
                    path,
                    errno,
                    startup_ref: startup.receipt_ref.clone(),
                    config_ref: startup.config_ref.clone(),
                    binding_ref: binding_ref.clone(),
                }
            } else {
                MoltenError::invalid_harness("node status health denial lacks one selected source binding")
            }
        }
        other => other,
    })?;
    import_artifact(root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(root, &fixed_node_path(CONTROL_STATUS_FILE)?, &control_receipt_value)?;
    import_artifact(root, &control_receipt_value)?;
    Ok(Status {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}
