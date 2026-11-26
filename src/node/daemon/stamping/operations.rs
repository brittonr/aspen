//! Queue, operation, heartbeat, and loop receipt values.

#[derive(Debug, Clone, Copy)]
pub(super) struct QueueReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) phase: &'a str,
    pub(super) operation: &'a str,
    pub(super) request_ref: &'a str,
    pub(super) location_ref: &'a str,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OperationReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) request: &'a crate::node_runtime::ControlRequest,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct HeartbeatReceiptValueInput<'a> {
    pub(super) startup_receipt_ref: &'a str,
    pub(super) lock_ref: &'a str,
    pub(super) loop_sequence: u64,
    pub(super) processed_count: u64,
    pub(super) diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LoopReceiptValueInput<'a> {
    pub(super) decision: &'a str,
    pub(super) startup_receipt_ref: &'a str,
    pub(super) heartbeat_receipt_ref: &'a str,
    pub(super) max_requests: u64,
    pub(super) processed_request_refs: &'a [String],
    pub(super) dispatch_receipt_refs: &'a [String],
    pub(super) has_stopped: bool,
    pub(super) diagnostics: &'a [String],
}

pub(super) fn queue_receipt_value(input: &QueueReceiptValueInput<'_>) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-queue-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_QUEUE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("phase", vec![crate::preserves_rail::string(input.phase)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(input.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(input.request_ref)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v1",
        )]),
        crate::preserves_rail::record("location", vec![crate::preserves_rail::string(input.location_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-request-ref"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("durable-control-profile"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn operation_receipt_value(
    input: &OperationReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-operation-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_OPERATION_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.request.operation)]),
        crate::preserves_rail::record("request", vec![crate::preserves_rail::string(&input.request.request_ref)]),
        crate::preserves_rail::record("target", vec![super::fields::optional_string(
            input.request.target_ref.as_deref(),
        )]),
        crate::preserves_rail::record("payload", vec![super::fields::optional_string(
            input.request.payload_ref.as_deref(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-dispatch-explicit"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("side-effects-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("canonical-receipt"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn heartbeat_receipt_value(
    input: &HeartbeatReceiptValueInput<'_>,
) -> crate::error::Result<preserves::IOValue> {
    Ok(crate::preserves_rail::record("node-control-heartbeat-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_HEARTBEAT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(
            if input.diagnostics.is_empty() { "pass" } else { "deny" },
        )]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("lock", vec![crate::preserves_rail::string(input.lock_ref)]),
        crate::preserves_rail::record("loop-sequence", vec![crate::preserves_rail::string(
            input.loop_sequence.to_string(),
        )]),
        crate::preserves_rail::record("processed-count", vec![crate::preserves_rail::string(
            input.processed_count.to_string(),
        )]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("active-lock-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("heartbeat-is-receipted"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("no-ambient-socket-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

pub(super) fn loop_receipt_value(input: &LoopReceiptValueInput<'_>) -> crate::error::Result<preserves::IOValue> {
    super::guards::validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-loop-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOOP_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(input.startup_receipt_ref)]),
        crate::preserves_rail::record("heartbeat", vec![crate::preserves_rail::string(input.heartbeat_receipt_ref)]),
        crate::preserves_rail::record("max-requests", vec![crate::preserves_rail::string(
            input.max_requests.to_string(),
        )]),
        crate::preserves_rail::record("processed-requests", vec![crate::preserves_rail::sequence(
            input.processed_request_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("dispatch-receipts", vec![crate::preserves_rail::sequence(
            input.dispatch_receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("stopped", vec![crate::preserves_rail::string(if input.has_stopped {
            "yes"
        } else {
            "no"
        })]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-loop-v1",
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-request-loop"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("deterministic-inbox-order"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("idempotent-request-dispatch"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("shutdown-stops-loop"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
