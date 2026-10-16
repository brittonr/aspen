#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRunBinding {
    pub run_ref: String,
    pub state_root_ref: String,
    pub startup_receipt_ref: String,
    pub previous_run_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeRunBindingValueInput<'a> {
    pub state_root_ref: &'a str,
    pub startup_receipt_ref: &'a str,
    pub previous_run_ref: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRunShutdownObservation {
    pub observation_ref: String,
    pub run_ref: String,
    pub startup_receipt_ref: String,
    pub shutdown_receipt_ref: String,
    pub control_receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeRunShutdownObservationValueInput<'a> {
    pub run_ref: &'a str,
    pub startup_receipt_ref: &'a str,
    pub shutdown_receipt_ref: &'a str,
    pub control_receipt_ref: &'a str,
}

// r[impl molten.audit_f14.current_binding]
pub fn node_run_binding_value(input: &NodeRunBindingValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.state_root_ref, "node run state root ref")?;
    validate_ref(input.startup_receipt_ref, "node run startup receipt ref")?;
    if let Some(previous_run_ref) = input.previous_run_ref {
        validate_ref(previous_run_ref, "node run previous run ref")?;
    }
    Ok(record("node-control-run-v1", vec![
        string(crate::preserves_rail::NODE_CONTROL_RUN_SCHEMA),
        record("state-root", vec![string(input.state_root_ref)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("previous-run", vec![optional_ref_value(input.previous_run_ref)]),
    ]))
}

// r[impl molten.audit_f14.current_binding]
// r[impl molten.audit_f14.preserve_state]
pub fn parse_node_run_binding(value: &IoValue) -> Result<NodeRunBinding> {
    let fields = simple_record(value, "node-control-run-v1", 4)?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_RUN_SCHEMA, "node run binding")?;
    Ok(NodeRunBinding {
        run_ref: canonical_hash(value)?,
        state_root_ref: record_ref(&fields[1], "state-root")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        previous_run_ref: record_optional_ref(&fields[3], "previous-run")?,
        value: value.clone(),
    })
}

// r[impl molten.audit_f14.current_binding]
pub fn node_run_shutdown_observation_value(input: &NodeRunShutdownObservationValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.run_ref, "node run shutdown run ref")?;
    validate_ref(input.startup_receipt_ref, "node run shutdown startup receipt ref")?;
    validate_ref(input.shutdown_receipt_ref, "node run shutdown shutdown receipt ref")?;
    validate_ref(input.control_receipt_ref, "node run shutdown control receipt ref")?;
    Ok(record("node-control-run-shutdown-v1", vec![
        string(crate::preserves_rail::NODE_CONTROL_RUN_SHUTDOWN_SCHEMA),
        record("run", vec![string(input.run_ref)]),
        record("startup", vec![string(input.startup_receipt_ref)]),
        record("shutdown", vec![string(input.shutdown_receipt_ref)]),
        record("control", vec![string(input.control_receipt_ref)]),
    ]))
}

// r[impl molten.audit_f14.current_binding]
// r[impl molten.audit_f14.preserve_state]
pub fn parse_node_run_shutdown_observation(value: &IoValue) -> Result<NodeRunShutdownObservation> {
    let fields = simple_record(value, "node-control-run-shutdown-v1", 5)?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_RUN_SHUTDOWN_SCHEMA,
        "node run shutdown observation",
    )?;
    Ok(NodeRunShutdownObservation {
        observation_ref: canonical_hash(value)?,
        run_ref: record_ref(&fields[1], "run")?,
        startup_receipt_ref: record_ref(&fields[2], "startup")?,
        shutdown_receipt_ref: record_ref(&fields[3], "shutdown")?,
        control_receipt_ref: record_ref(&fields[4], "control")?,
        value: value.clone(),
    })
}
