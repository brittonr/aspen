
pub fn serve_control(input: &ControlServeInput<'_>) -> Result<ControlServe> {
    validate_state_root(input.state_root)?;
    validate_node_id(input.topic)?;
    ensure_state_layout(input.state_root)?;
    let max_ticks = validate_service_tick_limit(input.max_ticks)?;
    let max_requests_per_tick = validate_loop_request_limit(input.max_requests_per_tick)?;
    let supervisor_policy = input
        .supervisor_policy_value
        .map(|value| import_control_supervisor_policy(input.state_root, value))
        .transpose()?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;

    let existing_lock = handle_existing_service_lock(input, &startup, supervisor_policy.as_ref(), Vec::new())?;
    if let Some(denied) = existing_lock.denied {
        return Ok(denied);
    }
    if let Some(policy) = supervisor_policy.as_ref() {
        let prior_runs = count_prior_supervised_service_runs(input.state_root, &policy.policy_ref)?;
        if prior_runs > policy.max_restarts {
            return denied_restart_attempt(input, &startup, policy, prior_runs, &existing_lock.supervisor_receipt_refs);
        }
    }

    let start = start_service_run(input, &startup, supervisor_policy.as_ref(), existing_lock.supervisor_receipt_refs)?;
    let run = run_service_ticks(ServiceTickInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        tick_capacity: max_ticks,
        event_capacity: max_ticks.saturating_mul(max_requests_per_tick),
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
    })?;
    let shutdown = note_shutdown_drain(ShutdownDrainInput {
        state_root: input.state_root,
        topic: input.topic,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
        policy: supervisor_policy.as_ref(),
        run,
        supervisor_receipt_refs: start.supervisor_receipt_refs,
    })?;
    remove_service_lock(input.state_root, &start.service_lock_ref)?;
    finish_service_run(FinishServiceInput {
        state_root: input.state_root,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
        supervisor_policy_ref: supervisor_policy.as_ref().map(|policy| policy.policy_ref.as_str()),
        supervisor_receipt_refs: shutdown.supervisor_receipt_refs,
        run: shutdown.run,
    })
}

struct ExistingServiceLock {
    supervisor_receipt_refs: Vec<String>,
    denied: Option<ControlServe>,
}

struct ServiceStart {
    service_lock_ref: String,
    supervisor_receipt_refs: Vec<String>,
}

struct ServiceTickInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    tick_capacity: usize,
    event_capacity: usize,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
}

struct ServiceRunParts {
    heartbeat_receipt_refs: Vec<String>,
    ingress_receipt_refs: Vec<String>,
    loop_receipt_refs: Vec<String>,
    processed_request_refs: Vec<String>,
    diagnostics: Vec<String>,
    ticks: u64,
    has_stopped: bool,
}

struct ShutdownDrainInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    policy: Option<&'a ControlSupervisorPolicy>,
    run: ServiceRunParts,
    supervisor_receipt_refs: Vec<String>,
}

struct ShutdownDrain {
    run: ServiceRunParts,
    supervisor_receipt_refs: Vec<String>,
}

struct FinishServiceInput<'a> {
    state_root: &'a Path,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    supervisor_policy_ref: Option<&'a str>,
    supervisor_receipt_refs: Vec<String>,
    run: ServiceRunParts,
}

fn handle_existing_service_lock(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> Result<ExistingServiceLock> {
    if !input.state_root.join(CONTROL_SERVICE_LOCK_FILE).exists() {
        return Ok(ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    if let Some(policy) = supervisor_policy
        && policy.stale_lock_recovery
    {
        let lock_value = read_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE))?;
        let stale_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
        let diagnostics = vec!["node control stale service lock recovered by supervisor policy".to_string()];
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: "pass",
            operation: "stale-lock-recover",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&stale_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
        fs::remove_file(input.state_root.join(CONTROL_SERVICE_LOCK_FILE)).map_err(MoltenError::from)?;
        return Ok(ExistingServiceLock {
            supervisor_receipt_refs,
            denied: None,
        });
    }
    let denied = denied_duplicate_service_run(input, startup, supervisor_policy, &supervisor_receipt_refs)?;
    Ok(ExistingServiceLock {
        supervisor_receipt_refs,
        denied: Some(denied),
    })
}

fn denied_restart_attempt(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    policy: &ControlSupervisorPolicy,
    prior_runs: u64,
    inherited_supervisor_receipt_refs: &[String],
) -> Result<ControlServe> {
    let diagnostics = vec![format!(
        "node control supervisor restart attempts {prior_runs} exceeded bound {}",
        policy.max_restarts
    )];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
        decision: "deny",
        operation: "restart-attempt-deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: None,
        supervisor_policy_ref: Some(&policy.policy_ref),
        topic: input.topic,
        diagnostics: &diagnostics,
    })?;
    supervisor_receipt_refs.push(receipt_ref);
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
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
    write_preserves(&control_service_run_receipt_path(input.state_root, &service_receipt_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlServe {
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

fn start_service_run(
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    mut supervisor_receipt_refs: Vec<String>,
) -> Result<ServiceStart> {
    let identity = crate::node_identity::parse_identity(&read_preserves(&input.state_root.join(IDENTITY_FILE))?)?;
    let service_run_id = local_ref(
        "node-control-service-run",
        &format!("{}:{}:{}:{}", startup.receipt_ref, input.topic, input.max_ticks, input.max_requests_per_tick),
    )?;
    let lock_value = service_lock_value(&ServiceLockValueInput {
        state_root: input.state_root,
        startup_receipt_ref: &startup.receipt_ref,
        node_id: &identity.node_id,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        service_run_ref: &service_run_id,
    })?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    write_preserves(&input.state_root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value)?;
    import_artifact(input.state_root, &lock_value)?;
    if let Some(policy) = supervisor_policy {
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
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
    Ok(ServiceStart {
        service_lock_ref,
        supervisor_receipt_refs,
    })
}

fn run_service_ticks(input: ServiceTickInput<'_>) -> Result<ServiceRunParts> {
    let mut run = ServiceRunParts {
        heartbeat_receipt_refs: Vec::with_capacity(input.tick_capacity),
        ingress_receipt_refs: Vec::with_capacity(input.event_capacity),
        loop_receipt_refs: Vec::with_capacity(input.tick_capacity),
        processed_request_refs: Vec::with_capacity(input.event_capacity),
        diagnostics: Vec::with_capacity(input.tick_capacity.saturating_mul(2)),
        ticks: 0,
        has_stopped: false,
    };

    for tick in 0..input.max_ticks {
        run.ticks = tick + 1;
        if run_service_tick(&input, &mut run, tick)? {
            break;
        }
    }
    if !run.has_stopped {
        match has_pending_service_work(input.state_root, input.topic) {
            Ok(true) => run.diagnostics.push("node control service reached max ticks with pending work".to_string()),
            Ok(false) => {}
            Err(error) => run.diagnostics.push(format!("node control service pending-work scan failed: {error}")),
        }
    }
    Ok(run)
}

fn run_service_tick(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts, tick: u64) -> Result<bool> {
    write_service_heartbeat(input, run, tick)?;
    if deliver_service_ingress(input, run)? {
        return Ok(true);
    }
    process_service_loop(input, run)
}
