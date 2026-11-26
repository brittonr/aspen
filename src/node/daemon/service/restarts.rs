//! Existing service lock handling, restart denials, and service start.

pub(super) fn handle_existing_service_lock(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&super::outcomes::ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> crate::error::Result<super::serving::ExistingServiceLock> {
    let lock_path = crate::node_state::RelativePath::parse(super::settings::CONTROL_SERVICE_LOCK_FILE)?;
    if !state_root.try_exists(&lock_path)? {
        return Ok(super::serving::ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    if let Some(policy) = supervisor_policy
        && policy.stale_lock_recovery
    {
        let lock_value = super::storage::read_preserves(state_root, &lock_path)?;
        let stale_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
        let diagnostics = vec!["node control stale service lock recovered by supervisor policy".to_string()];
        let receipt_ref =
            super::writers::write_supervisor_receipt(state_root, &super::services::SupervisorReceiptValueInput {
                decision: "pass",
                operation: "stale-lock-recover",
                startup_receipt_ref: &startup.receipt_ref,
                service_lock_ref: Some(&stale_lock_ref),
                supervisor_policy_ref: Some(&policy.policy_ref),
                topic: input.topic,
                diagnostics: &diagnostics,
            })?;
        supervisor_receipt_refs.push(receipt_ref);
        state_root.remove_regular_file(&lock_path)?;
        return Ok(super::serving::ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    let denied = super::draining::denied_duplicate_service_run(
        state_root,
        input,
        startup,
        supervisor_policy,
        &supervisor_receipt_refs,
    )?;
    Ok(super::serving::ExistingServiceLock {
        supervisor_receipt_refs,
        denied: Some(denied),
    })
}

pub(super) struct RestartDenialInput<'a> {
    pub(super) startup: &'a crate::node_runtime::NodeStartupReceipt,
    pub(super) policy: &'a super::outcomes::ControlSupervisorPolicy,
    pub(super) prior_runs: u64,
    pub(super) inherited_supervisor_receipt_refs: &'a [String],
}

pub(super) fn denied_restart_attempt(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
    restart: RestartDenialInput<'_>,
) -> crate::error::Result<super::outcomes::ControlServe> {
    let RestartDenialInput {
        startup,
        policy,
        prior_runs,
        inherited_supervisor_receipt_refs,
    } = restart;
    let diagnostics = vec![format!(
        "node control supervisor restart attempts {prior_runs} exceeded bound {}",
        policy.max_restarts
    )];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    let receipt_ref =
        super::writers::write_supervisor_receipt(state_root, &super::services::SupervisorReceiptValueInput {
            decision: "deny",
            operation: "restart-attempt-deny",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: None,
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &diagnostics,
        })?;
    supervisor_receipt_refs.push(receipt_ref);
    let receipt_value = super::services::service_run_receipt_value(&super::services::ServiceRunReceiptValueInput {
        decision: "deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: None,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: 0,
        heartbeat_receipt_refs: &[],
        ingress_receipt_refs: &[],
        loop_receipt_refs: &[],
        processed_request_refs: &[],
        has_stopped: false,
        supervisor_policy_ref: Some(&policy.policy_ref),
        supervisor_receipt_refs: &supervisor_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_service_run_receipt_path(&service_receipt_ref)?,
        &receipt_value,
    )?;
    super::storage::import_artifact(state_root, &receipt_value)?;
    Ok(super::outcomes::ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: None,
        heartbeat_receipt_refs: Vec::new(),
        ingress_receipt_refs: Vec::new(),
        loop_receipt_refs: Vec::new(),
        processed_request_refs: Vec::new(),
        supervisor_policy_ref: Some(policy.policy_ref.clone()),
        supervisor_receipt_refs,
        ticks: 0,
        has_stopped: false,
        decision: "deny".to_string(),
    })
}

pub(super) fn start_service_run(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&super::outcomes::ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> crate::error::Result<super::serving::ServiceStart> {
    let identity = crate::node_identity::parse_identity(&super::storage::read_preserves(
        state_root,
        &crate::node_state::RelativePath::parse(super::settings::IDENTITY_FILE)?,
    )?)?;
    let service_run_id = super::storage::local_ref(
        "node-control-service-run",
        &format!("{}:{}:{}:{}", startup.receipt_ref, input.topic, input.max_ticks, input.max_requests_per_tick),
    )?;
    let lock_value = super::services::service_lock_value(&super::services::ServiceLockValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        node_id: &identity.node_id,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        service_run_ref: &service_run_id,
    })?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    super::storage::write_preserves(
        state_root,
        &crate::node_state::RelativePath::parse(super::settings::CONTROL_SERVICE_LOCK_FILE)?,
        &lock_value,
    )?;
    super::storage::import_artifact(state_root, &lock_value)?;
    if let Some(policy) = supervisor_policy {
        let receipt_ref =
            super::writers::write_supervisor_receipt(state_root, &super::services::SupervisorReceiptValueInput {
                decision: "pass",
                operation: "restart-attempt",
                startup_receipt_ref: &startup.receipt_ref,
                service_lock_ref: Some(&service_lock_ref),
                supervisor_policy_ref: Some(&policy.policy_ref),
                topic: input.topic,
                diagnostics: &[],
            })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    Ok(super::serving::ServiceStart {
        service_lock_ref,
        supervisor_receipt_refs,
    })
}
