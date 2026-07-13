
    #[test]
    fn discovered_request_replacement_denies_without_cleanup() {
        // r[verify molten.node.cap_std_request_lifecycle]
        // r[verify molten.node.cap_std_validation]
        let root_path = initialized_control_root("node-control-request-replacement", "node:request-replacement");
        let request = status_request().expect("status request");
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: &root_path,
            request_value: &request.value,
        })
        .expect("submit request");
        let root = crate::node_state::NodeStateRoot::open(&root_path).expect("state root");
        let pending = pending_control_request_by_name(&root, &submitted.inbox_entry).expect("discover request");
        let replacement = shutdown_request().expect("replacement request");
        let replacement_text = crate::preserves_rail::to_text(&replacement.value).expect("replacement text");
        let entry_path = crate::node_state::NodeStatePath::parse(&submitted.inbox_entry).expect("entry path");
        let inbox = root.control_inbox().expect("inbox");
        inbox.write(&entry_path, replacement_text.as_bytes()).expect("replace request");

        let error = dispatch_pending_control_request(&root, pending).expect_err("replacement must deny");
        assert!(error.to_string().contains("changed between discovery and dispatch"));
        assert_eq!(
            inbox.read_to_string(&entry_path, crate::node_state::MAX_NODE_STATE_FILE_BYTES)
                .expect("replacement remains"),
            replacement_text
        );
    }

    #[test]
    fn stale_logical_request_entry_denies_before_dispatch() {
        // r[verify molten.node.cap_std_request_lifecycle]
        // r[verify molten.node.cap_std_validation]
        let root_path = initialized_control_root("node-control-stale-entry", "node:stale-entry");
        let error = dispatch_control_request_entry(&ControlDispatchEntryInput {
            state_root: &root_path,
            request_entry: Some("missing.preserves"),
        })
        .expect_err("stale entry must deny");
        assert!(error.to_string().contains("is not pending"));
    }

    #[cfg(unix)]
    #[test]
    fn replaced_control_lock_cannot_redirect_cleanup() {
        // r[verify molten.node.cap_std_request_lifecycle]
        // r[verify molten.node.cap_std_validation]
        use std::os::unix::fs::symlink;

        let root_path = initialized_control_root("node-control-lock-replacement", "node:lock-replacement");
        let root = crate::node_state::NodeStateRoot::open(&root_path).expect("state root");
        let outside_root =
            crate::test_support::process_workspace("node_control_lock_outside").expect("outside workspace");
        std::fs::create_dir_all(&outside_root).expect("outside root");
        let outside_path = outside_root.join("outside.lock");
        std::fs::write(&outside_path, b"outside").expect("outside lock target");
        let lock_path = root_path.join(CONTROL_LOCK_FILE);
        std::fs::remove_file(&lock_path).expect("remove original lock");
        symlink(&outside_path, &lock_path).expect("replace lock with symlink");

        let error = remove_active_lock(&root).expect_err("replacement lock cleanup must deny");
        assert!(error.to_string().contains("regular file"));
        assert_eq!(std::fs::read(&outside_path).expect("outside bytes"), b"outside");
    }

    fn live_profile_send_case() -> (std::path::PathBuf, ControlLiveTicket, SendMaterial) {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let (root, identity) = init_send_case();
            let state_root = crate::node_state::NodeStateRoot::open(&root).expect("open node state root");
            let lookup = iroh::address_lookup::memory::MemoryLookup::new();
            let receiver_endpoint = live_gossip_endpoint(
                &lookup,
                Some(stable_live_endpoint_secret(&state_root, &identity).expect("transport secret")),
            )
            .await
            .expect("receiver endpoint");
            let receiver_addr = receiver_endpoint.addr();
            let ticket =
                live_ticket_for_bound_endpoint(&state_root, &identity, DEFAULT_CONTROL_INGRESS_TOPIC, &receiver_addr)
                .expect("live ticket");
            let material = send_material(&root, &ticket);
            (root, ticket, material)
        })
    }

    #[test]
    fn live_profiles_preflight_passes_and_receipt_binds_refs() {
        let (root, ticket, material) = live_profile_send_case();
        let topology_ref = local_ref("live-topology-profile", "send").expect("topology ref");
        let transport_ref = local_ref("live-transport-profile", "send").expect("transport ref");
        let admitted_ticket_refs = vec![ticket.ticket_ref.clone()];
        let admitted_peer_refs = vec![material.admission.admission_ref.clone()];
        let allowed_alpns = [LIVE_CONTROL_INGRESS_TRANSPORT];
        let topology = LiveTopologyProfile {
            profile_ref: &topology_ref,
            expected_node: "node:live-send",
            expected_peer: "peer:external-send",
            expected_topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            expected_endpoint: Some(&ticket.live_endpoint_id),
            allowed_alpns: &allowed_alpns,
            ticket_refs: &admitted_ticket_refs,
            peer_admission_refs: &admitted_peer_refs,
            role: Some("control-live-send"),
        };
        let profiled_attempts = DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS + 1;
        const PROFILE_TIMEOUT_DIVISOR: u64 = 2;
        let profiled_timeout_ms = MAX_CONTROL_LIVE_SEND_TIMEOUT_MS / PROFILE_TIMEOUT_DIVISOR;
        let transport = LiveTransportProfile {
            profile_ref: &transport_ref,
            max_attempts: profiled_attempts,
            join_timeout_ms: profiled_timeout_ms,
            publish_timeout_ms: profiled_timeout_ms,
            relay_preference: LIVE_PROFILE_RELAY_AUTO,
        };
        let mut input = build_send_input(&root, &ticket, &material);
        input.topology_profile = Some(&topology);
        input.transport_profile = Some(&transport);

        let profile = preflight_live_profiles(&input).expect("profile preflight");
        assert_eq!(profile.decision, "pass");
        assert_eq!(profile.topology_profile_ref.as_deref(), Some(topology_ref.as_str()));
        assert_eq!(profile.transport_profile_ref.as_deref(), Some(transport_ref.as_str()));
        assert_eq!(profile.effective_max_attempts, profiled_attempts);
        assert_eq!(profile.effective_join_timeout_ms, profiled_timeout_ms);
        assert!(profile.diagnostics.is_empty());

        let envelope = send_envelope(&input, &ticket).expect("send envelope");
        let receipt = live_send_receipt_value(&LiveSendReceiptValueInput {
            decision: "pass",
            from_peer: input.from_peer,
            ticket: &ticket,
            envelope: &envelope,
            transport_receipt_ref: None,
            topology_profile_ref: selected_topology_profile_ref(&input),
            transport_profile_ref: selected_transport_profile_ref(&input),
            effective_max_attempts: effective_live_send_max_attempts(&input),
            effective_join_timeout_ms: effective_live_send_join_timeout_ms(&input),
            diagnostics: &[],
        })
        .expect("send receipt");
        let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
        assert!(text.contains("live-profiles"));
        assert!(text.contains(&topology_ref));
        assert!(text.contains(&transport_ref));
        assert!(text.contains(&profiled_attempts.to_string()));
        assert!(text.contains(&profiled_timeout_ms.to_string()));

        let transport_receipt = live_transport_receipt_value(&LiveTransportReceiptValueInput {
            operation: "publish",
            decision: "pass",
            node_id: input.from_peer,
            delivered_from: None,
            envelope: &envelope,
            ingress_receipt_ref: None,
            topology_profile_ref: selected_topology_profile_ref(&input),
            transport_profile_ref: selected_transport_profile_ref(&input),
            effective_max_attempts: Some(effective_live_send_max_attempts(&input)),
            effective_join_timeout_ms: Some(effective_live_send_join_timeout_ms(&input)),
            diagnostics: &[],
        })
        .expect("transport receipt");
        let transport_text = crate::preserves_rail::to_text(&transport_receipt).expect("transport text");
        assert!(transport_text.contains("live-profiles"));
        assert!(transport_text.contains(&topology_ref));
        assert!(transport_text.contains(&transport_ref));
    }

    #[test]
    fn live_profiles_no_profile_receipt_records_explicit_caveat() {
        let (root, ticket, material) = live_profile_send_case();
        let input = build_send_input(&root, &ticket, &material);
        let envelope = send_envelope(&input, &ticket).expect("send envelope");
        let receipt = live_send_receipt_value(&LiveSendReceiptValueInput {
            decision: "pass",
            from_peer: input.from_peer,
            ticket: &ticket,
            envelope: &envelope,
            transport_receipt_ref: None,
            topology_profile_ref: selected_topology_profile_ref(&input),
            transport_profile_ref: selected_transport_profile_ref(&input),
            effective_max_attempts: effective_live_send_max_attempts(&input),
            effective_join_timeout_ms: effective_live_send_join_timeout_ms(&input),
            diagnostics: &[],
        })
        .expect("send receipt");
        let text = crate::preserves_rail::to_text(&receipt).expect("receipt text");
        assert!(text.contains("explicit-flags-no-profile"));
        assert!(text.contains(&DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS.to_string()));
        assert!(text.contains(&input.join_timeout_ms.to_string()));
    }

    #[test]
    fn live_profiles_preflight_denies_mismatched_caps_and_non_authority() {
        let (root, ticket, material) = live_profile_send_case();
        let topology_ref = local_ref("live-topology-profile", "wrong").expect("topology ref");
        let transport_ref = local_ref("live-transport-profile", "over-cap").expect("transport ref");
        let admitted_ticket_refs = vec![ticket.ticket_ref.clone()];
        let admitted_peer_refs = vec![material.admission.admission_ref.clone()];
        let wrong_alpns = ["not-live-gossip"];
        let wrong_endpoint = "node:wrong-live-endpoint";
        let wrong_peer = "peer:wrong-external-send";
        let wrong_topic = "node-control-wrong-topic";
        let topology = LiveTopologyProfile {
            profile_ref: &topology_ref,
            expected_node: "node:live-send",
            expected_peer: wrong_peer,
            expected_topic: wrong_topic,
            expected_endpoint: Some(wrong_endpoint),
            allowed_alpns: &wrong_alpns,
            ticket_refs: &admitted_ticket_refs,
            peer_admission_refs: &admitted_peer_refs,
            role: Some("control-live-send"),
        };
        let over_attempt_cap = MAX_CONTROL_LIVE_SEND_ATTEMPTS + 1;
        let over_timeout_cap = MAX_CONTROL_LIVE_SEND_TIMEOUT_MS + 1;
        let transport = LiveTransportProfile {
            profile_ref: &transport_ref,
            max_attempts: over_attempt_cap,
            join_timeout_ms: over_timeout_cap,
            publish_timeout_ms: over_timeout_cap,
            relay_preference: LIVE_PROFILE_RELAY_AUTO,
        };
        let empty_refs: Vec<String> = Vec::new();
        let mut input = build_send_input(&root, &ticket, &material);
        input.topology_profile = Some(&topology);
        input.transport_profile = Some(&transport);
        input.peer_bootstrap_refs = &empty_refs;
        input.authority_refs = &empty_refs;
        input.policy_refs = &empty_refs;
        input.resource_refs = &empty_refs;

        let profile = preflight_live_profiles(&input).expect("profile preflight");
        assert_eq!(profile.decision, "deny");
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("peer")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("topic")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("endpoint")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("ALPN")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("attempts")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("join timeout")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("operation authority")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("policy admission")));
        assert!(profile.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource authority")));
    }
