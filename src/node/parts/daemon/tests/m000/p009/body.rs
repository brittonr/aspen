
    const DENY_CASES: &[DenyCase<'static>] = &[
        DenyCase {
            name: "unknown-grant",
            grant_peer: None,
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "not found",
        },
        DenyCase {
            name: "wrong-peer",
            grant_peer: Some("peer:other"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "does not match peer:case",
        },
        DenyCase {
            name: "wrong-op",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["shutdown"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "does not allow operation status",
        },
        DenyCase {
            name: "wrong-target",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: Some("blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            target_scope: "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "target scope",
        },
        DenyCase {
            name: "wrong-resource",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            epoch: 1,
            expires_at: None,
            is_revoked: false,
            sequence: 1,
            expected: "resource scope",
        },
        DenyCase {
            name: "expired",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(1),
            is_revoked: false,
            sequence: 2,
            expected: "expired at epoch 1",
        },
        DenyCase {
            name: "revoked",
            grant_peer: Some("peer:case"),
            grant_node: "node:live-authority",
            grant_operations: &["status"],
            target_ref: None,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            is_revoked: true,
            sequence: 1,
            expected: "has revocation refs",
        },
    ];

    fn denied_case_refs(root: &Path, case: &DenyCase<'_>) -> DenyCaseRefs {
        let policy_refs = vec![local_ref("node-control-policy", case.name).expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", case.name).expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(root, "peer:case", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let authority_refs = if let Some(grant_peer) = case.grant_peer {
            let operations = case.grant_operations.iter().map(|operation| (*operation).to_string()).collect::<Vec<_>>();
            let revocation_refs = if case.is_revoked {
                vec![local_ref("node-control-revocation", case.name).expect("revocation ref")]
            } else {
                Vec::new()
            };
            let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
                peer_id: grant_peer,
                node_id: case.grant_node,
                operations: &operations,
                target_scope: case.target_scope,
                resource_scope: case.resource_scope,
                epoch: case.epoch,
                expires_at: case.expires_at,
                policy_refs: &policy_refs,
                revocation_refs: &revocation_refs,
                evidence_refs: &[],
            })
            .expect("authority grant value");
            vec![import_control_authority_grant(root, &grant_value).expect("import authority grant").grant_ref]
        } else {
            vec![local_ref("node-control-authority", case.name).expect("authority ref")]
        };
        DenyCaseRefs {
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
        }
    }

    fn assert_denied_case(case: &DenyCase<'_>) {
        let root = temp_dir(&format!("node-control-live-authority-{}", case.name));
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-authority",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let refs = denied_case_refs(&root, case);
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: case.target_ref,
                payload_ref: None,
                authority_refs: &refs.authority_refs,
                policy_refs: &refs.policy_refs,
                resource_refs: &refs.resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:case",
            to_node: "node:live-authority",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: case.sequence,
            peer_bootstrap_refs: &refs.peer_bootstrap_refs,
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish live envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver live envelope");
        assert!(!delivered.has_enqueued, "{} enqueued", case.name);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains(case.expected), "{} receipt: {receipt_text}", case.name);
        assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
    }

    #[test]
    fn control_live_authority_delegation_fails_closed() {
        for case in DENY_CASES {
            assert_denied_case(case);
        }
    }

    #[test]
    fn control_live_transport_receipts_do_not_bootstrap_or_authorize() {
        let root = temp_dir("node-control-live-transport-is-not-authority");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:transport-proof",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "transport-proof").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "transport-proof").expect("resource ref")];
        let valid_authority_refs =
            test_live_authority_refs(&root, "peer:transport", "node:transport-proof", "status", &policy_refs)
                .expect("authority grant ref");
        let transport_ref = transport_receipt_ref(&root, &policy_refs, &resource_refs, &valid_authority_refs);

        assert_transport_ref_does_not_bootstrap(&root, &policy_refs, &resource_refs, &valid_authority_refs, &transport_ref);
        assert_transport_ref_does_not_authorize(&root, &policy_refs, &resource_refs, &transport_ref);
    }

    fn transport_receipt_ref(
        root: &Path,
        policy_refs: &[String],
        resource_refs: &[String],
        authority_refs: &[String],
    ) -> String {
        let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("transport seed request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:transport",
            to_node: "node:transport-proof",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &[],
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("transport seed envelope");
        let transport_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
            operation: "receive",
            decision: "pass",
            node_id: "node:transport-proof",
            delivered_from: Some("peer:transport"),
            envelope: &envelope,
            ingress_receipt_ref: None,
            topology_profile_ref: None,
            transport_profile_ref: None,
            effective_max_attempts: None,
            effective_join_timeout_ms: None,
            diagnostics: &[],
        })
        .expect("transport receipt");
        import_artifact(root, &transport_value).expect("import transport receipt")
    }
