//! Receipt summaries for bundles, gates, applies, sends, and state.

pub(super) fn bundle_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-v1", Some(10)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
            "node control live workflow bundle",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle ticket={} admission={} grant={} receipts={}",
            super::fields::record_string(&fields[5], "ticket-ref")?,
            super::fields::record_string(&fields[6], "peer-admission-ref")?,
            super::fields::record_string(&fields[7], "authority-grant-ref")?,
            super::fields::record_sequence_len(&fields[8], "receipt-refs")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-export-receipt-v1", Some(9)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle export receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle export decision={} bundle={} receipts={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "bundle")?,
            super::fields::record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-import-receipt-v1", Some(12)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle import receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle import decision={} bundle={} imported={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "bundle")?,
            super::fields::record_sequence_len(&fields[9], "imported")?
        )));
    }
    Ok(None)
}

pub(super) fn gate_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
            "node control live workflow bundle verify receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle verify decision={} bundle={} receipts={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "bundle")?,
            super::fields::record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
            "node control live workflow bundle gate receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle gate decision={} bundle={} verify={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "bundle")?,
            super::fields::record_optional_string(&fields[3], "verify-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

pub(super) fn apply_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value
        .collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(17))
        .or_else(|| value.collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15)))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
            "node control live workflow bundle apply receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle apply decision={} bundle={} mode={} send={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "bundle")?,
            super::fields::record_string(&fields[8], "mode")?,
            super::fields::record_optional_string(&fields[11], "send-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) =
        value.collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
            "node control live workflow bundle reconcile receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle reconcile decision={} bundle={} envelope={} control={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "bundle")?,
            super::fields::record_optional_string(&fields[8], "envelope")?.unwrap_or_else(|| "none".to_string()),
            super::fields::record_optional_string(&fields[7], "control-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

pub(super) fn send_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-send-retry-receipt-v1", Some(14)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA,
            "node control live send retry receipt",
        )?;
        return Ok(Some(format!(
            "node control live send retry decision={} attempt={}/{} envelope={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "attempt")?,
            super::fields::record_string(&fields[3], "max-attempts")?,
            super::fields::record_string(&fields[10], "envelope")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-duplicate-receipt-v1", Some(13)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA,
            "node control live send duplicate receipt",
        )?;
        return Ok(Some(format!(
            "node control live send duplicate operation={} prior={}",
            super::fields::record_string(&fields[9], "operation")?,
            super::fields::record_string(&fields[10], "prior-send-receipt")?
        )));
    }
    if let Some(fields) = value
        .collect_simple_record("node-control-live-send-receipt-v1", Some(15))
        .or_else(|| value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
            "node control live send receipt",
        )?;
        return Ok(Some(format!(
            "node control live send decision={} from={} to={} ticket={} envelope={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[4], "from-peer")?,
            super::fields::record_string(&fields[5], "to-node")?,
            super::fields::record_string(&fields[6], "receiver-ticket")?,
            super::fields::record_string(&fields[9], "envelope")?
        )));
    }
    if let Some(fields) = value
        .collect_simple_record("node-control-live-transport-receipt-v1", Some(13))
        .or_else(|| value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
            "node control live transport receipt",
        )?;
        return Ok(Some(format!(
            "node control live transport operation={} decision={} envelope={} ingress={}",
            super::fields::record_string(&fields[1], "operation")?,
            super::fields::record_string(&fields[2], "decision")?,
            super::fields::record_string(&fields[7], "envelope")?,
            super::fields::record_optional_string(&fields[8], "ingress-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

pub(super) fn state_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(summary) = super::phrasing::run_state_summary(value)? {
        return Ok(Some(summary));
    }
    if let Ok(health) = crate::node_runtime::parse_node_health_receipt(value) {
        return Ok(Some(format!(
            "node health decision={} receipt={} replay={}",
            health.decision, health.receipt_ref, health.replay_status
        )));
    }
    if let Ok(shutdown) = crate::node_runtime::parse_node_shutdown_receipt(value) {
        return Ok(Some(format!(
            "node shutdown decision={} receipt={} adapters={}",
            shutdown.decision,
            shutdown.receipt_ref,
            shutdown.adapters.len()
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-lock-v1", Some(6)) {
        return Ok(Some(format!(
            "node control lock startup={} owner={}",
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_string(&fields[3], "owner")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-lock-v1", Some(10)) {
        return Ok(Some(format!(
            "node control service lock startup={} topic={} max_ticks={}",
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_string(&fields[4], "topic")?,
            super::fields::record_string(&fields[5], "max-ticks")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control service heartbeat decision={} startup={} tick={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "startup")?,
            super::fields::record_string(&fields[4], "tick")?
        )));
    }
    Ok(None)
}
