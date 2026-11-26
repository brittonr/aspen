//! Service shutdown drain, finish, and lock removal.

pub(super) fn note_shutdown_drain(
    input: super::serving::ShutdownDrainInput<'_>,
) -> crate::error::Result<super::serving::ShutdownDrain> {
    let mut run = input.run;
    let mut supervisor_receipt_refs = input.supervisor_receipt_refs;
    if let Some(policy) = input.policy
        && run.has_stopped
    {
        let mut shutdown_diagnostics = Vec::new();
        if run.ticks > policy.shutdown_drain_ticks {
            let diagnostic = format!(
                "node control shutdown drain ticks {} exceeded supervisor bound {}",
                run.ticks, policy.shutdown_drain_ticks
            );
            run.diagnostics.push(diagnostic.clone());
            shutdown_diagnostics.push(diagnostic);
        }
        let receipt_ref = super::writers::write_supervisor_receipt(
            input.state_root,
            &super::services::SupervisorReceiptValueInput {
                decision: if shutdown_diagnostics.is_empty() {
                    "pass"
                } else {
                    "deny"
                },
                operation: "shutdown-drain",
                startup_receipt_ref: input.startup_receipt_ref,
                service_lock_ref: Some(input.service_lock_ref),
                supervisor_policy_ref: Some(&policy.policy_ref),
                topic: input.topic,
                diagnostics: &shutdown_diagnostics,
            },
        )?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    Ok(super::serving::ShutdownDrain {
        run,
        supervisor_receipt_refs,
    })
}

pub(super) fn finish_service_run(
    input: super::serving::FinishServiceInput<'_>,
) -> crate::error::Result<super::outcomes::ControlServe> {
    let decision = if input.run.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    let receipt_value = super::services::service_run_receipt_value(&super::services::ServiceRunReceiptValueInput {
        decision,
        startup_receipt_ref: input.startup_receipt_ref,
        service_lock_ref: Some(input.service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: input.run.ticks,
        heartbeat_receipt_refs: &input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: &input.run.ingress_receipt_refs,
        loop_receipt_refs: &input.run.loop_receipt_refs,
        processed_request_refs: &input.run.processed_request_refs,
        has_stopped: input.run.has_stopped,
        supervisor_policy_ref: input.supervisor_policy_ref,
        supervisor_receipt_refs: &input.supervisor_receipt_refs,
        diagnostics: &input.run.diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_service_run_receipt_path(&service_receipt_ref)?,
        &receipt_value,
    )?;
    super::storage::import_artifact(input.state_root, &receipt_value)?;
    Ok(super::outcomes::ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(input.service_lock_ref.to_string()),
        heartbeat_receipt_refs: input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: input.run.ingress_receipt_refs,
        loop_receipt_refs: input.run.loop_receipt_refs,
        processed_request_refs: input.run.processed_request_refs,
        supervisor_policy_ref: input.supervisor_policy_ref.map(|value| value.to_string()),
        supervisor_receipt_refs: input.supervisor_receipt_refs,
        ticks: input.run.ticks,
        has_stopped: input.run.has_stopped,
        decision: decision.to_string(),
    })
}

pub(super) fn denied_duplicate_service_run(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&super::outcomes::ControlSupervisorPolicy>,
    inherited_supervisor_receipt_refs: &[String],
) -> crate::error::Result<super::outcomes::ControlServe> {
    let lock_value = super::storage::read_preserves(
        state_root,
        &crate::node_state::RelativePath::parse(super::settings::CONTROL_SERVICE_LOCK_FILE)?,
    )?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let diagnostics = vec!["node control service runner already active".to_string()];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    if let Some(policy) = supervisor_policy {
        let receipt_ref =
            super::writers::write_supervisor_receipt(state_root, &super::services::SupervisorReceiptValueInput {
                decision: "deny",
                operation: "duplicate-runner-deny",
                startup_receipt_ref: &startup.receipt_ref,
                service_lock_ref: Some(&service_lock_ref),
                supervisor_policy_ref: Some(&policy.policy_ref),
                topic: input.topic,
                diagnostics: &diagnostics,
            })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    let receipt_value = super::services::service_run_receipt_value(&super::services::ServiceRunReceiptValueInput {
        decision: "deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: Some(&service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: 0,
        heartbeat_receipt_refs: &[],
        ingress_receipt_refs: &[],
        loop_receipt_refs: &[],
        processed_request_refs: &[],
        has_stopped: false,
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.as_str()),
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
        service_lock_ref: Some(service_lock_ref),
        heartbeat_receipt_refs: Vec::new(),
        ingress_receipt_refs: Vec::new(),
        loop_receipt_refs: Vec::new(),
        processed_request_refs: Vec::new(),
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.clone()),
        supervisor_receipt_refs,
        ticks: 0,
        has_stopped: false,
        decision: "deny".to_string(),
    })
}

pub(super) fn remove_service_lock(
    state_root: &crate::node_state::DirectoryRoot,
    service_lock_ref: &str,
) -> crate::error::Result<()> {
    let path = crate::node_state::RelativePath::parse(super::settings::CONTROL_SERVICE_LOCK_FILE)?;
    if !state_root.try_exists(&path)? {
        return Ok(());
    }
    let current_ref = crate::preserves_rail::canonical_hash(&super::storage::read_preserves(state_root, &path)?)?;
    if current_ref != service_lock_ref {
        return Err(crate::error::Failure::invalid_harness("node control service lock changed during serve"));
    }
    state_root.remove_regular_file(&path)
}

pub(super) fn ingress_receipt_decision(value: &preserves::IOValue) -> crate::error::Result<String> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    super::fields::record_string(&fields[1], "decision")
}
