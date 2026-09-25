
    #[test]
    fn protocol_session_gate_denies_ambiguous_branch_replay() {
        let empty_left = ProtocolBranchInput {
            label: "left".to_string(),
            steps: Vec::new(),
        };
        let empty_right = ProtocolBranchInput {
            label: "right".to_string(),
            steps: Vec::new(),
        };
        let global = protocol_global_choice_value(&ProtocolChoiceInput {
            decider: "client".to_string(),
            branches: vec![empty_left, empty_right],
        })
        .expect("ambiguous choice global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:ambiguous-choice".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels: vec!["left".to_string(), "right".to_string()],
            payloads: vec![
                ProtocolPayloadInput {
                    tag: "left".to_string(),
                    schema_ref: test_ref("left-schema"),
                },
                ProtocolPayloadInput {
                    tag: "right".to_string(),
                    schema_ref: test_ref("right-schema"),
                },
            ],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("ambiguous choice manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install ambiguous choice");
        let client = start_protocol_session(&install, "client", "session:ambiguous", auth(), resources())
            .expect("client state");
        let branch = choose_protocol_branch(ProtocolBranchOperationInput {
            state: client.value.clone(),
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("choose branch");
        let gate = gate_protocol_session_lifecycle(ProtocolSessionGateInput {
            install_receipt: install.value,
            initial_states: vec![client.value],
            operation_receipts: vec![branch.receipt.value],
            messages: Vec::new(),
            next_states: vec![branch.next_state.expect("branch next").value],
        })
        .expect("ambiguous branch gate");
        assert_eq!(gate.decision, "deny");
        assert!(gate.diagnostics.iter().any(|diagnostic| diagnostic.contains("ambiguous")));
    }

    #[test]
    fn protocol_session_gate_accepts_generated_branch_offer_trace() {
        let install = branch_gate_install();
        let client = start_protocol_session(&install, "client", "session:branch-gate", auth(), resources())
            .expect("client state");
        let server = start_protocol_session(&install, "server", "session:branch-gate", auth(), resources())
            .expect("server state");
        let branch = choose_protocol_branch(ProtocolBranchOperationInput {
            state: client.value.clone(),
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("choose branch");
        let offer = offer_protocol_branch(ProtocolBranchOperationInput {
            state: server.value.clone(),
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("offer branch");
        let branch_next = branch.next_state.clone().expect("branch next");
        let offer_next = offer.next_state.clone().expect("offer next");
        let send = send_protocol_message(ProtocolSendInput {
            state: branch_next.value.clone(),
            to_role: "server".to_string(),
            label: "left".to_string(),
            payload_tag: "left".to_string(),
            body_or_ref: record("body", vec![string("left")]),
            authority_refs: auth(),
            resource_refs: resources(),
            evidence_refs: vec![branch.receipt.receipt_ref.clone()],
        })
        .expect("branch send");
        let message = send.message.clone().expect("branch message");
        let receive = receive_protocol_message(ProtocolReceiveInput {
            state: offer_next.value.clone(),
            message: message.value.clone(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("branch receive");
        let gate = gate_protocol_session_lifecycle(ProtocolSessionGateInput {
            install_receipt: install.value,
            initial_states: vec![client.value, server.value],
            operation_receipts: vec![
                branch.receipt.value,
                offer.receipt.value,
                send.receipt.value,
                receive.receipt.value,
            ],
            messages: vec![message.value],
            next_states: vec![
                branch_next.value,
                offer_next.value,
                send.next_state.expect("send next").value,
                receive.next_state.expect("receive next").value,
            ],
        })
        .expect("branch gate");
        let expected_terminal_roles = 2;
        assert_eq!(gate.decision, "pass");
        assert_eq!(gate.final_state_count, expected_terminal_roles);
    }

    /// Installs a client-decided left/right branch protocol with one client-to-server message per branch.
    fn branch_gate_install() -> ProtocolInstallReceipt {
        let left = ProtocolBranchInput {
            label: "left".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "left".to_string(),
                payload_tag: "left".to_string(),
            }],
        };
        let right = ProtocolBranchInput {
            label: "right".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "right".to_string(),
                payload_tag: "right".to_string(),
            }],
        };
        let global = protocol_global_choice_value(&ProtocolChoiceInput {
            decider: "client".to_string(),
            branches: vec![left, right],
        })
        .expect("branch global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:branch-gate".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels: vec!["left".to_string(), "right".to_string()],
            payloads: vec![
                ProtocolPayloadInput {
                    tag: "left".to_string(),
                    schema_ref: test_ref("left-schema"),
                },
                ProtocolPayloadInput {
                    tag: "right".to_string(),
                    schema_ref: test_ref("right-schema"),
                },
            ],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("branch manifest");
        install_protocol_manifest_value(&manifest_value).expect("install branch")
    }

    #[test]
    fn bad_payload_tag_and_replay_deny() {
        let lifecycle = request_response_lifecycle().expect("lifecycle");
        let server = lifecycle.initial_states[1].clone();
        let request = lifecycle.operations[0].message.as_ref().expect("request").clone();
        let bad_message = protocol_message_value(&ProtocolMessageInput {
            protocol_ref: request.protocol_ref,
            session_id: request.session_id,
            from_role: request.from_role,
            to_role: request.to_role,
            label: request.label,
            payload_tag: "response".to_string(),
            body_or_ref: request.body_or_ref,
            sequence: request.sequence,
            evidence_refs: Vec::new(),
        })
        .expect("bad tag message");
        let bad = receive_protocol_message(ProtocolReceiveInput {
            state: server.value,
            message: bad_message,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("bad tag deny");
        assert_eq!(bad.decision, "deny");

        let after_receive = lifecycle.operations[1].next_state.as_ref().expect("next state").clone();
        let replay = receive_protocol_message(ProtocolReceiveInput {
            state: after_receive.value,
            message: request.value,
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("replay deny");
        assert_eq!(replay.decision, "deny");
    }

    #[test]
    fn branch_choice_and_offer_follow_projected_state() {
        let left = ProtocolBranchInput {
            label: "left".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "left".to_string(),
                payload_tag: "left".to_string(),
            }],
        };
        let right = ProtocolBranchInput {
            label: "right".to_string(),
            steps: vec![ProtocolCommInput {
                from_role: "client".to_string(),
                to_role: "server".to_string(),
                label: "right".to_string(),
                payload_tag: "right".to_string(),
            }],
        };
        let global = protocol_global_choice_value(&ProtocolChoiceInput {
            decider: "client".to_string(),
            branches: vec![left, right],
        })
        .expect("choice global");
        let manifest_value = protocol_manifest_value(&ProtocolManifestInput {
            protocol_id: "proto:choice".to_string(),
            roles: vec!["client".to_string(), "server".to_string()],
            labels: vec!["left".to_string(), "right".to_string()],
            payloads: vec![
                ProtocolPayloadInput {
                    tag: "left".to_string(),
                    schema_ref: test_ref("left-schema"),
                },
                ProtocolPayloadInput {
                    tag: "right".to_string(),
                    schema_ref: test_ref("right-schema"),
                },
            ],
            global,
            policy_refs: vec![test_ref("policy")],
            capability_refs: vec![test_ref("capability")],
            resource_refs: vec![test_ref("resource")],
        })
        .expect("choice manifest");
        let install = install_protocol_manifest_value(&manifest_value).expect("install choice");
        assert_eq!(install.decision, "pass");
        let client = start_protocol_session(&install, "client", "session:choice", auth(), resources()).expect("client");
        let server = start_protocol_session(&install, "server", "session:choice", auth(), resources()).expect("server");
        let branch = choose_protocol_branch(ProtocolBranchOperationInput {
            state: client.value,
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("choose branch");
        assert_eq!(branch.decision, "pass");
        let offer = offer_protocol_branch(ProtocolBranchOperationInput {
            state: server.value,
            label: "left".to_string(),
            authority_refs: auth(),
            resource_refs: resources(),
            carrier_refs: Vec::new(),
        })
        .expect("offer branch");
        assert_eq!(offer.decision, "pass");
    }
