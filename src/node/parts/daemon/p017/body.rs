
fn live_workflow_bundle_expected_value(input: &LiveWorkflowBundleExpectedInput<'_>) -> IoValue {
    crate::preserves_rail::record("expected", vec![crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("node", vec![optional_string(input.expected_node)]),
        crate::preserves_rail::record("topic", vec![optional_string(input.expected_topic)]),
        crate::preserves_rail::record("endpoint", vec![optional_string(input.expected_endpoint)]),
        crate::preserves_rail::record("peer", vec![optional_string(input.expected_peer)]),
        crate::preserves_rail::record("operations", vec![crate::preserves_rail::sequence(
            input.expected_operations.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("target-scope", vec![optional_string(input.expected_target_scope)]),
        crate::preserves_rail::record("resource-scope", vec![optional_string(input.expected_resource_scope)]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("as-of-epoch", vec![crate::preserves_rail::string(
            input.as_of_epoch.to_string(),
        )]),
    ])])
}

fn live_workflow_bundle_import_receipt_value(input: &LiveWorkflowBundleImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-workflow-bundle-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
        crate::preserves_rail::record("bundle", vec![crate::preserves_rail::string(&input.bundle.bundle_ref)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.bundle.ticket_ref)]),
        crate::preserves_rail::record("peer-admission", vec![crate::preserves_rail::string(
            &input.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant", vec![crate::preserves_rail::string(
            &input.bundle.authority_grant_ref,
        )]),
        crate::preserves_rail::record("ticket-import", vec![optional_string(input.ticket_import_ref)]),
        crate::preserves_rail::record("authority-import", vec![optional_string(input.authority_import_ref)]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-admission-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("authority-grant-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-receipt-imported"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bundle-import-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub fn control_supervisor_policy_value(input: &ControlSupervisorPolicyInput<'_>) -> Result<IoValue> {
    validate_supervisor_policy_bounds(input.max_restarts, "max restarts")?;
    validate_supervisor_policy_bounds(input.restart_window_ticks, "restart window ticks")?;
    validate_supervisor_policy_bounds(input.heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    validate_supervisor_policy_bounds(input.shutdown_drain_ticks, "shutdown drain ticks")?;
    validate_ingress_refs(input.policy_refs, "node control supervisor policy ref")?;
    validate_ingress_refs(input.evidence_refs, "node control supervisor evidence ref")?;
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

pub fn parse_control_supervisor_policy(value: &IoValue) -> Result<ControlSupervisorPolicy> {
    let fields = value
        .collect_simple_record("node-control-supervisor-policy-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-policy-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_POLICY_SCHEMA,
        "node control supervisor policy",
    )?;
    let has_stale_lock_recovery = match record_string(&fields[5], "stale-lock-recovery")?.as_str() {
        "allow" => true,
        "deny" => false,
        other => {
            return Err(MoltenError::invalid_harness(format!(
                "node control supervisor stale lock recovery must be allow or deny, got {other}"
            )));
        }
    };
    let max_restarts = record_u64_string(&fields[1], "max-restarts")?;
    let restart_window_ticks = record_u64_string(&fields[2], "restart-window-ticks")?;
    let heartbeat_timeout_ticks = record_u64_string(&fields[3], "heartbeat-timeout-ticks")?;
    let shutdown_drain_ticks = record_u64_string(&fields[4], "shutdown-drain-ticks")?;
    validate_supervisor_policy_bounds(max_restarts, "max restarts")?;
    validate_supervisor_policy_bounds(restart_window_ticks, "restart window ticks")?;
    validate_supervisor_policy_bounds(heartbeat_timeout_ticks, "heartbeat timeout ticks")?;
    validate_supervisor_policy_bounds(shutdown_drain_ticks, "shutdown drain ticks")?;
    Ok(ControlSupervisorPolicy {
        policy_ref: crate::preserves_rail::canonical_hash(value)?,
        max_restarts,
        restart_window_ticks,
        heartbeat_timeout_ticks,
        shutdown_drain_ticks,
        stale_lock_recovery: has_stale_lock_recovery,
        policy_refs: record_ref_strings(&fields[6], "policy")?,
        evidence_refs: record_ref_strings(&fields[7], "evidence")?,
        value: value.clone(),
    })
}

pub fn import_control_supervisor_policy(state_root: &Path, policy_value: &IoValue) -> Result<ControlSupervisorPolicy> {
    validate_state_root(state_root)?;
    ensure_state_layout(state_root)?;
    let policy = parse_control_supervisor_policy(policy_value)?;
    import_artifact(state_root, policy_value)?;
    Ok(policy)
}

fn parse_control_supervisor_receipt(value: &IoValue) -> Result<ControlSupervisorReceipt> {
    let fields = value
        .collect_simple_record("node-control-supervisor-receipt-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-supervisor-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA,
        "node control supervisor receipt",
    )?;
    Ok(ControlSupervisorReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        operation: record_string(&fields[2], "operation")?,
        supervisor_policy_ref: record_optional_string(&fields[5], "policy")?,
        diagnostics: record_strings(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

fn service_run_supervisor_policy_ref(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return record_optional_string(&fields[13], "supervisor-policy");
    }
    Ok(None)
}

fn count_prior_supervised_service_runs(state_root: &Path, supervisor_policy_ref: &str) -> Result<u64> {
    let service_dir = state_root.join(CONTROL_SERVICE_DIR);
    if !service_dir.exists() {
        return Ok(0);
    }
    let mut count = 0_u64;
    for entry in fs::read_dir(&service_dir)
        .map_err(|error| MoltenError::invalid_harness(format!("read node control service dir failed: {error}")))?
    {
        let entry = entry.map_err(|error| {
            MoltenError::invalid_harness(format!("read node control service entry failed: {error}"))
        })?;
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".service-run-receipt.preserves"))
        {
            continue;
        }
        let value = read_preserves(&path)?;
        if service_run_supervisor_policy_ref(&value)?.as_deref() == Some(supervisor_policy_ref) {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

pub fn init_local(input: &InitInput<'_>) -> Result<Init> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.node_id)?;
    ensure_state_layout(input.state_root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, input.state_root.join("identity"));
    identity_config.policy_refs = policy_refs.clone();
    let identity_resolution = crate::node_identity::resolve(&identity_config)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let adapters = default_adapter_bindings(input.state_root)?;
    let capability_refs = vec![local_ref("node-capability", input.node_id)?];
    let resource_refs = vec![local_ref("node-resource", input.node_id)?];
    let effect_profile_refs = vec![local_ref("node-effect-profile", input.node_id)?];
    let state_root_ref = state_root_profile_ref(input.state_root)?;
    let profile_resolution = crate::node_profile_config::resolve_local_default_config(
        &crate::node_profile_config::LocalDefaultConfigInput {
            identity_ref: identity.identity_ref.clone(),
            state_root_ref,
            adapters,
            policy_refs,
            capability_refs,
            resource_refs,
            effect_profile_refs,
        },
    )?;
    write_preserves(&input.state_root.join(CONFIG_FILE), &profile_resolution.config_value)?;
    write_preserves(
        &input.state_root.join(PROFILE_RESOLUTION_FILE),
        &profile_resolution.resolution_value,
    )?;
    write_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE), &identity_resolution.receipt_value)?;
    write_preserves(&input.state_root.join(IDENTITY_FILE), &identity.value)?;
    Ok(Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}

pub fn init_with_profile(input: &ProfileInitInput<'_>) -> Result<Init> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.node_id)?;
    ensure_state_layout(input.state_root)?;
    let policy_refs = vec![local_ref("node-policy", input.node_id)?];
    let mut identity_config = crate::node_identity::Config::new(input.node_id, input.state_root.join("identity"));
    identity_config.policy_refs = policy_refs;
    let identity_resolution = crate::node_identity::resolve(&identity_config)?;
    let identity = identity_resolution
        .identity
        .ok_or_else(|| MoltenError::invalid_harness("node daemon identity resolution denied"))?;
    let profile_resolution = crate::node_profile_config::resolve_profile_backed_config(
        &crate::node_profile_config::ProfileBackedConfigInput {
            identity_ref: identity.identity_ref.clone(),
            profile: input.profile.clone(),
            overrides: input.overrides.clone(),
        },
    )?;
    if profile_resolution.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node profile-backed init denied: {}",
            profile_resolution.diagnostics.join("; ")
        )));
    }
    write_preserves(&input.state_root.join(CONFIG_FILE), &profile_resolution.config_value)?;
    write_preserves(
        &input.state_root.join(PROFILE_RESOLUTION_FILE),
        &profile_resolution.resolution_value,
    )?;
    write_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE), &identity_resolution.receipt_value)?;
    write_preserves(&input.state_root.join(IDENTITY_FILE), &identity.value)?;
    Ok(Init {
        config_ref: profile_resolution.config_ref,
        identity_ref: identity.identity_ref,
        identity_receipt_ref: identity_resolution.receipt_ref,
        profile_resolution_ref: profile_resolution.resolution_ref,
        config_value: profile_resolution.config_value,
        identity_receipt_value: identity_resolution.receipt_value,
        profile_resolution_value: profile_resolution.resolution_value,
    })
}
