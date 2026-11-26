//! Supervisor policy encoding, parsing, import, and prior-run accounting.

pub fn control_supervisor_policy_value(
    input: &super::requests::ControlSupervisorPolicyInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_supervisor_policy_bounds(input.max_restarts, "max restarts")?;
    super::guards::validate_supervisor_policy_bounds(input.restart_window_ticks, "restart window ticks")?;
    super::guards::validate_supervisor_policy_bounds(input.heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    super::guards::validate_supervisor_policy_bounds(input.shutdown_drain_ticks, "shutdown drain ticks")?;
    super::guards::validate_ingress_refs(input.policy_refs, "node control supervisor policy ref")?;
    super::guards::validate_ingress_refs(input.evidence_refs, "node control supervisor evidence ref")?;
    Ok(crate::preserves_rail::record("node-control-supervisor-policy-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA),
        crate::preserves_rail::record("max-restarts", vec![crate::preserves_rail::string(
            input.max_restarts.to_string(),
        )]),
        crate::preserves_rail::record("restart-window-ticks", vec![crate::preserves_rail::string(
            input.restart_window_ticks.to_string(),
        )]),
        crate::preserves_rail::record("heartbeat-timeout-ticks", vec![crate::preserves_rail::string(
            input.heartbeat_timeout_ticks.to_string(),
        )]),
        crate::preserves_rail::record("shutdown-drain-ticks", vec![crate::preserves_rail::string(
            input.shutdown_drain_ticks.to_string(),
        )]),
        crate::preserves_rail::record("stale-lock-recovery", vec![crate::preserves_rail::string(
            if input.stale_lock_recovery { "allow" } else { "deny" },
        )]),
        crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-restarts"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-heartbeat-timeout"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-stale-lock-policy"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-drain-bound"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn parse_control_supervisor_policy(
    value: &preserves::IOValue,
) -> crate::error::Result<super::outcomes::ControlSupervisorPolicy> {
    let fields = value
        .collect_simple_record("node-control-supervisor-policy-v1", Some(9))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-supervisor-policy-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA,
        "node control supervisor policy",
    )?;
    let has_stale_lock_recovery = match super::fields::record_string(&fields[5], "stale-lock-recovery")?.as_str() {
        "allow" => true,
        "deny" => false,
        other => {
            return Err(crate::error::Failure::invalid_harness(format!(
                "node control supervisor stale lock recovery must be allow or deny, got {other}"
            )));
        }
    };
    let max_restarts = super::fields::record_u64_string(&fields[1], "max-restarts")?;
    let restart_window_ticks = super::fields::record_u64_string(&fields[2], "restart-window-ticks")?;
    let heartbeat_timeout_ticks = super::fields::record_u64_string(&fields[3], "heartbeat-timeout-ticks")?;
    let shutdown_drain_ticks = super::fields::record_u64_string(&fields[4], "shutdown-drain-ticks")?;
    super::guards::validate_supervisor_policy_bounds(max_restarts, "max restarts")?;
    super::guards::validate_supervisor_policy_bounds(restart_window_ticks, "restart window ticks")?;
    super::guards::validate_supervisor_policy_bounds(heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    super::guards::validate_supervisor_policy_bounds(shutdown_drain_ticks, "shutdown drain ticks")?;
    Ok(super::outcomes::ControlSupervisorPolicy {
        policy_ref: crate::preserves_rail::canonical_hash(value)?,
        max_restarts,
        restart_window_ticks,
        heartbeat_timeout_ticks,
        shutdown_drain_ticks,
        stale_lock_recovery: has_stale_lock_recovery,
        policy_refs: super::fields::record_ref_strings(&fields[6], "policy")?,
        evidence_refs: super::fields::record_ref_strings(&fields[7], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_supervisor_policy(
    state_root: &std::path::Path,
    policy_value: &preserves::IOValue,
) -> crate::error::Result<super::outcomes::ControlSupervisorPolicy> {
    super::storage::validate_state_root(state_root)?;
    let root = crate::node_state::DirectoryRoot::open(state_root)?;
    import_control_supervisor_policy_with_root(&root, policy_value)
}

pub fn import_control_supervisor_policy_with_root(
    root: &crate::node_state::DirectoryRoot,
    policy_value: &preserves::IOValue,
) -> crate::error::Result<super::outcomes::ControlSupervisorPolicy> {
    super::storage::ensure_state_layout(root)?;
    let policy = parse_control_supervisor_policy(policy_value)?;
    super::storage::import_artifact(root, policy_value)?;
    Ok(policy)
}

pub(super) fn parse_control_supervisor_receipt(
    value: &preserves::IOValue,
) -> crate::error::Result<super::outcomes::ControlSupervisorReceipt> {
    let fields = value
        .collect_simple_record("node-control-supervisor-receipt-v1", Some(9))
        .ok_or_else(|| crate::error::Failure::invalid_harness("expected <node-control-supervisor-receipt-v1 ...>"))?;
    super::guards::require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA,
        "node control supervisor receipt",
    )?;
    Ok(super::outcomes::ControlSupervisorReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: super::fields::record_string(&fields[1], "decision")?,
        operation: super::fields::record_string(&fields[2], "operation")?,
        supervisor_policy_ref: super::fields::record_optional_string(&fields[5], "policy")?,
        diagnostics: super::fields::record_strings(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

fn service_run_supervisor_policy_ref(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return super::fields::record_optional_string(&fields[13], "supervisor-policy");
    }
    Ok(None)
}

pub(super) fn count_prior_supervised_service_runs(
    root: &crate::node_state::DirectoryRoot,
    supervisor_policy_ref: &str,
) -> crate::error::Result<u64> {
    let service = root.control_service()?;
    let mut count = 0_u64;
    for entry in service.list_entries()? {
        if entry.kind != crate::node_state::EntryKind::RegularFile
            || !entry.name.ends_with(".service-run-receipt.preserves")
        {
            continue;
        }
        let bytes = service.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            crate::error::Failure::invalid_harness(format!("node service receipt is not UTF-8: {error}"))
        })?;
        let value = crate::preserves_rail::parse_text(&text)?;
        if service_run_supervisor_policy_ref(&value)?.as_deref() == Some(supervisor_policy_ref) {
            count = count
                .checked_add(1)
                .ok_or_else(|| crate::error::Failure::invalid_harness("node supervised service run count overflow"))?;
        }
    }
    Ok(count)
}
