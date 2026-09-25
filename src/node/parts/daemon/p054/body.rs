
fn live_send_retry_receipt_value(input: &LiveSendRetryReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    Ok(crate::preserves_rail::record("node-control-live-send-retry-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RETRY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("attempt", vec![crate::preserves_rail::string(input.attempt.to_string())]),
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(
            input.max_attempts.to_string(),
        )]),
        crate::preserves_rail::record("transport", vec![crate::preserves_rail::string(LIVE_CONTROL_INGRESS_TRANSPORT)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.envelope.topic)]),
        crate::preserves_rail::record("from-peer", vec![crate::preserves_rail::string(input.from_peer)]),
        crate::preserves_rail::record("to-node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("receiver-ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("receiver-endpoint", vec![crate::preserves_rail::string(
            &input.ticket.live_endpoint_id,
        )]),
        crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(&input.envelope.envelope_ref)]),
        crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(&input.envelope.operation_ref)]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("bounded-retry"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("operation-id-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("transport-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}
