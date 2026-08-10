
    #[test]
    fn facade_generation_receipts_bind_admitted_manifest_and_non_claims() {
        let manifest_value = request_response_manifest_value().expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install");
        let receipt = generate_protocol_facade_receipt(ProtocolFacadeGenerationInput {
            install_receipt: install.value.clone(),
            generator_ref: test_ref("facade-generator"),
            artifact_ref: test_ref("facade-artifact"),
        })
        .expect("facade generation receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.manifest_ref, install.manifest.manifest_ref);
        assert_eq!(receipt.install_ref, install.receipt_ref);
        assert_eq!(receipt.endpoint_refs, install_endpoint_refs(&install));
        assert!(receipt.non_claims.iter().any(|claim| claim == FACADE_NON_CLAIM_CHORUS));
        assert!(receipt.non_claims.iter().any(|claim| claim == FACADE_NON_CLAIM_JSON));
        assert!(to_text(&receipt.value).expect("facade receipt text").contains("facade-non-authority"));
        let parsed = parse_protocol_facade_generation_receipt(&receipt.value).expect("parse facade receipt");
        assert_eq!(parsed.receipt_ref, receipt.receipt_ref);
    }

    #[test]
    fn facade_generation_denies_non_projectable_install_without_endpoints() {
        let global = protocol_global_script_value(&[ProtocolCommInput {
            from_role: "client".to_string(),
            to_role: "client".to_string(),
            label: "loop".to_string(),
            payload_tag: "loop".to_string(),
        }])
        .expect("global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:facade-bad".to_string(),
            roles: vec!["client".to_string()],
            labels: vec!["loop".to_string()],
            payloads: vec![ProtocolPayloadInput {
                tag: "loop".to_string(),
                schema_ref: test_ref("loop-schema"),
            }],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install");
        assert_eq!(install.decision, "deny");
        let receipt = generate_protocol_facade_receipt(ProtocolFacadeGenerationInput {
            install_receipt: install.value,
            generator_ref: test_ref("facade-generator"),
            artifact_ref: test_ref("facade-artifact"),
        })
        .expect("denied facade generation receipt");
        assert_eq!(receipt.decision, "deny");
        assert!(receipt.endpoint_refs.is_empty());
        assert!(receipt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("passing projectability install")));
        assert!(to_text(&receipt.value).expect("facade deny text").contains("no-chorus-compatibility"));
    }

    #[test]
    fn facade_sans_io_send_matches_projected_runtime_without_shell_effects() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let expected_send = &lifecycle.operations[0];
        let client0 = &lifecycle.initial_states[0];
        let facade = evaluate_protocol_facade_transition(ProtocolFacadeTransitionInput {
            operation: "send".to_string(),
            state: client0.value.clone(),
            peer: Some("server".to_string()),
            label: "request".to_string(),
            payload_tag: Some("request".to_string()),
            body_or_ref: Some(record("body", vec![string("hello")])),
            message: None,
            authority_refs: expected_send.receipt.authority_refs.clone(),
            resource_refs: expected_send.receipt.resource_refs.clone(),
            evidence_refs: vec![lifecycle.install.receipt_ref.clone()],
        })
        .expect("facade transition");
        assert_eq!(facade.decision, "pass");
        assert_eq!(
            facade.message_descriptor.as_ref().expect("facade message").message_ref,
            expected_send.message.as_ref().expect("runtime message").message_ref
        );
        assert_eq!(
            facade.next_state.as_ref().expect("facade next").state_ref,
            expected_send.next_state.as_ref().expect("runtime next").state_ref
        );
        assert!(to_text(&facade.value).expect("facade transition text").contains("no-shell-effects"));
    }

    #[test]
    fn facade_transition_denies_wrong_label_missing_evidence_and_replay() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let client0 = &lifecycle.initial_states[0];
        let wrong_label = evaluate_protocol_facade_transition(ProtocolFacadeTransitionInput {
            operation: "send".to_string(),
            state: client0.value.clone(),
            peer: Some("server".to_string()),
            label: "response".to_string(),
            payload_tag: Some("response".to_string()),
            body_or_ref: Some(record("body", vec![string("bad")])) ,
            message: None,
            authority_refs: lifecycle.operations[0].receipt.authority_refs.clone(),
            resource_refs: lifecycle.operations[0].receipt.resource_refs.clone(),
            evidence_refs: vec![lifecycle.install.receipt_ref.clone()],
        })
        .expect("wrong label facade transition");
        assert_eq!(wrong_label.decision, "deny");
        assert!(wrong_label.message_descriptor.is_none());
        assert!(wrong_label.next_state.is_none());
        assert!(wrong_label
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(PROTOCOL_TRANSITION_SEND_MISMATCH)));

        let missing_evidence = evaluate_protocol_facade_transition(ProtocolFacadeTransitionInput {
            operation: "send".to_string(),
            state: client0.value.clone(),
            peer: Some("server".to_string()),
            label: "request".to_string(),
            payload_tag: Some("request".to_string()),
            body_or_ref: Some(record("body", vec![string("bad")])) ,
            message: None,
            authority_refs: Vec::new(),
            resource_refs: lifecycle.operations[0].receipt.resource_refs.clone(),
            evidence_refs: vec![lifecycle.install.receipt_ref.clone()],
        })
        .expect("missing evidence facade transition");
        assert_eq!(missing_evidence.decision, "deny");
        assert!(missing_evidence.message_descriptor.is_none());
        assert!(missing_evidence
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("authority")));

        let server_after_receive = lifecycle.operations[1].next_state.as_ref().expect("server next");
        let request_message = lifecycle.operations[0].message.as_ref().expect("request message");
        let replay = evaluate_protocol_facade_transition(ProtocolFacadeTransitionInput {
            operation: "receive".to_string(),
            state: server_after_receive.value.clone(),
            peer: None,
            label: "request".to_string(),
            payload_tag: Some("request".to_string()),
            body_or_ref: None,
            message: Some(request_message.value.clone()),
            authority_refs: lifecycle.operations[1].receipt.authority_refs.clone(),
            resource_refs: lifecycle.operations[1].receipt.resource_refs.clone(),
            evidence_refs: Vec::new(),
        })
        .expect("replay facade transition");
        assert_eq!(replay.decision, "deny");
        assert!(replay.next_state.is_none());
        assert!(replay
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("duplicate protocol message replay")));
    }

    #[test]
    fn facade_payload_access_is_role_scoped_and_evidence_bound() {
        let payload = ProtocolLocatedPayload {
            owner_role: "client".to_string(),
            payload_tag: "request".to_string(),
            payload_ref: test_ref("payload"),
        };
        let evidence = vec![test_ref("payload-evidence")];
        let allowed = evaluate_protocol_facade_payload_access(ProtocolFacadePayloadAccessInput {
            payload: &payload,
            local_role: "client",
            expected_payload_tag: "request",
            evidence_refs: &evidence,
        })
        .expect("allowed payload access");
        assert_eq!(allowed.decision, "pass");
        assert_eq!(allowed.payload_ref.as_deref(), Some(payload.payload_ref.as_str()));

        let wrong_role = evaluate_protocol_facade_payload_access(ProtocolFacadePayloadAccessInput {
            payload: &payload,
            local_role: "server",
            expected_payload_tag: "request",
            evidence_refs: &evidence,
        })
        .expect("wrong role payload access");
        assert_eq!(wrong_role.decision, "deny");
        assert!(wrong_role.payload_ref.is_none());
        assert!(wrong_role
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("owner role mismatch")));

        let missing_evidence = evaluate_protocol_facade_payload_access(ProtocolFacadePayloadAccessInput {
            payload: &payload,
            local_role: "client",
            expected_payload_tag: "request",
            evidence_refs: &[],
        })
        .expect("missing evidence payload access");
        assert_eq!(missing_evidence.decision, "deny");
        assert!(missing_evidence
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("requires evidence")));
    }

    #[test]
    fn facade_dependency_boundary_rejects_chorus_import_drift() {
        // r[verify molten.choreography.chorus_design_reference]
        let clean = protocol_facade_dependency_boundary_diagnostics(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")),
        );
        assert!(clean.is_empty());
        let drift = protocol_facade_dependency_boundary_diagnostics("chorus_lib = \"0.1\"", "");
        assert!(drift.iter().any(|diagnostic| diagnostic.contains("chorus_lib")));
    }
