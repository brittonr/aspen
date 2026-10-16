#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRunLifecycle {
    Active,
    Stopped,
    Unresolved,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeRunLifecycleInput<'a> {
    pub operation: &'a str,
    pub is_duplicate: bool,
    pub current_run_ref: &'a str,
    pub current_startup_receipt_ref: &'a str,
    pub active_run_ref: Option<&'a str>,
    pub shutdown_observation: Option<&'a NodeRunShutdownObservation>,
    pub shutdown_receipt: Option<&'a NodeShutdownReceipt>,
    pub control_receipt: Option<&'a ControlReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRunLifecycleOutcome {
    pub is_duplicate: bool,
    pub lifecycle: NodeRunLifecycle,
}

// r[impl molten.audit_f14.duplicate_scope]
// r[impl molten.audit_f14.current_binding]
// r[impl molten.audit_f14.preserve_state]
// r[impl molten.audit_f14.validation]
pub fn classify_node_run_lifecycle(input: &NodeRunLifecycleInput<'_>) -> NodeRunLifecycleOutcome {
    NodeRunLifecycleOutcome {
        is_duplicate: input.is_duplicate,
        lifecycle: node_run_lifecycle(input),
    }
}

fn node_run_lifecycle(input: &NodeRunLifecycleInput<'_>) -> NodeRunLifecycle {
    if !lifecycle_ref_is_canonical(input.current_run_ref)
        || !lifecycle_ref_is_canonical(input.current_startup_receipt_ref)
        || input.control_receipt.is_some_and(|control| !is_passing_lifecycle_control(control))
    {
        return NodeRunLifecycle::Unresolved;
    }
    if let Some(active_run_ref) = input.active_run_ref {
        // Historical receipt success cannot stop the currently locked run.
        return if active_run_ref == input.current_run_ref
            && input.shutdown_observation.is_none()
            && input.shutdown_receipt.is_none()
        {
            NodeRunLifecycle::Active
        } else {
            NodeRunLifecycle::Unresolved
        };
    }
    let (Some(control), Some(observation), Some(shutdown)) =
        (input.control_receipt, input.shutdown_observation, input.shutdown_receipt)
    else {
        return NodeRunLifecycle::Unresolved;
    };
    if input.operation != "shutdown"
        || shutdown.decision != "pass"
        || !lifecycle_ref_is_canonical(&observation.observation_ref)
        || !lifecycle_ref_is_canonical(&shutdown.receipt_ref)
        || observation.run_ref != input.current_run_ref
        || observation.startup_receipt_ref != input.current_startup_receipt_ref
        || control.startup_receipt_ref != input.current_startup_receipt_ref
        || shutdown.startup_receipt_ref != input.current_startup_receipt_ref
        || observation.control_receipt_ref != control.receipt_ref
        || observation.shutdown_receipt_ref != shutdown.receipt_ref
        || !control.subreceipt_refs.iter().any(|value| value == &shutdown.receipt_ref)
    {
        return NodeRunLifecycle::Unresolved;
    }
    NodeRunLifecycle::Stopped
}

fn is_passing_lifecycle_control(control: &ControlReceipt) -> bool {
    lifecycle_ref_is_canonical(&control.receipt_ref)
        && lifecycle_ref_is_canonical(&control.request_ref)
        && lifecycle_ref_is_canonical(&control.startup_receipt_ref)
        && control.subreceipt_refs.iter().all(|value| lifecycle_ref_is_canonical(value))
        && control.decision == "pass"
}

// The fallible rail validator formats errors. This predicate keeps even malformed
// lifecycle inputs allocation-free while enforcing the same canonical ref shape.
fn lifecycle_ref_is_canonical(value: &str) -> bool {
    value.strip_prefix("blake3:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
