
    struct TestRemoteClearanceInput<'a> {
        root: &'a std::path::Path,
        label: &'a str,
        requester_ref: &'a str,
        peer_ref: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
        remote_ref: &'a str,
        policy_ref: &'a str,
        authority_ref: &'a str,
        is_current: bool,
        revoked_refs: &'a [String],
        retained_refs: &'a [String],
    }

    struct TestAdmissionInput<'a> {
        root: &'a std::path::Path,
        kind: &'a str,
        label: &'a str,
        requester_ref: &'a str,
        object_ref: &'a str,
        object_kind: &'a str,
        retention_class: &'a str,
        action: &'a str,
        remote_refs: &'a [String],
        is_reference_index_complete: bool,
        is_current: bool,
        revoked_refs: &'a [String],
    }

    fn sensitive_explain_value(object_ref: &str, plan_ref: &str) -> IoValue {
        let plan_refs = vec![plan_ref.to_string()];
        candidate_explain_value(&CandidateExplainValueInput {
            object_ref,
            object_kind: Some("encrypted-ref"),
            retention_class: Some(CLASS_PRIVATE_SECRET_REF),
            action: Some(ACTION_DELETE),
            subsystem: Some("ledger-gc"),
            pin_refs: &[],
            admission_refs: &[],
            remote_clearance_refs: &[],
            remote_clearance_import_refs: &[],
            gc_plan_refs: &plan_refs,
            gc_apply_refs: &[],
            gc_execution_refs: &[],
            gc_audit_refs: &[],
            retention_receipt_refs: &[],
            tombstone_refs: &[],
            diagnostics: &[],
        })
        .expect("sensitive explain value")
    }

    fn fake_live_refs(label: &str) -> Vec<String> {
        (0..8).map(|index| fake_ref(&format!("{label}-live-ref-{index}"))).collect()
    }

    fn fake_live_transport_receipt(operation: &str, node_id: &str, envelope_label: &str, ingress_ref: &str) -> IoValue {
        crate::preserves_rail::record("node-control-live-transport-receipt-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TRANSPORT_RECEIPT_SCHEMA),
            crate::preserves_rail::record("operation", vec![crate::preserves_rail::string(operation)]),
            crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
            crate::preserves_rail::record("transport", vec![crate::preserves_rail::string("iroh-gossip")]),
            crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(
                molten_node_runtime::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
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
