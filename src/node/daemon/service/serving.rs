//! Control service entry and service-run working records.

pub fn serve_control(
    input: &super::requests::ControlServeInput<'_>,
) -> crate::error::Result<super::outcomes::ControlServe> {
    let state_root = crate::node_state::DirectoryRoot::open(input.state_root)?;
    super::storage::validate_state_root(input.state_root)?;
    serve_control_with_root(&state_root, input)
}

pub(super) fn serve_control_with_root(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
) -> crate::error::Result<super::outcomes::ControlServe> {
    super::storage::validate_node_id(input.topic)?;
    super::storage::ensure_state_layout(state_root)?;
    let max_ticks = super::guards::validate_service_tick_limit(input.max_ticks)?;
    let max_requests_per_tick = super::guards::validate_loop_request_limit(input.max_requests_per_tick)?;
    let supervisor_policy = input
        .supervisor_policy_value
        .map(|value| super::supervision::import_control_supervisor_policy_with_root(state_root, value))
        .transpose()?;
    super::leases::require_active_lock(state_root)?;
    let startup = super::leases::current_startup_receipt(state_root)?;

    let existing_lock = super::restarts::handle_existing_service_lock(
        state_root,
        input,
        &startup,
        supervisor_policy.as_ref(),
        Vec::new(),
    )?;
    if let Some(denied) = existing_lock.denied {
        return Ok(denied);
    }
    if let Some(denied) = exhausted_restart_denial(
        state_root,
        input,
        &startup,
        supervisor_policy.as_ref(),
        &existing_lock.supervisor_receipt_refs,
    )? {
        return Ok(denied);
    }

    let start = super::restarts::start_service_run(
        state_root,
        input,
        &startup,
        supervisor_policy.as_ref(),
        existing_lock.supervisor_receipt_refs,
    )?;
    let run = super::ticking::run_service_ticks(ServiceTickInput {
        state_root,
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        tick_capacity: max_ticks,
        event_capacity: max_ticks.saturating_mul(max_requests_per_tick),
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
    })?;
    let shutdown = super::draining::note_shutdown_drain(ShutdownDrainInput {
        state_root,
        topic: input.topic,
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: &start.service_lock_ref,
        policy: supervisor_policy.as_ref(),
        run,
        supervisor_receipt_refs: start.supervisor_receipt_refs,
    })?;
    super::draining::remove_service_lock(state_root, &start.service_lock_ref)?;
    super::draining::finish_service_run(FinishServiceInput {
        state_root,
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

fn exhausted_restart_denial(
    state_root: &crate::node_state::DirectoryRoot,
    input: &super::requests::ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&super::outcomes::ControlSupervisorPolicy>,
    inherited_supervisor_receipt_refs: &[String],
) -> crate::error::Result<Option<super::outcomes::ControlServe>> {
    let Some(policy) = supervisor_policy else {
        return Ok(None);
    };
    let prior_runs = super::supervision::count_prior_supervised_service_runs(state_root, &policy.policy_ref)?;
    if prior_runs > policy.max_restarts {
        return super::restarts::denied_restart_attempt(state_root, input, super::restarts::RestartDenialInput {
            startup,
            policy,
            prior_runs,
            inherited_supervisor_receipt_refs,
        })
        .map(Some);
    }
    Ok(None)
}

pub(super) struct ExistingServiceLock {
    pub(super) supervisor_receipt_refs: Vec<String>,
    pub(super) denied: Option<super::outcomes::ControlServe>,
}

pub(super) struct ServiceStart {
    pub(super) service_lock_ref: String,
    pub(super) supervisor_receipt_refs: Vec<String>,
}

pub(super) struct ServiceTickInput<'a> {
    pub(super) state_root: &'a crate::node_state::DirectoryRoot,
    pub(super) topic: &'a str,
    pub(super) max_ticks: u64,
    pub(super) max_requests_per_tick: u64,
    pub(super) tick_capacity: usize,
    pub(super) event_capacity: usize,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: &'a str,
}

pub(super) struct ServiceRunParts {
    pub(super) heartbeat_receipt_refs: Vec<String>,
    pub(super) ingress_receipt_refs: Vec<String>,
    pub(super) loop_receipt_refs: Vec<String>,
    pub(super) processed_request_refs: Vec<String>,
    pub(super) diagnostics: Vec<String>,
    pub(super) ticks: u64,
    pub(super) has_stopped: bool,
}

pub(super) struct ShutdownDrainInput<'a> {
    pub(super) state_root: &'a crate::node_state::DirectoryRoot,
    pub(super) topic: &'a str,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: &'a str,
    pub(super) policy: Option<&'a super::outcomes::ControlSupervisorPolicy>,
    pub(super) run: ServiceRunParts,
    pub(super) supervisor_receipt_refs: Vec<String>,
}

pub(super) struct ShutdownDrain {
    pub(super) run: ServiceRunParts,
    pub(super) supervisor_receipt_refs: Vec<String>,
}

pub(super) struct FinishServiceInput<'a> {
    pub(super) state_root: &'a crate::node_state::DirectoryRoot,
    pub(super) topic: &'a str,
    pub(super) max_ticks: u64,
    pub(super) max_requests_per_tick: u64,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: &'a str,
    pub(super) supervisor_policy_ref: Option<&'a str>,
    pub(super) supervisor_receipt_refs: Vec<String>,
    pub(super) run: ServiceRunParts,
}
