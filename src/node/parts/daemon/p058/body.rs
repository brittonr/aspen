const CURRENT_RUN_FILE: &str = "control/current-run.preserves";
const CURRENT_RUN_SHUTDOWN_FILE: &str = "control/current-run-shutdown.preserves";

#[derive(Debug)]
struct ObservedControlDispatch {
    dispatch: ControlDispatch,
    is_duplicate: bool,
}

struct NodeRunObservations {
    binding: crate::node_runtime::NodeRunBinding,
    active_run_ref: Option<String>,
    shutdown_observation: Option<crate::node_runtime::NodeRunShutdownObservation>,
    shutdown_receipt: Option<crate::node_runtime::NodeShutdownReceipt>,
}

impl NodeRunObservations {
    fn classify(
        &self,
        operation: &str,
        control_receipt: Option<&crate::node_runtime::ControlReceipt>,
        is_duplicate: bool,
    ) -> crate::node_runtime::NodeRunLifecycleOutcome {
        crate::node_runtime::classify_node_run_lifecycle(&crate::node_runtime::NodeRunLifecycleInput {
            operation,
            is_duplicate,
            current_run_ref: &self.binding.run_ref,
            current_startup_receipt_ref: &self.binding.startup_receipt_ref,
            active_run_ref: self.active_run_ref.as_deref(),
            shutdown_observation: self.shutdown_observation.as_ref(),
            shutdown_receipt: self.shutdown_receipt.as_ref(),
            control_receipt,
        })
    }
}

fn read_optional_node_artifact(root: &crate::node_state::DirectoryRoot, name: &str) -> Result<Option<IoValue>> {
    let path = fixed_node_path(name)?;
    if root.try_exists(&path)? {
        Ok(Some(read_preserves(root, &path)?))
    } else {
        Ok(None)
    }
}

// r[impl molten.audit_f14.current_binding]
fn current_node_run_binding(root: &crate::node_state::DirectoryRoot) -> Result<crate::node_runtime::NodeRunBinding> {
    let value = read_preserves(root, &fixed_node_path(CURRENT_RUN_FILE)?)?;
    let binding = crate::node_runtime::parse_node_run_binding(&value)?;
    validate_member_ref(&binding.state_root_ref, &state_root_profile_ref(root)?, "node run state root")?;
    Ok(binding)
}

fn begin_node_run_binding(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
) -> Result<crate::node_runtime::NodeRunBinding> {
    let previous = read_optional_node_artifact(root, CURRENT_RUN_FILE)?
        .as_ref()
        .map(crate::node_runtime::parse_node_run_binding)
        .transpose()?;
    let state_root_ref = state_root_profile_ref(root)?;
    if let Some(previous) = &previous {
        validate_member_ref(&previous.state_root_ref, &state_root_ref, "previous node run state root")?;
    }
    let value = crate::node_runtime::node_run_binding_value(&crate::node_runtime::NodeRunBindingValueInput {
        state_root_ref: &state_root_ref,
        startup_receipt_ref,
        previous_run_ref: previous.as_ref().map(|binding| binding.run_ref.as_str()),
    })?;
    let binding = crate::node_runtime::parse_node_run_binding(&value)?;
    write_preserves(root, &fixed_node_path(CURRENT_RUN_FILE)?, &value)?;
    import_artifact(root, &value)?;
    let completion_path = fixed_node_path(CURRENT_RUN_SHUTDOWN_FILE)?;
    if root.try_exists(&completion_path)? {
        root.remove_regular_file(&completion_path)?;
    }
    Ok(binding)
}

fn active_lock_run_ref(value: &IoValue, binding: &crate::node_runtime::NodeRunBinding) -> Result<String> {
    let fields = value
        .collect_simple_record("node-control-lock-v2", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-lock-v2 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA_V2, "node control lock")?;
    validate_member_ref(&record_string(&fields[1], "state-root")?, &binding.state_root_ref, "node lock state root")?;
    validate_member_ref(&record_string(&fields[2], "startup")?, &binding.startup_receipt_ref, "node lock startup")?;
    let run_ref = record_string(&fields[6], "run")?;
    validate_ingress_ref(&run_ref, "node lock run ref")?;
    Ok(run_ref)
}

// r[impl molten.audit_f14.preserve_state]
fn read_node_run_observations(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
) -> Result<NodeRunObservations> {
    let binding = current_node_run_binding(root)?;
    validate_member_ref(&binding.startup_receipt_ref, startup_receipt_ref, "current node run startup")?;
    let active_run_ref = read_optional_node_artifact(root, CONTROL_LOCK_FILE)?
        .as_ref()
        .map(|value| active_lock_run_ref(value, &binding))
        .transpose()?;
    let shutdown_observation = read_optional_node_artifact(root, CURRENT_RUN_SHUTDOWN_FILE)?
        .as_ref()
        .map(crate::node_runtime::parse_node_run_shutdown_observation)
        .transpose()?;
    let shutdown_receipt = read_optional_node_artifact(root, SHUTDOWN_FILE)?
        .as_ref()
        .map(crate::node_runtime::parse_node_shutdown_receipt)
        .transpose()?;
    Ok(NodeRunObservations {
        binding,
        active_run_ref,
        shutdown_observation,
        shutdown_receipt,
    })
}

fn completed_node_run_is_stopped(root: &crate::node_state::DirectoryRoot, startup_receipt_ref: &str) -> Result<bool> {
    let observations = read_node_run_observations(root, startup_receipt_ref)?;
    let control = completed_node_run_control(root, &observations)?;
    Ok(observations.classify("shutdown", control.as_ref(), false).lifecycle
        == crate::node_runtime::NodeRunLifecycle::Stopped)
}

fn write_node_run_shutdown_observation(
    root: &crate::node_state::DirectoryRoot,
    input: &crate::node_runtime::NodeRunShutdownObservationValueInput<'_>,
) -> Result<()> {
    let value = crate::node_runtime::node_run_shutdown_observation_value(input)?;
    write_preserves(root, &fixed_node_path(CURRENT_RUN_SHUTDOWN_FILE)?, &value)?;
    import_artifact(root, &value)?;
    Ok(())
}

fn admit_current_run_shutdown_observations(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
) -> Result<crate::node_runtime::NodeRunBinding> {
    let observations = read_node_run_observations(root, startup_receipt_ref)?;
    if observations.active_run_ref.as_deref() != Some(observations.binding.run_ref.as_str())
        || observations.shutdown_observation.is_some()
        || observations.shutdown_receipt.is_some()
    {
        return Err(MoltenError::invalid_harness("node shutdown requires consistent current-run active observations"));
    }
    Ok(observations.binding)
}

fn note_service_shutdown_observation(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts) {
    match completed_node_run_is_stopped(input.state_root, input.startup_receipt_ref) {
        Ok(true) => run.has_stopped = true,
        Ok(false) => run
            .diagnostics
            .push("node control service has unresolved current-run shutdown observations".to_string()),
        Err(error) => run.diagnostics.push(format!("node control service shutdown observation failed: {error}")),
    }
}

fn run_state_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-lock-v2", Some(7)) {
        require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA_V2, "node control lock")?;
        return Ok(Some(format!(
            "node control lock startup={} owner={} run={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[3], "owner")?,
            record_string(&fields[6], "run")?
        )));
    }
    if let Ok(binding) = crate::node_runtime::parse_node_run_binding(value) {
        return Ok(Some(format!(
            "node control run binding run={} startup={}",
            binding.run_ref, binding.startup_receipt_ref
        )));
    }
    if let Ok(observation) = crate::node_runtime::parse_node_run_shutdown_observation(value) {
        return Ok(Some(format!(
            "node control run shutdown observation run={} startup={} shutdown={}",
            observation.run_ref, observation.startup_receipt_ref, observation.shutdown_receipt_ref
        )));
    }
    Ok(None)
}

fn completed_node_run_control(
    root: &crate::node_state::DirectoryRoot,
    observations: &NodeRunObservations,
) -> Result<Option<crate::node_runtime::ControlReceipt>> {
    let Some(completion) = &observations.shutdown_observation else {
        return Ok(None);
    };
    let ledger = root.ledger_store()?;
    let value = crate::ledger::read_artifact_with_root(&ledger, &completion.control_receipt_ref)?;
    crate::node_runtime::parse_control_receipt(&value).map(Some)
}

fn write_control_loop_heartbeat(
    root: &crate::node_state::DirectoryRoot,
    startup_receipt_ref: &str,
    lock_ref: &str,
) -> Result<(String, IoValue)> {
    let value = heartbeat_receipt_value(&HeartbeatReceiptValueInput {
        startup_receipt_ref,
        lock_ref,
        loop_sequence: 0,
        processed_count: 0,
        diagnostics: &[],
    })?;
    let reference = crate::preserves_rail::canonical_hash(&value)?;
    write_preserves(root, &control_heartbeat_receipt_path(&reference)?, &value)?;
    import_artifact(root, &value)?;
    Ok((reference, value))
}

fn classify_control_dispatch_run(
    root: &crate::node_state::DirectoryRoot,
    binding: &crate::node_runtime::NodeRunBinding,
    observed: &ObservedControlDispatch,
) -> Result<crate::node_runtime::NodeRunLifecycle> {
    let dispatched = &observed.dispatch;
    let control = crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value)?;
    if dispatched.operation != "shutdown" || control.decision != "pass" {
        return Ok(crate::node_runtime::NodeRunLifecycle::Active);
    }
    let observations = read_node_run_observations(root, &binding.startup_receipt_ref)?;
    validate_member_ref(&observations.binding.run_ref, &binding.run_ref, "control loop current run")?;
    Ok(observations.classify(&dispatched.operation, Some(&control), observed.is_duplicate).lifecycle)
}
