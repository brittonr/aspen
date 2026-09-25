    type TestCase = hegel::TestCase;

    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("protocol-test-ref", vec![string(label)])).expect("test ref")
    }

    #[test]
    fn protocol_refs_reject_malformed_content_refs() {
        for reference in [
            "blake3:short",
            "blake3:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "blake3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let error = require_ref(reference, "protocol regression ref").expect_err("malformed ref must fail closed");
            assert!(error.to_string().contains("canonical content ref"));
        }
    }

    fn auth() -> Vec<String> {
        vec![test_ref("authority")]
    }

    fn resources() -> Vec<String> {
        vec![test_ref("resource")]
    }

    fn gate_input(lifecycle: &RequestResponseLifecycle) -> ProtocolSessionGateInput {
        ProtocolSessionGateInput {
            install_receipt: lifecycle.install.value.clone(),
            initial_states: lifecycle.initial_states.iter().map(|state| state.value.clone()).collect(),
            operation_receipts: lifecycle.operations.iter().map(|operation| operation.receipt.value.clone()).collect(),
            messages: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.message.as_ref().map(|message| message.value.clone()))
                .collect(),
            next_states: lifecycle
                .operations
                .iter()
                .filter_map(|operation| operation.next_state.as_ref().map(|state| state.value.clone()))
                .collect(),
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("molten-protocol-{label}-{}-{id}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn request_response_installs_and_interprets() {
        let lifecycle = request_response_lifecycle().expect("request response lifecycle");
        assert_eq!(lifecycle.install.decision, "pass");
        assert_eq!(lifecycle.install.endpoints.len(), 2);
        assert_eq!(lifecycle.operations.len(), 4);
        for operation in &lifecycle.operations {
            assert_eq!(operation.decision, "pass");
        }
        let gate = gate_protocol_session_lifecycle(gate_input(&lifecycle)).expect("protocol session gate");
        assert_eq!(gate.decision, "pass");
        assert_eq!(gate.operation_count, 4);
        let gate_receipt = parse_protocol_session_gate_receipt(&gate.value).expect("parse protocol gate receipt");
        assert_eq!(gate_receipt.decision, "pass");
        assert_eq!(gate_receipt.operation_refs.len(), 4);
        assert!(matches!(
            lifecycle
                .operations
                .last()
                .expect("last op")
                .next_state
                .as_ref()
                .expect("next")
                .local_state
                .terminal,
            ProtocolLocalTerminal::End
        ));
    }

    #[test]
    fn non_projectable_protocol_denies_install() {
        let global = protocol_global_script_value(&[ProtocolCommInput {
            from_role: "client".to_string(),
            to_role: "client".to_string(),
            label: "loop".to_string(),
            payload_tag: "loop".to_string(),
        }])
        .expect("global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:bad".to_string(),
            roles: vec!["client".to_string()],
            labels: vec!["loop".to_string()],
            payloads: vec![ProtocolPayloadInput {
                tag: "loop".to_string(),
                schema_ref: test_ref("schema"),
            }],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install receipt");
        assert_eq!(install.decision, "deny");
        assert!(install.endpoints.is_empty());
    }

    #[test]
    fn wrong_label_and_missing_authority_deny_before_message() {
        let manifest_value = request_response_manifest_value().expect("manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install");
        let client = start_protocol_session(&install, "client", "session:deny", auth(), resources()).expect("client");
        let wrong = send_protocol_message(ProtocolSendInput {
            state: client.value.clone(),
            to_role: "server".to_string(),
            label: "response".to_string(),
            payload_tag: "response".to_string(),
            body_or_ref: record("body", vec![string("bad")]),
            authority_refs: auth(),
            resource_refs: resources(),
            evidence_refs: Vec::new(),
        })
        .expect("wrong label denial");
        assert_eq!(wrong.decision, "deny");
        assert!(wrong.message.is_none());
        let missing_auth = send_protocol_message(ProtocolSendInput {
            state: client.value,
            to_role: "server".to_string(),
            label: "request".to_string(),
            payload_tag: "request".to_string(),
            body_or_ref: record("body", vec![string("bad")]),
            authority_refs: Vec::new(),
            resource_refs: resources(),
            evidence_refs: Vec::new(),
        })
        .expect("missing auth denial");
        assert_eq!(missing_auth.decision, "deny");
        assert!(missing_auth.receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    #[test]
    fn protocol_endpoint_transition_core_accepts_and_rejects_projected_edges() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let client0 = &lifecycle.initial_states[0];
        let send_request = &lifecycle.operations[0];
        let request_message = send_request.message.as_ref().expect("request message");
        let client1 = send_request.next_state.as_ref().expect("client next state");
        let legal_send = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
            operation: "send",
            prior: client0,
            peer: Some("server"),
            label: "request",
            payload_tag: Some("request"),
            message: Some(request_message),
            next: Some(client1),
        })
        .expect("legal send transition");
        assert_eq!(legal_send.decision, "pass");

        let wrong_peer = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
            operation: "send",
            prior: client0,
            peer: Some("client"),
            label: "request",
            payload_tag: Some("request"),
            message: None,
            next: None,
        })
        .expect("wrong peer transition");
        assert_eq!(wrong_peer.decision, "deny");
        assert!(wrong_peer
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains(PROTOCOL_TRANSITION_SEND_MISMATCH)));

        let wrong_label = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
            operation: "send",
            prior: client0,
            peer: Some("server"),
            label: "response",
            payload_tag: Some("response"),
            message: None,
            next: None,
        })
        .expect("wrong label transition");
        assert_eq!(wrong_label.decision, "deny");

        let wrong_next = evaluate_protocol_endpoint_transition(ProtocolEndpointTransitionInput {
            operation: "send",
            prior: client0,
            peer: Some("server"),
            label: "request",
            payload_tag: Some("request"),
            message: Some(request_message),
            next: Some(&lifecycle.initial_states[1]),
        })
        .expect("wrong next transition");
        assert_eq!(wrong_next.decision, "deny");
        assert!(wrong_next
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == PROTOCOL_TRANSITION_NEXT_BINDING));
    }

    #[test]
    fn protocol_session_gate_denies_missing_next_state() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let mut input = gate_input(&lifecycle);
        input.next_states.pop();
        let gate = gate_protocol_session_lifecycle(input).expect("gate missing next state");
        assert_eq!(gate.decision, "deny");
        assert!(
            gate.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("next state") || diagnostic.contains("terminal"))
        );
    }

    #[test]
    fn protocol_session_gate_denies_missing_message_and_stale_receipt() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let mut missing_message = gate_input(&lifecycle);
        missing_message.messages.clear();
        let missing = gate_protocol_session_lifecycle(missing_message).expect("gate missing message");
        assert_eq!(missing.decision, "deny");
        assert!(missing.diagnostics.iter().any(|diagnostic| diagnostic.contains("message is missing")));

        let mut stale = gate_input(&lifecycle);
        let receipt = &lifecycle.operations[0].receipt;
        stale.operation_receipts[0] = operation_receipt_value(&OperationReceiptValueInput {
            operation: &receipt.operation,
            decision: "pass",
            protocol_ref: &receipt.protocol_ref,
            session_id: &receipt.session_id,
            role: &receipt.role,
            prior_state_ref: &receipt.prior_state_ref,
            message_ref: receipt.message_ref.as_deref(),
            next_state_ref: receipt.next_state_ref.as_deref(),
            sequence: receipt.sequence.saturating_add(1),
            authority_refs: &receipt.authority_refs,
            resource_refs: &receipt.resource_refs,
            carrier_refs: &receipt.carrier_refs,
            diagnostics: &[],
        })
        .expect("stale receipt value");
        let stale_gate = gate_protocol_session_lifecycle(stale).expect("gate stale receipt");
        assert_eq!(stale_gate.decision, "deny");
        assert!(stale_gate
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("sequence") || diagnostic.contains("replay")));
    }
