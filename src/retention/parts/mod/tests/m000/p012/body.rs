
    fn fake_live_transport_receipt(operation: &str, node_id: &str, envelope_label: &str, ingress_ref: &str) -> IoValue {
        crate::preserves_rail::record("node-control-live-transport-receipt-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
            crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(operation)]),
            crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
            crate::preserves_rail::record("transport", vec![crate::preserves_rail::string("iroh-gossip")]),
            crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(
                crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
            )]),
            crate::preserves_rail::record("node", vec![crate::preserves_rail::string(node_id)]),
            crate::preserves_rail::record("delivered-from", vec![optional_ref_value(Some(&fake_ref(&format!(
                "{envelope_label}-peer"
            ))))]),
            crate::preserves_rail::record("envelope", vec![crate::preserves_rail::string(fake_ref(envelope_label))]),
            crate::preserves_rail::record("ingress-receipt", vec![optional_ref_value(Some(ingress_ref))]),
            crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(Vec::new())]),
            checks_value(&[
                ("canonical-envelope-ref", "pass"),
                ("live-iroh-gossip", "pass"),
                ("peer-bootstrap-before-enqueue", "pass"),
                ("transport-is-not-authority", "pass"),
                ("durable-inbox-boundary", "pass"),
            ]),
        ])
    }
