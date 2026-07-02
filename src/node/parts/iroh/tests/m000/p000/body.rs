    use super::*;

    const ROUTER_GENERATION_ONE: u64 = 1;
    const ROUTER_GENERATION_TWO: u64 = 2;
    const VALID_FRAME_SEQUENCE: u64 = 0;
    const VALID_FRAME_LIMIT: u64 = 1024;
    const OVERSIZED_FRAME_LENGTH: u64 = 1025;
    const RELAY_LATENCY_MS: u64 = 42;
    const WATCHER_OBSERVED_EVENTS: u64 = 7;
    const WATCHER_RETAINED_EVENTS: u64 = 1;
    const METRIC_VALUE: u64 = 3;
    const PORT_DURATION_SECONDS: u64 = 600;
    const EXTERNAL_PORT: u64 = 443;
    const INTERNAL_PORT: u64 = 8443;

    fn refs() -> Vec<String> {
        vec![fixture_ref("ref")]
    }

    fn router_input(operation: &str, generation: u64) -> RouterOperationInput {
        RouterOperationInput {
            operation: operation.to_string(),
            alpn: "molten/node-control/1".to_string(),
            handler_kind: "node-control".to_string(),
            generation,
            prior_generation: None,
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
            shutdown_evidence_ref: None,
        }
    }

    fn installed_registry() -> ProtocolRegistry {
        evaluate_router_operation(&empty_protocol_registry(), &router_input("install", ROUTER_GENERATION_ONE))
            .expect("install")
            .registry
    }

    #[test]
    fn router_installs_replaces_removes_and_denies_unsupported_alpn() {
        let registry = empty_protocol_registry();
        let install =
            evaluate_router_operation(&registry, &router_input("install", ROUTER_GENERATION_ONE)).expect("install");
        assert_eq!(install.decision, "pass");
        assert!(install.registry.handlers.contains_key("molten/node-control/1"));

        let mut replace_input = router_input("replace", ROUTER_GENERATION_TWO);
        replace_input.prior_generation = Some(ROUTER_GENERATION_ONE);
        replace_input.shutdown_evidence_ref = Some(fixture_ref("shutdown"));
        let replace = evaluate_router_operation(&install.registry, &replace_input).expect("replace");
        assert_eq!(replace.outcome, "replaced");

        let mut remove_input = router_input("remove", ROUTER_GENERATION_TWO);
        remove_input.prior_generation = Some(ROUTER_GENERATION_TWO);
        remove_input.shutdown_evidence_ref = Some(fixture_ref("shutdown-two"));
        let remove = evaluate_router_operation(&replace.registry, &remove_input).expect("remove");
        assert_eq!(remove.outcome, "removed");
        assert!(remove.registry.handlers.is_empty());

        let unsupported = evaluate_router_operation(&remove.registry, &RouterOperationInput {
            operation: "unsupported-alpn".to_string(),
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            ..router_input("unsupported-alpn", ROUTER_GENERATION_ONE)
        })
        .expect("unsupported");
        assert_eq!(unsupported.decision, "deny");
        assert!(unsupported.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported ALPN")));
    }

    #[test]
    fn stale_router_generation_denies_without_mutation() {
        let registry = installed_registry();
        let mut replace_input = router_input("replace", ROUTER_GENERATION_TWO);
        replace_input.prior_generation = Some(ROUTER_GENERATION_TWO);
        replace_input.shutdown_evidence_ref = Some(fixture_ref("shutdown"));
        let denied = evaluate_router_operation(&registry, &replace_input).expect("stale deny");
        assert_eq!(denied.decision, "deny");
        assert_eq!(denied.registry, registry);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("stale-generation")));
    }

    fn framed_input(registry: &ProtocolRegistry) -> FramedEnvelopeInput {
        let envelope =
            crate::preserves_rail::record("node-control-envelope", vec![crate::preserves_rail::string("status")]);
        let bytes = crate::preserves_rail::canonical_bytes(&envelope).expect("bytes");
        let declared = crate::preserves_rail::content_ref_from_bytes(&bytes);
        assert!(registry.handlers.contains_key("molten/node-control/1"));
        FramedEnvelopeInput {
            alpn: "molten/node-control/1".to_string(),
            peer: "peer-a".to_string(),
            node: "node-b".to_string(),
            stream_id: "stream-1".to_string(),
            sequence: VALID_FRAME_SEQUENCE,
            declared_length: bytes.len() as u64,
            declared_envelope_ref: declared,
            envelope_bytes: bytes,
            limit_profile_ref: default_limit_profile_ref(),
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
            limits: FramedEnvelopeLimits {
                max_frame_bytes: VALID_FRAME_LIMIT,
                max_frames_per_session: MAX_SESSION_FRAMES,
                max_outstanding_frames: MAX_SESSION_FRAMES,
            },
        }
    }

    #[test]
    fn framed_envelope_passes_for_canonical_preserves_and_denies_bad_refs() {
        let registry = installed_registry();
        let input = framed_input(&registry);
        let pass = evaluate_framed_envelope(&registry, &input).expect("frame pass");
        assert_eq!(pass.decision, "pass");
        assert_eq!(pass.actual_envelope_ref.as_ref(), Some(&input.declared_envelope_ref));

        let bad = FramedEnvelopeInput {
            declared_envelope_ref: fixture_ref("wrong"),
            ..input
        };
        let deny = evaluate_framed_envelope(&registry, &bad).expect("frame deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("mismatch")));
    }

    #[test]
    fn framed_envelope_denies_oversized_before_payload_parse() {
        let registry = installed_registry();
        let input = FramedEnvelopeInput {
            declared_length: OVERSIZED_FRAME_LENGTH,
            limits: FramedEnvelopeLimits {
                max_frame_bytes: VALID_FRAME_LIMIT,
                max_frames_per_session: MAX_SESSION_FRAMES,
                max_outstanding_frames: MAX_SESSION_FRAMES,
            },
            envelope_bytes: b"not preserves".to_vec(),
            ..framed_input(&registry)
        };
        let deny = evaluate_framed_envelope(&registry, &input).expect("oversized deny");
        assert_eq!(deny.decision, "deny");
        assert!(deny.diagnostics.iter().any(|diagnostic| diagnostic.contains("oversized frame")));
        assert!(deny.actual_envelope_ref.is_none());
    }

    #[test]
    fn service_session_uses_same_model_for_local_and_remote() {
        let local = ServiceSessionInput {
            service_id: "node-control".to_string(),
            operation_id: "status".to_string(),
            interaction_kind: "unary".to_string(),
            path_kind: "local".to_string(),
            request_ref: fixture_ref("request"),
            response_refs: refs(),
            capability_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            alpn: None,
            peer: None,
            node: None,
            stream_id: None,
            frame_receipt_refs: Vec::new(),
        };
        let local_decision = evaluate_service_session(&local).expect("local");
        assert_eq!(local_decision.decision, "pass");

        let remote = ServiceSessionInput {
            path_kind: "remote".to_string(),
            alpn: Some("molten/node-control/1".to_string()),
            peer: Some("peer-a".to_string()),
            node: Some("node-b".to_string()),
            stream_id: Some("stream-1".to_string()),
            frame_receipt_refs: refs(),
            ..local
        };
        let remote_decision = evaluate_service_session(&remote).expect("remote");
        assert_eq!(remote_decision.decision, "pass");
        let text = crate::preserves_rail::to_text(&remote_decision.receipt_value).expect("text");
        assert!(text.contains("postcard-not-canonical-boundary"));
    }

    #[test]
    fn diagnostics_reports_live_only_observations_as_degraded() {
        let report = network_diagnostics_report(&NetworkDiagnosticsInput {
            nat_class: "cone".to_string(),
            udp_status: "pass".to_string(),
            direct_path_status: "pass".to_string(),
            relay_latency_ms: Some(RELAY_LATENCY_MS),
            port_map_protocols: vec!["pcp".to_string()],
            interface_refs: refs(),
            route_refs: refs(),
            live_observations_recorded: false,
            diagnostics: Vec::new(),
        })
        .expect("report");
        assert_eq!(report.decision, "degraded");
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.contains("non-replayable")));
    }

    #[test]
    fn connectivity_probe_reports_relay_only_as_degraded_and_identity_mismatch_as_deny() {
        let probe = ConnectivityProbeInput {
            source_node: "node-a".to_string(),
            target_node: "node-b".to_string(),
            expected_endpoint_ref: fixture_ref("endpoint-a"),
            observed_endpoint_ref: Some(fixture_ref("endpoint-a")),
            direct_path_status: "deny".to_string(),
            relay_path_status: "pass".to_string(),
            timeout_ms: None,
            authority_refs: refs(),
            policy_refs: refs(),
            resource_refs: refs(),
            evidence_refs: refs(),
        };
        let degraded = connectivity_probe_receipt(&probe).expect("relay-only");
        assert_eq!(degraded.decision, "degraded");

        let mismatch = ConnectivityProbeInput {
            observed_endpoint_ref: Some(fixture_ref("other")),
            ..probe
        };
        let deny = connectivity_probe_receipt(&mismatch).expect("identity deny");
        assert_eq!(deny.decision, "deny");
    }

    #[test]
    fn port_mapping_denies_mutation_without_evidence_and_probe_does_not_mutate() {
        let deny = port_mapping_receipt(&PortMappingInput {
            mode: "mutate".to_string(),
            requester_ref: None,
            identity_ref: None,
            protocol: "pcp".to_string(),
            external_port: Some(EXTERNAL_PORT),
            internal_port: Some(INTERNAL_PORT),
            duration_seconds: Some(PORT_DURATION_SECONDS),
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            operator_evidence_refs: Vec::new(),
            available_protocols: vec!["pcp".to_string()],
        })
        .expect("mutation deny");
        assert_eq!(deny.decision, "deny");

        let probe = port_mapping_receipt(&PortMappingInput {
            mode: "probe".to_string(),
            requester_ref: None,
            identity_ref: None,
            protocol: "pcp".to_string(),
            external_port: None,
            internal_port: None,
            duration_seconds: None,
            authority_refs: Vec::new(),
            policy_refs: Vec::new(),
            resource_refs: Vec::new(),
            operator_evidence_refs: Vec::new(),
            available_protocols: vec!["pcp".to_string()],
        })
        .expect("probe pass");
        assert_eq!(probe.decision, "pass");
    }

    #[test]
    fn watcher_snapshot_keeps_latest_state_bounded() {
        let snapshot = watcher_snapshot_value(&NetworkWatcherInput {
            node: "node-a".to_string(),
            interface_state: "eth0-up".to_string(),
            address_state: "ipv6-ready".to_string(),
            default_route: "via-relay".to_string(),
            relay_state: "online".to_string(),
            endpoint_state: "listening".to_string(),
            observed_event_count: WATCHER_OBSERVED_EVENTS,
            retained_event_count: WATCHER_RETAINED_EVENTS,
            evidence_refs: refs(),
        })
        .expect("watcher");
        assert_eq!(snapshot.decision, "pass");
    }
