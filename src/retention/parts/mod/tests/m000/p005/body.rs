
    fn assert_two_node_admission(root: &Path, refs: TwoNodeRefs, imported: RemoteGcClearanceLiveImportWorkflow) {
        let clearance_ref = imported.import.clearance_ref.expect("clearance stored");
        let TwoNodeRefs {
            requester_ref,
            peer_ref,
            object_ref,
            remote_ref,
            policy,
            authority,
            support,
            index,
            remote_gc,
        } = refs;
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root,
            evidence: &DestructiveEvidence {
                requester_ref: Some(requester_ref),
                policy_refs: vec![policy],
                authority_refs: vec![authority],
                evidence_refs: vec![support],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![peer_ref],
                remote_refs: vec![remote_ref],
                reference_index_refs: vec![index],
                remote_gc_refs: vec![remote_gc],
                remote_clearance_refs: vec![clearance_ref],
                is_reference_index_complete: true,
            },
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("two-node destructive admission");
        assert_eq!(admission.decision, "pass");
    }

    async fn start_bound_live_node(state_root: &Path, topic: &str) -> LiveNodeHarness {
        let identity_text = fs::read_to_string(state_root.join("identity.preserves")).expect("node identity file");
        let identity_value = crate::preserves_rail::parse_text(&identity_text).expect("parse node identity file");
        let identity = crate::node_identity::parse_identity(&identity_value).expect("parse node identity");
        let seed = blake3::hash(
            format!("molten.node-control.live.endpoint.v1:{}:{}", identity.node_id, identity.endpoint_id).as_bytes(),
        );
        let lookup = iroh::address_lookup::memory::MemoryLookup::new();
        let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .address_lookup(lookup.clone())
            .alpns(vec![iroh_gossip::ALPN.to_vec()])
            .clear_ip_transports()
            .bind_addr((Ipv4Addr::LOCALHOST, 0))
            .expect("live endpoint bind addr")
            .secret_key(iroh::SecretKey::from_bytes(seed.as_bytes()))
            .bind()
            .await
            .expect("live endpoint bind");
        let endpoint_addr = endpoint.addr();
        let live_endpoint_id = format!("iroh:{}", endpoint.id());
        let address_refs = endpoint_addr.addrs.iter().map(ToString::to_string).collect::<Vec<_>>();
        let ticket_value = crate::node_daemon::control_live_ticket_value(&crate::node_daemon::ControlLiveTicketInput {
            node_id: &identity.node_id,
            identity_ref: &identity.identity_ref,
            logical_endpoint_id: &identity.endpoint_id,
            live_endpoint_id: &live_endpoint_id,
            topic,
            address_refs: &address_refs,
            policy_refs: &identity.policy_refs,
            evidence_refs: &identity.receipt_refs,
        })
        .expect("bound live ticket value");
        let ticket = crate::node_daemon::parse_control_live_ticket(&ticket_value).expect("bound live ticket");
        lookup.add_endpoint_info(endpoint_addr);
        let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
        let router = iroh::protocol::Router::builder(endpoint).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
        let topic = gossip.subscribe(local_live_topic_id(topic), Vec::new()).await.expect("subscribe live topic");
        LiveNodeHarness { ticket, topic, router }
    }

    fn install_live_direction_evidence(input: &LiveDirectionEvidenceInput<'_>) -> LiveDirectionEvidence {
        let admission = crate::node_daemon::admit_control_live_peer(&crate::node_daemon::ControlLivePeerAdmitInput {
            state_root: input.receiver_root,
            ticket_value: &input.receiver_ticket.value,
            peer_id: input.sender_node_id,
            sequence: 1,
            expires_at: None,
            policy_refs: input.policy_refs,
            evidence_refs: &[],
        })
        .expect("live peer admission");
        let import =
            crate::node_daemon::import_control_live_ticket(&crate::node_daemon::ControlLiveTicketImportInput {
                state_root: input.sender_root,
                ticket_value: &input.receiver_ticket.value,
                peer_admission_value: Some(&admission.value),
                expected_node: Some(input.receiver_node_id),
                expected_topic: Some(input.topic),
                expected_endpoint: Some(&input.receiver_ticket.live_endpoint_id),
                expected_peer: Some(input.sender_node_id),
                as_of_sequence: 1,
            })
            .expect("sender imports live ticket admission");
        assert_eq!(import.decision, "pass");
        let operations = vec!["gate".to_string()];
        let grant_value =
            crate::node_daemon::control_authority_grant_value(&crate::node_daemon::ControlAuthorityGrantInput {
                peer_id: input.sender_node_id,
                node_id: input.receiver_node_id,
                operations: &operations,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                policy_refs: input.policy_refs,
                revocation_refs: &[],
                evidence_refs: &[],
            })
            .expect("live authority grant value");
        let sender_grant = crate::node_daemon::import_control_authority_grant(input.sender_root, &grant_value)
            .expect("sender imports authority grant");
        let receiver_grant = crate::node_daemon::import_control_authority_grant(input.receiver_root, &grant_value)
            .expect("receiver imports authority grant");
        assert_eq!(sender_grant.grant_ref, receiver_grant.grant_ref);
        LiveDirectionEvidence {
            peer_bootstrap_refs: vec![admission.admission_ref],
            authority_refs: vec![sender_grant.grant_ref],
        }
    }

    async fn receive_one_live_ingress(
        state_root: &Path,
        topic: &str,
        receiver_node: &str,
        receiver: &mut iroh_gossip::api::GossipTopic,
    ) -> crate::node_daemon::ControlLiveIngressReceive {
        for _ in 0..16 {
            let event = tokio::time::timeout(Duration::from_millis(1_000), receiver.next())
                .await
                .expect("live receive event timeout")
                .expect("live receive stream ended")
                .expect("live receive event");
            if let Some(received) =
                crate::node_daemon::receive_control_live_ingress_event(state_root, &event, topic, receiver_node)
                    .expect("receive live ingress")
            {
                return received;
            }
        }
        panic!("live receiver did not observe ingress envelope");
    }

    fn local_live_topic_id(topic: &str) -> iroh_gossip::TopicId {
        let digest = blake3::hash(format!("molten.node-control.live.topic.v1:{topic}").as_bytes());
        iroh_gossip::TopicId::from_bytes(*digest.as_bytes())
    }

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

    struct LiveDirectionRefs {
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        evidence_refs: Vec<String>,
    }

    fn live_direction_refs(root: &std::path::Path, peer_id: &str, label: &str) -> LiveDirectionRefs {
        let policy_refs = vec![fake_ref(&format!("{label}-node-policy"))];
        let resource_refs = vec![fake_ref(&format!("{label}-node-resource"))];
        let evidence_refs = vec![fake_ref(&format!("{label}-node-evidence"))];
        let ticket =
            crate::node_daemon::export_control_live_ticket(&crate::node_daemon::ControlLiveTicketExportInput {
                state_root: root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy_refs,
                evidence_refs: &evidence_refs,
            })
            .expect("export live ticket");
        let admission = crate::node_daemon::admit_control_live_peer(&crate::node_daemon::ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id,
            sequence: 1,
            expires_at: None,
            policy_refs: &policy_refs,
            evidence_refs: &evidence_refs,
        })
        .expect("admit live peer");
        assert_eq!(admission.decision, "pass");
        let operations = vec!["gate".to_string()];
        let revocation_refs = Vec::new();
        let authority_value =
            crate::node_daemon::control_authority_grant_value(&crate::node_daemon::ControlAuthorityGrantInput {
                peer_id,
                node_id: &ticket.node_id,
                operations: &operations,
                target_scope: "*",
                resource_scope: "*",
                epoch: 1,
                expires_at: None,
                policy_refs: &policy_refs,
                revocation_refs: &revocation_refs,
                evidence_refs: &evidence_refs,
            })
            .expect("authority grant value");
        let authority =
            crate::node_daemon::import_control_authority_grant(root, &authority_value).expect("import authority grant");
        LiveDirectionRefs {
            peer_bootstrap_refs: vec![admission.admission_ref],
            authority_refs: vec![authority.grant_ref],
            policy_refs,
            resource_refs,
            evidence_refs,
        }
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
