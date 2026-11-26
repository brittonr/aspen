//! Receipt summaries: dispatch by schema, runtime, import, access, and flow.

pub fn summary(value: &preserves::IOValue) -> crate::error::Result<String> {
    if let Some(summary) = runtime_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = import_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = access_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = flow_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::narration::bundle_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::narration::gate_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::narration::apply_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::narration::send_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::narration::state_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::phrasing::supervisor_summary(value)? {
        return Ok(summary);
    }
    if let Some(summary) = super::phrasing::control_summary(value)? {
        return Ok(summary);
    }
    if let Ok(summary) = crate::protocol_session::protocol_summary(value) {
        return Ok(summary);
    }
    if let Ok(summary) = crate::provenance::summary(value) {
        return Ok(summary);
    }
    Err(crate::error::Failure::invalid_harness("unsupported node daemon artifact for show"))
}

fn runtime_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Ok(config) = crate::node_runtime::parse_node_config(value) {
        return Ok(Some(format!(
            "node config ref={} identity={} adapters={}",
            config.config_ref,
            config.identity_ref,
            config.adapters.len()
        )));
    }
    if let Ok(startup) = crate::node_runtime::parse_node_startup_receipt(value) {
        return Ok(Some(format!(
            "node startup decision={} receipt={} adapters={}",
            startup.decision,
            startup.receipt_ref,
            startup.adapters.len()
        )));
    }
    if let Ok(control) = crate::node_runtime::parse_control_receipt(value) {
        return Ok(Some(format!(
            "node control decision={} receipt={} request={}",
            control.decision, control.receipt_ref, control.request_ref
        )));
    }
    if let Ok(ingress) = super::delivery::parse_control_ingress_envelope(value) {
        return Ok(Some(format!(
            "node control ingress envelope ref={} topic={} from={} to={} request={}",
            ingress.envelope_ref, ingress.topic, ingress.from_peer, ingress.to_node, ingress.request.request_ref
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-ingress-receipt-v1", Some(15)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
            "node control ingress receipt",
        )?;
        return Ok(Some(format!(
            "node control ingress decision={} phase={} envelope={} request={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "phase")?,
            super::fields::record_string(&fields[8], "envelope")?,
            super::fields::record_string(&fields[10], "request")?
        )));
    }
    Ok(None)
}

fn import_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value
        .collect_simple_record("node-control-live-ticket-import-receipt-v1", Some(15))
        .or_else(|| value.collect_simple_record("node-control-live-ticket-import-receipt-v1", Some(13)))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA,
            "node control live ticket import receipt",
        )?;
        return Ok(Some(format!(
            "node control live ticket import decision={} ticket={} imported={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "ticket")?,
            super::fields::record_sequence_len(&fields[10], "imported")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-grant-import-receipt-v1", Some(13)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_GRANT_IMPORT_RECEIPT_SCHEMA,
            "node control authority grant import receipt",
        )?;
        return Ok(Some(format!(
            "node control authority grant import decision={} grant={} imported={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "grant")?,
            super::fields::record_sequence_len(&fields[10], "imported")?
        )));
    }
    Ok(None)
}

fn access_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Ok(ticket) = super::issuance::parse_control_live_ticket(value) {
        return Ok(Some(format!(
            "node control live ticket ref={} node={} topic={} endpoint={}",
            ticket.ticket_ref, ticket.node_id, ticket.topic, ticket.live_endpoint_id
        )));
    }
    if let Ok(admission) = super::peering::parse_control_live_peer_admission(value) {
        return Ok(Some(format!(
            "node control live peer admission decision={} peer={} node={} topic={}",
            admission.decision, admission.peer_id, admission.node_id, admission.topic
        )));
    }
    if let Ok(grant) = super::issuance::parse_control_authority_grant(value) {
        return Ok(Some(format!(
            "node control authority grant ref={} peer={} node={} operations={}",
            grant.grant_ref,
            grant.peer_id,
            grant.node_id,
            grant.operations.join(",")
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-authority-receipt-v1", Some(10)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_AUTHORITY_RECEIPT_SCHEMA,
            "node control authority receipt",
        )?;
        return Ok(Some(format!(
            "node control authority decision={} envelope={} operation={} grant={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[2], "envelope")?,
            super::fields::record_string(&fields[6], "operation")?,
            super::fields::record_optional_string(&fields[7], "grant")?.unwrap_or_else(|| "none".to_string())
        )));
    }
    Ok(None)
}

fn flow_summary(value: &preserves::IOValue) -> crate::error::Result<Option<String>> {
    if let Some(fields) = value
        .collect_simple_record("node-control-live-listener-receipt-v1", Some(16))
        .or_else(|| value.collect_simple_record("node-control-live-listener-receipt-v1", Some(14)))
    {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_LISTENER_RECEIPT_SCHEMA,
            "node control live listener receipt",
        )?;
        return Ok(Some(format!(
            "node control live listener decision={} topic={} events={} service={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[6], "topic")?,
            super::fields::record_string(&fields[8], "observed-events")?,
            super::fields::record_string(&fields[11], "service-run")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-live-workflow-receipt-v1", Some(14)) {
        super::guards::require_schema(
            &fields[0],
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_RECEIPT_SCHEMA,
            "node control live workflow receipt",
        )?;
        return Ok(Some(format!(
            "node control live workflow decision={} peer={} node={} send={} service={}",
            super::fields::record_string(&fields[1], "decision")?,
            super::fields::record_string(&fields[3], "peer")?,
            super::fields::record_string(&fields[4], "node")?,
            super::fields::record_string(&fields[8], "send-receipt")?,
            super::fields::record_string(&fields[11], "service-run")?
        )));
    }
    Ok(None)
}
