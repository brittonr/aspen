//! Local stop and the admitted shutdown plan.

pub fn stop_local(input: &super::requests::StopInput<'_>) -> crate::error::Result<super::outcomes::Stop> {
    super::storage::validate_state_root(input.state_root)?;
    let root = crate::node_state::DirectoryRoot::open_existing(input.state_root)?;
    stop_local_with_root(&root)
}

pub fn stop_local_with_root(root: &crate::node_state::DirectoryRoot) -> crate::error::Result<super::outcomes::Stop> {
    let request = super::running::shutdown_request()?;
    stop_local_node_with_request(root, &request)
}

// r[impl molten.audit_f01.admission]
// r[impl molten.audit_f01.preserve_state]
fn stop_local_node_with_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
) -> crate::error::Result<super::outcomes::Stop> {
    let startup = super::leases::current_startup_receipt(root)?;
    let admission = admit_shutdown_request(root, request, &startup)?;
    let Some(plan) = admission.plan else {
        let denial_ref = write_shutdown_denial(root, request, &startup.receipt_ref, &admission.diagnostics)?;
        return Err(crate::error::Failure::invalid_harness(format!(
            "node shutdown denied: {} (denial receipt={})",
            admission.diagnostics.join("; "),
            denial_ref
        )));
    };
    execute_shutdown_plan(root, request, &plan)
}

pub(super) fn admit_shutdown_request(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    startup: &crate::node_runtime::NodeStartupReceipt,
) -> crate::error::Result<crate::node_runtime::ShutdownAdmission> {
    let has_active_lock = root.try_exists(&super::storage::fixed_node_path(super::settings::CONTROL_LOCK_FILE)?)?;
    crate::node_runtime::admit_node_shutdown(&crate::node_runtime::ShutdownAdmissionInput {
        request,
        startup_receipt_ref: &startup.receipt_ref,
        adapter_receipts: &startup.adapters,
        has_active_lock,
    })
}

fn write_shutdown_denial(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    startup_receipt_ref: &str,
    diagnostics: &[String],
) -> crate::error::Result<String> {
    let control_receipt_value =
        super::gating::control_receipt_for_request(root, request, startup_receipt_ref, &[], diagnostics)?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONTROL_STOP_FILE)?,
        &control_receipt_value,
    )?;
    super::storage::import_artifact(root, &control_receipt_value)?;
    Ok(control_receipt_ref)
}

// r[impl molten.audit_f01.observed_effects]
pub(super) fn execute_shutdown_plan(
    root: &crate::node_state::DirectoryRoot,
    request: &crate::node_runtime::ControlRequest,
    plan: &crate::node_runtime::ShutdownPlan,
) -> crate::error::Result<super::outcomes::Stop> {
    let binding = super::tracking::admit_current_run_shutdown_observations(root, &plan.startup_receipt_ref)?;
    let shutdown_adapters = write_adapter_shutdown_receipts(root, plan)?;
    let index_refs = super::storage::index_receipt_refs(root)?;
    let shutdown_value =
        crate::node_runtime::node_shutdown_receipt_value(&crate::node_runtime::ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &plan.startup_receipt_ref,
            adapter_receipts: &shutdown_adapters,
            drained_job_refs: &[],
            index_receipt_refs: &index_refs,
            diagnostics: &[],
        })?;
    let shutdown_ref = crate::preserves_rail::canonical_hash(&shutdown_value)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::SHUTDOWN_FILE)?,
        &shutdown_value,
    )?;
    super::storage::import_artifact(root, &shutdown_value)?;
    let control_receipt_value = super::gating::control_receipt_for_request(
        root,
        request,
        &plan.startup_receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    super::storage::write_preserves(
        root,
        &super::storage::fixed_node_path(super::settings::CONTROL_STOP_FILE)?,
        &control_receipt_value,
    )?;
    super::storage::import_artifact(root, &control_receipt_value)?;
    super::tracking::write_node_run_shutdown_observation(
        root,
        &crate::node_runtime::NodeRunShutdownObservationValueInput {
            run_ref: &binding.run_ref,
            startup_receipt_ref: &plan.startup_receipt_ref,
            shutdown_receipt_ref: &shutdown_ref,
            control_receipt_ref: &control_receipt_ref,
        },
    )?;
    super::leases::remove_active_lock(root)?;
    Ok(super::outcomes::Stop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

fn write_adapter_shutdown_receipts(
    root: &crate::node_state::DirectoryRoot,
    plan: &crate::node_runtime::ShutdownPlan,
) -> crate::error::Result<Vec<crate::node_runtime::NodeAdapterReceiptRef>> {
    let mut shutdown_adapters = Vec::with_capacity(plan.adapter_receipts.len());
    for adapter in plan.adapter_receipts.iter() {
        let binding = crate::node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = crate::node_runtime::node_adapter_lifecycle_receipt_value(
            &crate::node_runtime::AdapterLifecycleReceiptInput {
                operation: "shutdown",
                decision: "pass",
                adapter: &binding,
                index_receipt_refs: &super::storage::index_receipt_refs(root)?,
                resource_receipt_refs: &super::storage::resource_receipt_refs(root)?,
                diagnostics: &[],
            },
        )?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
        let path = super::paths::node_leaf_path("receipts", &format!("adapter-shutdown-{}.preserves", adapter.name))?;
        super::storage::write_preserves(root, &path, &value)?;
        super::storage::import_artifact(root, &value)?;
        shutdown_adapters.push(crate::node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    Ok(shutdown_adapters)
}
