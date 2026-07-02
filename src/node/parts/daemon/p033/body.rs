
fn access_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(ticket) = parse_control_live_ticket(value) {
        return Ok(Some(format!(
            "node control live ticket ref={} node={} topic={} endpoint={}",
            ticket.ticket_ref, ticket.node_id, ticket.topic, ticket.live_endpoint_id
        )));
    }
    if let Ok(admission) = parse_control_live_peer_admission(value) {
        return Ok(Some(format!(
            "node control live peer admission decision={} peer={} node={} topic={}",
            admission.decision, admission.peer_id, admission.node_id, admission.topic
        )));
    }
    if let Ok(grant) = parse_control_authority_grant(value) {
        return Ok(Some(format!(
            "node control authority grant ref={} peer={} node={} operations={}",
            grant.grant_ref,
            grant.peer_id,
            grant.node_id,
            grant.operations.join(",")
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-receipt-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA,
            "node control authority receipt",
        )?;
        return Ok(Some(format!(
            "node control authority decision={} envelope={} operation={} grant={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "envelope")?,
            record_string(&fields[6], "operation")?,
            record_optional_string(&fields[7], "grant")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn flow_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
            "node control live listener receipt",
        )?;
        return Ok(Some(format!(
            "node control live listener decision={} topic={} events={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[6], "topic")?,
            record_string(&fields[8], "observed-events")?,
            record_string(&fields[11], "service-run")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA,
            "node control live workflow receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow decision={} peer={} node={} send={} service={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "peer")?,
            record_string(&fields[4], "node")?,
            record_string(&fields[8], "send-receipt")?,
            record_string(&fields[11], "service-run")?
        )));
    }
    Ok(None)
}

fn bundle_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
            "node control live workflow bundle",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle ticket={} admission={} grant={} receipts={}",
            record_string(&fields[5], "ticket-ref")?,
            record_string(&fields[6], "peer-admission-ref")?,
            record_string(&fields[7], "authority-grant-ref")?,
            record_sequence_len(&fields[8], "receipt-refs")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-export-receipt-v1", Some(9)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_EXPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle export receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle export decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-import-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_IMPORT_RECEIPT_SCHEMA,
            "node control live workflow bundle import receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle import decision={} bundle={} imported={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_sequence_len(&fields[9], "imported")?
        )));
    }
    Ok(None)
}

fn gate_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-verify-receipt-v1", Some(10)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_VERIFY_RECEIPT_SCHEMA,
            "node control live workflow bundle verify receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle verify decision={} bundle={} receipts={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_sequence_len(&fields[6], "receipts")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-gate-receipt-v1", Some(12)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_GATE_RECEIPT_SCHEMA,
            "node control live workflow bundle gate receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle gate decision={} bundle={} verify={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "bundle")?,
            record_optional_string(&fields[3], "verify-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn apply_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-bundle-apply-receipt-v1", Some(15)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_APPLY_RECEIPT_SCHEMA,
            "node control live workflow bundle apply receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle apply decision={} bundle={} mode={} send={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_string(&fields[8], "mode")?,
            record_optional_string(&fields[11], "send-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) =
        value.collect_simple_record("node-control-live-workflow-bundle-reconcile-receipt-v1", Some(13))
    {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_RECONCILE_RECEIPT_SCHEMA,
            "node control live workflow bundle reconcile receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow bundle reconcile decision={} bundle={} envelope={} control={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[3], "bundle")?,
            record_optional_string(&fields[8], "envelope")?.unwrap_or_else(|| "none".to_string()),
            record_optional_string(&fields[7], "control-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn send_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-live-send-retry-receipt-v1", Some(14)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA,
            "node control live send retry receipt",
        )?;
        return Ok(Some(format!(
            "node control live send retry decision={} attempt={}/{} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "attempt")?,
            record_string(&fields[3], "max-attempts")?,
            record_string(&fields[10], "envelope")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-duplicate-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_DUPLICATE_RECEIPT_SCHEMA,
            "node control live send duplicate receipt",
        )?;
        return Ok(Some(format!(
            "node control live send duplicate operation={} prior={}",
            record_string(&fields[9], "operation")?,
            record_string(&fields[10], "prior-send-receipt")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
            "node control live send receipt",
        )?;
        return Ok(Some(format!(
            "node control live send decision={} from={} to={} ticket={} envelope={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[4], "from-peer")?,
            record_string(&fields[5], "to-node")?,
            record_string(&fields[6], "receiver-ticket")?,
            record_string(&fields[9], "envelope")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-transport-receipt-v1", Some(11)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA,
            "node control live transport receipt",
        )?;
        return Ok(Some(format!(
            "node control live transport operation={} decision={} envelope={} ingress={}",
            record_string(&fields[1], "operation")?,
            record_string(&fields[2], "decision")?,
            record_string(&fields[7], "envelope")?,
            record_optional_string(&fields[8], "ingress-receipt")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn state_summary(value: &IoValue) -> Result<Option<String>> {
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
            record_string(&fields[2], "startup")?,
            record_string(&fields[3], "owner")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-lock-v1", Some(10)) {
        return Ok(Some(format!(
            "node control service lock startup={} topic={} max_ticks={}",
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "topic")?,
            record_string(&fields[5], "max-ticks")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control service heartbeat decision={} startup={} tick={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[4], "tick")?
        )));
    }
    Ok(None)
}
