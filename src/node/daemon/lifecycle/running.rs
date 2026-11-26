//! Local run and status with the source gate and status requests.

pub fn run_local(input: &super::requests::RunInput<'_>) -> crate::error::Result<super::outcomes::Run> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open_existing(input.state_root)?;
    run_local_with_root(&root)
}

pub fn run_local_with_root(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<super::outcomes::Run> {
    let source = super::source_binding::load(root)?;
    run_local_with_source(root, source)
}

/// Performs source admission without preparing, writing or activating node state.
pub fn admit_startup_source_gate_with_root(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<crate::node_runtime::ValidatedNodeSourceGate> {
    let source = super::source_binding::load(root)?;
    Ok(crate::kernel::ValidatedNodeSourceGate {
        binding_ref: source.binding_ref,
        gate_ref: source.gate_ref,
        validation_ref: source.validation_ref,
    })
}

#[cfg(test)]
pub(crate) fn run_local_fixture_for_tests(
    input: &super::requests::RunInput<'_>,
) -> crate::error::Result<super::outcomes::Run> {
    let root = crate::node_state::DirectoryRoot::open_existing(input.state_root)?;
    run_local_with_root_fixture_for_tests(&root)
}

#[cfg(test)]
pub(crate) fn run_local_with_root_fixture_for_tests(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<super::outcomes::Run> {
    run_local_with_source(root, super::source_binding::tests::fixture()?)
}

fn run_local_with_source(
    root: &crate::node_state::DirectoryRoot,
    source: super::source_binding::Accepted,
) -> crate::error::Result<super::outcomes::Run> {
    let config_value =
        super::storage::read_preserves(root, &super::storage::fixed_node_path(super::settings::CONFIG_FILE)?)?;
    let identity_receipt = super::storage::read_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::IDENTITY_RECEIPT_FILE)?,
    )?;
    let identity_receipt_ref = crate::preserves_rail::canonical_hash(&identity_receipt)?;
    let index_receipt_refs = super::storage::index_receipt_refs(root)?;
    let resource_receipt_refs = super::storage::resource_receipt_refs(root)?;
    let capability_receipt_refs = super::storage::capability_receipt_refs(root)?;
    let version_refs = vec![source.binding_ref.clone()];
    let profile_metadata_refs = super::storage::profile_metadata_refs(root)?;
    let admitted_gate = crate::kernel::ValidatedNodeSourceGate {
        binding_ref: source.binding_ref.clone(),
        gate_ref: source.gate_ref.clone(),
        validation_ref: source.validation_ref.clone(),
    };
    let run = crate::node_runtime::start_node_runtime_with_source_gate(
        &crate::node_runtime::NodeRuntimeStartInput {
            config_value,
            identity_receipt_ref,
            index_receipt_refs,
            source_gate_receipt_refs: vec![source.gate_ref],
            source_gate_receipt_values: vec![source.gate_value],
            profile_metadata_refs,
            capability_receipt_refs,
            resource_receipt_refs,
            version_refs,
        },
        &admitted_gate,
    )?;
    if run.decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "node daemon startup denied receipt={}",
            run.startup_receipt.receipt_ref
        )));
    }
    super::storage::ensure_state_layout(root)?;
    super::inspection::verify_restart_state(root)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path("source-binding-validation.preserves")?,
        &source.validation_value,
    )?;
    for (adapter, value) in run.adapter_receipts.iter().zip(run.adapter_receipt_values.iter()) {
        let path = super::paths::node_leaf_path("receipts", &format!("adapter-start-{}.preserves", adapter.name))?;
        super::storage::write_preserves(root, &path, value)?;
    }
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::STARTUP_FILE)?,
        &run.startup_receipt.value,
    )?;
    let startup_ref = run.startup_receipt.receipt_ref.clone();
    super::leases::write_active_lock(root, &startup_ref)?;
    super::storage::import_artifact(root, &run.startup_receipt.value)?;
    Ok(super::outcomes::Run {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local(input: &super::requests::StatusInput<'_>) -> crate::error::Result<super::outcomes::Status> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open_existing(input.state_root)?;
    status_local_with_root(&root)
}

pub fn status_local_with_root(
    root: &crate::node_state::DirectoryRoot,
) -> crate::error::Result<super::outcomes::Status> {
    let request = status_request()?;
    status_local_node_with_request(root, &request)
}

pub(super) fn status_local_node_with_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::Status> {
    let startup_value =
        super::storage::read_preserves(root, &super::storage::fixed_node_path(super::settings::STARTUP_FILE)?)?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let observations = super::tracking::read_node_run_observations(root, &startup.receipt_ref)?;
    let control = super::tracking::completed_node_run_control(root, &observations)?;
    let lifecycle = observations.classify("shutdown", control.as_ref(), false);
    let shutdown_ref = match lifecycle.lifecycle {
        crate::node_runtime::NodeRunLifecycle::Active => None,
        crate::node_runtime::NodeRunLifecycle::Stopped => {
            observations.shutdown_receipt.map(|shutdown| shutdown.receipt_ref)
        }
        crate::node_runtime::NodeRunLifecycle::Unresolved => {
            return Err(crate::error::Failure::invalid_harness("node status has unresolved current-run observations"));
        }
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = crate::node_runtime::node_health_receipt_value(&crate::node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &super::storage::index_receipt_refs(root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health_value)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::HEALTH_FILE)?,
        &health_value,
    )
    .map_err(|error| health_write_error(error, &startup))?;
    super::storage::import_artifact(root, &health_value)?;
    let control_receipt_value = super::gating::control_receipt_for_request(
        root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONTROL_STATUS_FILE)?,
        &control_receipt_value,
    )?;
    super::storage::import_artifact(root, &control_receipt_value)?;
    Ok(super::outcomes::Status {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

fn health_write_error(
    error: crate::error::Failure,
    startup: &crate::node_runtime::NodeStartupReceipt,
) -> crate::error::Failure {
    match error {
        crate::error::Failure::NodeStateWriteOpenDenied { path, errno } if path == super::settings::HEALTH_FILE => {
            if let [binding_ref] = startup.version_refs.as_slice() {
                crate::error::Failure::NodeStatusHealthOpenDenied {
                    path,
                    errno,
                    startup_ref: startup.receipt_ref.clone(),
                    config_ref: startup.config_ref.clone(),
                    binding_ref: binding_ref.clone(),
                }
            } else {
                crate::error::Failure::invalid_harness("node status health denial lacks one selected source binding")
            }
        }
        other => other,
    }
}

pub(super) fn status_request() -> crate::error::Result<crate::node_runtime::ControlRequest> {
    control_request("status")
}

pub(super) fn shutdown_request() -> crate::error::Result<crate::node_runtime::ControlRequest> {
    control_request("shutdown")
}

fn control_request(operation: &str) -> crate::error::Result<crate::node_runtime::ControlRequest> {
    let authority_refs = vec![super::storage::local_ref("node-control-authority", operation)?];
    let policy_refs = vec![super::storage::local_ref("node-control-policy", operation)?];
    let resource_refs = vec![super::storage::local_ref("node-control-resource", operation)?];
    let value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation,
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })?;
    crate::node_runtime::parse_control_request(&value)
}
