
    #[test]
    fn control_ingress_enqueues_once_and_preserves_provenance_gate() {
        let root = temp_dir("node-control-ingress");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];

        let payload_value =
            crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
                "missing-provenance",
            )]);
        let payload_ref = import_artifact(&root, &payload_value).expect("import payload");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("install request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:operator",
            to_node: "node:ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
        assert_enqueued_then_denied(&root, &envelope);
    }
