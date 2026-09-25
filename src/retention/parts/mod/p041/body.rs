
fn node_live_send_diagnostics(phase: &str, send: &crate::node_daemon::ControlLiveSendReceipt) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(send.diagnostics.len().saturating_add(2));
    for diagnostic in &send.diagnostics {
        diagnostics.push(format!("remote-clearance-live-{phase}:{diagnostic}"));
    }
    if send.decision != "pass" {
        diagnostics.push(format!("remote-clearance-live-{phase}-send-deny:{}", send.decision));
    }
    if send.transport_receipt_ref.is_none() {
        diagnostics.push(format!("remote-clearance-live-{phase}-missing-transport-receipt"));
    }
    diagnostics
}

fn node_live_transport_diagnostics(phase: &str, value: &IoValue) -> Result<Vec<String>> {
    let receipt = parse_node_live_transport_receipt(value)?;
    node_live_transport_diagnostics_from(phase, &receipt)
}

fn node_live_transport_diagnostics_from(phase: &str, receipt: &NodeLiveTransportReceipt) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for diagnostic in &receipt.diagnostics {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}:{diagnostic}"),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    if receipt.decision != "pass" {
        push_bounded(
            &mut diagnostics,
            format!("remote-clearance-live-{phase}-transport-deny:{}:{}", receipt.operation, receipt.decision),
            MAX_RETENTION_DIAGNOSTICS,
            "retention live transport diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn node_live_receive_binding_diagnostics(
    phase: &str,
    send: &crate::node_daemon::ControlLiveSendReceipt,
    receive: &NodeLiveTransportReceipt,
    expected_ingress_ref: &str,
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if receive.operation != "receive" {
        diagnostics.push(format!("remote-clearance-live-{phase}-not-receive:{}", receive.operation));
    }
    if receive.envelope_ref != send.envelope_ref {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-envelope"));
    }
    if receive.node_id != send.to_node {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-node"));
    }
    if receive.ingress_receipt_ref.as_deref() != Some(expected_ingress_ref) {
        diagnostics.push(format!("remote-clearance-live-{phase}-wrong-ingress"));
    }
    diagnostics
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}
