
    #[tokio::test]
    async fn control_live_iroh_loopback_delivers_to_durable_inbox() {
        let root = crate::test_support::process_workspace("node_control_live_iroh")
            .expect("create isolated async node workspace");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ingress").expect("policy ref")];
        let authority_refs = test_live_authority_refs(&root, "peer:live", "node:live-ingress", "status", &policy_refs)
            .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-ingress").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:live", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");

        let live = control_live_iroh_loopback(&ControlLiveLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:live",
            to_node: "node:live-ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .await
        .expect("live loopback");
        assert!(live.has_enqueued);
        assert_eq!(crate::ledger::artifact_kind(&live.publish_receipt_value), "node-control-live-transport-receipt");
        assert_eq!(crate::ledger::artifact_kind(&live.receive_receipt_value), "node-control-live-transport-receipt");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("serve live ingress");
        assert_eq!(served.decision, "pass");
        assert_eq!(served.processed_request_refs.len(), 1);
    }

    #[test]
    fn control_service_delivers_ingress_and_dispatches_through_loop() {
        let root = temp_dir("node-control-service-ingress");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-ingress",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let authority_refs = vec![local_ref("node-control-authority", "service-ingress").expect("authority ref")];
        let policy_refs = vec![local_ref("node-control-policy", "service-ingress").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "service-ingress").expect("resource ref")];
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:service").expect("bootstrap ref")];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:service",
            to_node: "node:service-ingress",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 4,
            supervisor_policy_value: None,
        })
        .expect("serve ingress");
        assert_eq!(served.decision, "pass");
        assert_eq!(served.heartbeat_receipt_refs.len(), 1);
        assert_eq!(served.ingress_receipt_refs.len(), 1);
        assert_eq!(served.loop_receipt_refs.len(), 1);
        assert_eq!(served.processed_request_refs.len(), 1);
        assert_eq!(crate::ledger::artifact_kind(&served.service_receipt_value), "node-control-service-run-receipt");
        let control_value = read_preserves(&control_outbox_receipt_path(&root, &served.processed_request_refs[0]))
            .expect("read served control receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse served control");
        assert_eq!(control.decision, "pass");
    }

    #[test]
    fn control_service_duplicate_lock_denies_before_side_effects() {
        let root = temp_dir("node-control-service-duplicate");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-duplicate",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let startup = current_startup_receipt(&root).expect("startup");
        let identity =
            crate::node_identity::parse_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", "already-active").expect("service run ref");
        let lock_value = service_lock_value(&ServiceLockValueInput {
            state_root: &root,
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("service lock");
        write_preserves(&root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value).expect("write service lock");
        let request = status_request().expect("status request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &request.value,
        })
        .expect("submit pending request");

        let served = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("duplicate service denial");
        assert_eq!(served.decision, "deny");
        assert_eq!(served.ticks, 0);
        assert!(served.processed_request_refs.is_empty());
        assert!(next_pending_control_request(&root).expect("pending scan").is_some());
        let text = crate::preserves_rail::to_text(&served.service_receipt_value).expect("service receipt text");
        assert!(text.contains("already active"));
    }

    #[test]
    fn control_supervisor_policy_recovers_stale_lock_and_bounds_shutdown() {
        let root = initialized_control_root("node-control-supervisor-policy", "node:supervisor-policy");
        write_active_service_lock(&root, "stale");
        let policy_refs = vec![local_ref("node-control-supervisor-policy", "recover").expect("policy ref")];
        let recover_policy = recovering_policy(&policy_refs);

        let recovered = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("recover stale lock");
        assert_eq!(recovered.decision, "pass");
        assert_eq!(recovered.supervisor_receipt_refs.len(), 2);
        assert!(recovered.supervisor_policy_ref.is_some());
        assert!(!root.join(CONTROL_SERVICE_LOCK_FILE).exists());
        let restart_once = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("allowed restart");
        assert_eq!(restart_once.decision, "pass");
        let restart_denied = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&recover_policy),
        })
        .expect("bounded restart denial");
        assert_eq!(restart_denied.decision, "deny");
        assert_eq!(restart_denied.ticks, 0);
        let restart_denied_text =
            crate::preserves_rail::to_text(&restart_denied.service_receipt_value).expect("restart denial receipt text");
        assert!(restart_denied_text.contains("restart attempts"));

        let shutdown = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let tight_policy = bounded_shutdown_policy(&policy_refs);
        let stopped = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 4,
            max_requests_per_tick: 1,
            supervisor_policy_value: Some(&tight_policy),
        })
        .expect("shutdown serve");
        assert_eq!(stopped.decision, "deny");
        assert!(stopped.has_stopped);
        assert_eq!(stopped.supervisor_receipt_refs.len(), 2);
        let text = crate::preserves_rail::to_text(&stopped.service_receipt_value).expect("service receipt text");
        assert!(text.contains("exceeded supervisor bound"));
    }

    fn write_active_service_lock(root: &Path, service_suffix: &str) {
        let startup = current_startup_receipt(root).expect("startup");
        let identity =
            crate::node_identity::parse_identity(&read_preserves(&root.join(IDENTITY_FILE)).expect("identity"))
                .expect("parse identity");
        let service_run_ref = local_ref("node-control-service-run", service_suffix).expect("service run ref");
        let lock_value = service_lock_value(&ServiceLockValueInput {
            state_root: root,
            startup_receipt_ref: &startup.receipt_ref,
            node_id: &identity.node_id,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 1,
            max_requests_per_tick: 1,
            service_run_ref: &service_run_ref,
        })
        .expect("service lock");
        write_preserves(&root.join(CONTROL_SERVICE_LOCK_FILE), &lock_value).expect("write service lock");
    }

    fn recovering_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 1,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 1,
            stale_lock_recovery: true,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("recover policy")
    }

    fn bounded_shutdown_policy(policy_refs: &[String]) -> IoValue {
        control_supervisor_policy_value(&ControlSupervisorPolicyInput {
            max_restarts: 0,
            restart_window_ticks: 1,
            heartbeat_timeout_ticks: 1,
            shutdown_drain_ticks: 0,
            stale_lock_recovery: false,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("tight policy")
    }
