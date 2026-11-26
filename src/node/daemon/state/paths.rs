//! Logical state-root paths for control receipts and envelopes.

pub(super) fn node_leaf_path(base: &str, leaf: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    super::storage::fixed_node_path(base)?.join_segment(leaf)
}

pub(super) fn control_inbox_entry_name(request_ref: &str) -> String {
    format!("{}.preserves", ref_file_stem(request_ref))
}

pub(super) fn control_inbox_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(super::settings::CONTROL_INBOX_DIR, &control_inbox_entry_name(request_ref))
}

pub(super) fn queue_receipt_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_INBOX_DIR,
        &format!("{}.queue-receipt.preserves", ref_file_stem(request_ref)),
    )
}

pub(super) fn dispatch_receipt_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_OUTBOX_DIR,
        &format!("{}.dispatch-receipt.preserves", ref_file_stem(request_ref)),
    )
}

pub(super) fn control_outbox_request_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(super::settings::CONTROL_OUTBOX_DIR, &format!("{}.request.preserves", ref_file_stem(request_ref)))
}

pub(super) fn control_outbox_receipt_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_OUTBOX_DIR,
        &format!("{}.control-receipt.preserves", ref_file_stem(request_ref)),
    )
}

pub(super) fn control_operation_receipt_path(
    request_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_OUTBOX_DIR,
        &format!("{}.operation-receipt.preserves", ref_file_stem(request_ref)),
    )
}

pub(super) fn control_operation_subreceipt_path(
    request_ref: &str,
    label: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(super::settings::CONTROL_OUTBOX_DIR, &format!("{}.{}.preserves", ref_file_stem(request_ref), label))
}

pub(super) fn control_heartbeat_receipt_path(
    heartbeat_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_OUTBOX_DIR,
        &format!("{}.heartbeat-receipt.preserves", ref_file_stem(heartbeat_ref)),
    )
}

pub(super) fn control_loop_receipt_path(loop_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(super::settings::CONTROL_OUTBOX_DIR, &format!("{}.loop-receipt.preserves", ref_file_stem(loop_ref)))
}

pub(super) fn control_service_heartbeat_path(
    heartbeat_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_SERVICE_DIR,
        &format!("{}.service-heartbeat.preserves", ref_file_stem(heartbeat_ref)),
    )
}

pub(super) fn control_service_run_receipt_path(
    service_run_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_SERVICE_DIR,
        &format!("{}.service-run-receipt.preserves", ref_file_stem(service_run_ref)),
    )
}

pub(super) fn control_supervisor_receipt_path(
    receipt_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_SERVICE_DIR,
        &format!("{}.supervisor-receipt.preserves", ref_file_stem(receipt_ref)),
    )
}

pub(super) fn control_ingress_envelope_path(
    topic: &str,
    envelope_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    super::storage::fixed_node_path(super::settings::CONTROL_INGRESS_DIR)?
        .join_segment(topic)?
        .join_segment(&format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
}

pub(super) fn control_ingress_receipt_path(
    envelope_ref: &str,
    phase: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    super::storage::fixed_node_path(super::settings::CONTROL_INGRESS_DIR)?
        .join("receipts")?
        .join_segment(&format!("{}.{}.receipt.preserves", ref_file_stem(envelope_ref), phase))
}

pub(super) fn control_live_transport_receipt_path(
    envelope_ref: &str,
    operation: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.live-{}.receipt.preserves", ref_file_stem(envelope_ref), operation))
}

pub(super) fn control_live_send_receipt_path(send_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.live-send.receipt.preserves", ref_file_stem(send_ref)))
}

pub(super) fn control_live_send_retry_receipt_path(
    retry_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.live-send-retry.receipt.preserves", ref_file_stem(retry_ref)))
}

pub(super) fn control_live_send_duplicate_receipt_path(
    duplicate_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.live-send-duplicate.receipt.preserves", ref_file_stem(duplicate_ref)))
}

pub(super) fn control_live_workflow_receipt_path(
    workflow_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.live-workflow.receipt.preserves", ref_file_stem(workflow_ref)))
}

pub(super) fn control_live_listener_receipt_path(
    listener_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    node_leaf_path(
        super::settings::CONTROL_SERVICE_DIR,
        &format!("{}.live-listener-receipt.preserves", ref_file_stem(listener_ref)),
    )
}

pub(super) fn control_authority_receipt_path(
    envelope_ref: &str,
) -> crate::error::Result<crate::node_state::RelativePath> {
    ingress_receipt_leaf(&format!("{}.authority-receipt.preserves", ref_file_stem(envelope_ref)))
}

fn ingress_receipt_leaf(leaf: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    super::storage::fixed_node_path(super::settings::CONTROL_INGRESS_DIR)?
        .join("receipts")?
        .join_segment(leaf)
}

pub(super) fn ref_file_stem(value_ref: &str) -> String {
    value_ref.replace(':', "-")
}
