//! Receipt values for service locks, heartbeats, supervisors, and runs.

#[derive(Debug, Clone, Copy)]
pub(super) struct SupervisorReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) operation: &'a str,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: Option<&'a str>,
    pub(super) supervisor_policy_ref: Option<&'a str>,
    pub(super) topic: &'a str,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ServiceLockValueInput<'a> {
    pub(super) startup_receipt_ref: &'a str,
    pub(super) node_id: &'a str,
    pub(super) topic: &'a str,
    pub(super) max_ticks: u64,
    pub(super) max_requests_per_tick: u64,
    pub(super) service_run_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ServiceHeartbeatValueInput<'a> {
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: &'a str,
    pub(super) tick: u64,
    pub(super) delivered_count: u64,
    pub(super) processed_count: u64,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ServiceRunReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) service_lock_ref: Option<&'a str>,
    pub(super) topic: &'a str,
    pub(super) max_ticks: u64,
    pub(super) max_requests_per_tick: u64,
    pub(super) ticks: u64,
    pub(super) heartbeat_receipt_refs: &'a [String],
    pub(super) ingress_receipt_refs: &'a [String],
    pub(super) loop_receipt_refs: &'a [String],
    pub(super) processed_request_refs: &'a [String],
    pub(super) has_stopped: bool,
    pub(super) supervisor_policy_ref: Option<&'a str>,
    pub(super) supervisor_receipt_refs: &'a [String],
    pub(super) diagnostics: &'a [String],
}

pub(super) fn service_lock_value(input: &ServiceLockValueInput<'_>) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-service-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(
            &super::storage::state_root_profile_ref(&())?,
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(input.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("service-run", vec![crate::preserves_rail::string(input.service_run_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-supervised-node-control-v1",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-ticks"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn service_heartbeat_receipt_value(
    input: &ServiceHeartbeatValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-service-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![crate::preserves_rail::string(input.service_lock_ref)]),
        crate::preserves_rail::record("tick", vec![crate::preserves_rail::string(input.tick.to_string())]),
        crate::preserves_rail::record("delivered-count", vec![crate::preserves_rail::string(
            input.delivered_count.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("service-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("monotonic-tick"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn supervisor_receipt_value(
    input: &SupervisorReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-supervisor-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SUPERVISOR_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![super::fields::optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("policy", vec![super::fields::optional_string(input.supervisor_policy_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("supervisor-policy-bound"),
                crate::preserves_rail::string(if input.supervisor_policy_ref.is_some() {
                    "pass"
                } else {
                    "fail"
                }),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("single-active-service"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-restart-policy"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-drain-bound"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn service_run_check_sequence(input: &ServiceRunReceiptValueInput<'_>) -> preserves::IOValue {
    let has_supervisor_policy_binding =
        input.supervisor_policy_ref.is_none() || !input.supervisor_receipt_refs.is_empty();
    crate::preserves_rail::sequence(vec![
        super::stamps::receipt_check_value(
            "single-active-service",
            super::stamps::pass_if(input.service_lock_ref.is_some()),
        ),
        super::stamps::receipt_check_value("ingress-before-loop", "pass"),
        super::stamps::receipt_check_value("loop-reuse", "pass"),
        super::stamps::receipt_check_value("shutdown-stop-semantics", "pass"),
        super::stamps::receipt_check_value("bounded-ticks", "pass"),
        super::stamps::receipt_check_value(
            "supervisor-policy-bound",
            super::stamps::pass_if(has_supervisor_policy_binding),
        ),
    ])
}

pub(super) fn service_run_receipt_value(
    input: &ServiceRunReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-service-run-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_SERVICE_RUN_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("service-lock", vec![super::fields::optional_string(input.service_lock_ref)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(input.topic)]),
        crate::preserves_rail::record("max-ticks", vec![crate::preserves_rail::string(input.max_ticks.to_string())]),
        crate::preserves_rail::record("max-requests-per-tick", vec![crate::preserves_rail::string(
            input.max_requests_per_tick.to_string(),
        )]),
        crate::preserves_rail::record("ticks", vec![crate::preserves_rail::string(input.ticks.to_string())]),
        crate::preserves_rail::record("heartbeats", vec![crate::preserves_rail::sequence(
            input.heartbeat_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("ingress-receipts", vec![crate::preserves_rail::sequence(
            input.ingress_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("loop-receipts", vec![crate::preserves_rail::sequence(
            input.loop_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "true"
        } else {
            "false"
        })]),
        crate::preserves_rail::record("supervisor-policy", vec![super::fields::optional_string(
            input.supervisor_policy_ref,
        )]),
        crate::preserves_rail::record("supervisor-receipts", vec![crate::preserves_rail::sequence(
            input.supervisor_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![service_run_check_sequence(input)]),
    ]))
}
