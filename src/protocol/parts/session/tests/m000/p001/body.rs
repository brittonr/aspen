
    #[test]
    fn protocol_message_semantics_are_transport_neutral() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let server = lifecycle.initial_states[1].clone();
        let request = lifecycle.operations[0].message.as_ref().expect("request").clone();
        let local = receive_protocol_message(ProtocolReceiveInput {
            state: server.value.clone(),
            message: request.value.clone(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("local receive");
        let envelope = protocol_message_remote_envelope(ProtocolRemoteEnvelopeInput {
            from_peer: "peer:a".to_string(),
            from_actor: "client".to_string(),
            to_peer: "peer:b".to_string(),
            topic: "protocols".to_string(),
            message: request.value,
            capability_refs: vec![test_ref("capability")],
            evidence_refs: vec![test_ref("carrier-evidence")],
        })
        .expect("remote envelope");
        let remote = receive_protocol_message(ProtocolReceiveInput {
            state: server.value,
            message: envelope.payload,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: vec![envelope.envelope_ref],
        })
        .expect("remote receive");
        assert_eq!(
            local.next_state.expect("local next").local_state,
            remote.next_state.expect("remote next").local_state
        );
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_protocol_records() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let gate = gate_protocol_session_lifecycle(gate_input(&lifecycle)).expect("protocol session gate");
        assert_eq!(crate::ledger::artifact_kind(&lifecycle.manifest_value), "protocol-manifest");
        assert_eq!(crate::ledger::artifact_kind(&lifecycle.install.value), "protocol-install-receipt");
        assert_eq!(crate::ledger::artifact_kind(&gate.value), "protocol-session-gate-receipt");
        assert_eq!(crate::ledger::artifact_kind(&lifecycle.install.endpoints[0].value), "protocol-endpoint");
        assert_eq!(crate::ledger::artifact_kind(&lifecycle.initial_states[0].value), "protocol-session-state");
        assert_eq!(
            crate::ledger::artifact_kind(&lifecycle.operations[0].message.as_ref().expect("message").value),
            "protocol-message"
        );
        assert_eq!(crate::ledger::artifact_kind(&lifecycle.operations[0].receipt.value), "protocol-operation-receipt");
        let dir = temp_dir("catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let imported = crate::ledger::import_artifact(&ledger_root, &lifecycle.install.value).expect("ledger import");
        assert_eq!(imported.artifact_kind, "protocol-install-receipt");
        let listed = crate::catalog::list(&registry, Some(&ledger_root), &ListInput {
            kind: Some("protocol-install-receipt".to_string()),
            visibility: VisibilityInput::default(),
        })
        .expect("catalog list");
        assert_eq!(listed.items.len(), 1);
        let request = crate::catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string(
            "protocol-install-receipt",
        )])])
        .expect("mcp request");
        let mcp = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render mcp").contains("protocol-install-receipt"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_generated_linear_protocols_install_and_roundtrip(tc: TestCase) {
        let step_count = usize::try_from(tc.draw(hegel::generators::integers::<u64>().min_value(1).max_value(3)))
            .expect("usize step count");
        let mut steps = Vec::with_capacity(step_count);
        let mut labels = Vec::with_capacity(step_count);
        let mut payloads = Vec::with_capacity(step_count);
        for index in 0..step_count {
            let label = format!("l{index}");
            labels.push(label.clone());
            payloads.push(ProtocolPayloadInput {
                tag: label.clone(),
                schema_ref: test_ref(&format!("schema-{index}")),
            });
            let is_even = index % 2 == 0;
            let (from_role, to_role) = if is_even {
                ("client", "server")
            } else {
                ("server", "client")
            };
            steps.push(ProtocolCommInput {
                from_role: from_role.to_string(),
                to_role: to_role.to_string(),
                label: label.clone(),
                payload_tag: label,
            });
        }
        let global = protocol_global_script_value(&steps).expect("generated global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:generated".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels,
            payloads,
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("generated manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("generated install");
        assert_eq!(install.decision, "pass");
        assert_eq!(install.endpoints.len(), 2);
        let parsed = parse_protocol_install_receipt(&install.value).expect("parse install receipt");
        assert_eq!(parsed.receipt_ref, install.receipt_ref);
    }
