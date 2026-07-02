    type TestCase = hegel::TestCase;

    use super::*;

    type ListInput = crate::catalog::ListInput;
    type VisibilityInput = crate::catalog::VisibilityInput;

    fn content_ref_from_bytes(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn parse_text(source: &str) -> Result<IoValue> {
        crate::preserves_rail::parse_text(source)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn test_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn manifest_input() -> ServiceManifestInput {
        ServiceManifestInput {
            service_id: "svc:web".to_string(),
            owner_authority_ref: test_ref("authority"),
            target_ref: test_ref("target"),
            dependencies: vec!["svc:db".to_string()],
            provided_assertion_refs: vec![test_ref("provided")],
            restart_policy_ref: test_ref("restart"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            effect_profile_refs: vec![test_ref("effect")],
        }
    }

    #[test]
    fn service_manifest_roundtrips_with_stable_ref() {
        let value = service_manifest_value(&manifest_input()).expect("manifest value");
        let parsed = parse_service_manifest(&value).expect("parse manifest");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(parsed.service_id, "svc:web");
        assert_eq!(parsed.dependencies, vec!["svc:db".to_string()]);
        assert_eq!(parsed.manifest_ref, canonical_hash(&reparsed).expect("hash reparsed manifest"));
    }

    #[test]
    fn service_manifest_requires_explicit_boundaries() {
        let mut input = manifest_input();
        input.policy_refs.clear();
        let error = service_manifest_value(&input).expect_err("missing policy denied");
        assert!(error.to_string().contains("service policy refs"));

        let malformed = parse_text(
            "<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:web\"> \
             <owner \"not-a-ref\"> <target \"not-a-ref\"> <requires []> <provides []> \
             <restart-policy \"not-a-ref\"> <policy []> <resource []> <effect-profile []> \
             <checks [<check \"explicit-authority\" \"pass\"> <check \"policy-resource-effect-declared\" \"pass\">]>>",
        )
        .expect("parse malformed manifest");
        assert!(parse_service_manifest(&malformed).is_err());

        let short_ref = parse_text(
            "<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:web\"> \
             <owner \"blake3:short\"> <target \"blake3:short\"> <requires []> <provides []> \
             <restart-policy \"blake3:short\"> <policy [\"blake3:short\"]> <resource [\"blake3:short\"]> \
             <effect-profile [\"blake3:short\"]> \
             <checks [<check \"explicit-authority\" \"pass\"> <check \"policy-resource-effect-declared\" \"pass\">]>>",
        )
        .expect("parse short-ref manifest");
        let error = parse_service_manifest(&short_ref).expect_err("short refs fail closed");
        assert!(error.to_string().contains("canonical content ref"));
    }

    struct Core {
        manifest: IoValue,
        manifest_ref: String,
        demand: IoValue,
        status: IoValue,
        status_ref: String,
    }

    struct Aux {
        supervisor: IoValue,
        link: IoValue,
        monitor: IoValue,
        monitor_ref: String,
        restart: IoValue,
        restart_ref: String,
    }

    struct Receipts {
        decision: IoValue,
        lifecycle: IoValue,
        cleanup: IoValue,
    }

    struct Case {
        manifest: IoValue,
        demand: IoValue,
        status: IoValue,
        supervisor: IoValue,
        link: IoValue,
        monitor: IoValue,
        restart: IoValue,
        decision: IoValue,
        lifecycle: IoValue,
        cleanup: IoValue,
    }

    fn base() -> Core {
        let manifest = service_manifest_value(&manifest_input()).expect("manifest");
        let manifest_ref = canonical_hash(&manifest).expect("manifest ref");
        let demand = service_demand_value(&ServiceDemandInput {
            demand_id: "demand:web".to_string(),
            service_id: "svc:web".to_string(),
            requester_ref: test_ref("requester"),
            manifest_ref: Some(manifest_ref.clone()),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("demand");
        let demand_ref = canonical_hash(&demand).expect("demand ref");
        let status = service_status_value(&ServiceStatusInput {
            service_id: "svc:web".to_string(),
            state: "ready".to_string(),
            manifest_ref: Some(manifest_ref.clone()),
            demand_refs: vec![demand_ref.clone()],
            dependency_status_refs: vec![test_ref("dep-status")],
            readiness_assertion_refs: vec![test_ref("ready")],
            failure_refs: Vec::new(),
            restart_count: 0,
            monitor_refs: Vec::new(),
            replay_refs: vec![test_ref("replay")],
        })
        .expect("status");
        let status_ref = canonical_hash(&status).expect("status ref");
        Core {
            manifest,
            manifest_ref,
            demand,
            status,
            status_ref,
        }
    }

    fn aux() -> Aux {
        let supervisor = service_supervisor_value(&ServiceSupervisorInput {
            supervisor_id: "supervisor:web".to_string(),
            service_ids: vec!["svc:web".to_string()],
            link_refs: vec![test_ref("link")],
            monitor_refs: vec![test_ref("monitor")],
            policy_refs: vec![test_ref("policy")],
        })
        .expect("supervisor");
        let link = service_link_value(&ServiceLinkInput {
            supervisor_id: "supervisor:web".to_string(),
            parent_service_id: "svc:web".to_string(),
            child_service_id: "svc:web".to_string(),
            propagation: "restart".to_string(),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("link");
        let monitor = service_monitor_value(&ServiceMonitorInput {
            monitor_id: "monitor:web".to_string(),
            service_id: "svc:web".to_string(),
            observer_ref: test_ref("observer"),
            notification_policy: "failure".to_string(),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("monitor");
        let monitor_ref = canonical_hash(&monitor).expect("monitor ref");
        let restart = service_restart_policy_value(&ServiceRestartPolicyInput {
            policy_id: "restart:web".to_string(),
            max_attempts: 2,
            window_steps: 10,
            backoff_steps: 1,
            resource_refs: vec![test_ref("resource")],
        })
        .expect("restart policy");
        let restart_ref = canonical_hash(&restart).expect("restart policy ref");
        Aux {
            supervisor,
            link,
            monitor,
            monitor_ref,
            restart,
            restart_ref,
        }
    }

    fn decision(core: &Core, aux: &Aux) -> IoValue {
        service_restart_decision_value(&ServiceRestartDecisionInput {
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            policy_ref: aux.restart_ref.clone(),
            attempt: 1,
            max_attempts: 2,
            window_step: 0,
            backoff_slot: 0,
            prior_lifecycle_refs: vec![test_ref("prior")],
            authority_refs: vec![test_ref("authority")],
            resource_refs: vec![test_ref("resource")],
            diagnostics: Vec::new(),
        })
        .expect("restart decision")
    }

    fn lifecycle(core: &Core, aux: &Aux) -> IoValue {
        service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
            operation: "ready".to_string(),
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            status_ref: Some(core.status_ref.clone()),
            authority_refs: vec![test_ref("authority-receipt")],
            resource_refs: vec![test_ref("resource-receipt")],
            effect_profile_refs: vec![test_ref("effect")],
            supervision_refs: vec![aux.monitor_ref.clone()],
            diagnostics: Vec::new(),
        })
        .expect("lifecycle")
    }

    fn cleanup(core: &Core) -> IoValue {
        service_cleanup_receipt_value(&ServiceCleanupReceiptInput {
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            authority_refs: vec![test_ref("authority")],
            owned_assertion_refs: vec![test_ref("owned")],
            observer_refs: vec![test_ref("observer")],
            live_ref_refs: vec![test_ref("live")],
            exposed_ref_refs: vec![test_ref("exposed")],
            pending_effect_refs: vec![test_ref("effect")],
            retraction_refs: vec![test_ref("retraction")],
            revocation_refs: vec![test_ref("revocation")],
            retention_refs: vec![test_ref("retention")],
            diagnostics: Vec::new(),
        })
        .expect("cleanup")
    }

    fn receipts(core: &Core, aux: &Aux) -> Receipts {
        Receipts {
            decision: decision(core, aux),
            lifecycle: lifecycle(core, aux),
            cleanup: cleanup(core),
        }
    }

    fn case() -> Case {
        let core = base();
        let aux = aux();
        let receipts = receipts(&core, &aux);
        Case {
            manifest: core.manifest,
            demand: core.demand,
            status: core.status,
            supervisor: aux.supervisor,
            link: aux.link,
            monitor: aux.monitor,
            restart: aux.restart,
            decision: receipts.decision,
            lifecycle: receipts.lifecycle,
            cleanup: receipts.cleanup,
        }
    }

    fn assert_variants(case: &Case) {
        assert!(matches!(parse_service_record(&case.manifest).expect("manifest record"), ServiceRecord::Manifest(_)));
        assert!(matches!(parse_service_record(&case.demand).expect("demand record"), ServiceRecord::Demand(_)));
        assert!(matches!(parse_service_record(&case.status).expect("status record"), ServiceRecord::Status(_)));
        assert!(matches!(
            parse_service_record(&case.supervisor).expect("supervisor record"),
            ServiceRecord::Supervisor(_)
        ));
        assert!(matches!(parse_service_record(&case.link).expect("link record"), ServiceRecord::Link(_)));
        assert!(matches!(parse_service_record(&case.monitor).expect("monitor record"), ServiceRecord::Monitor(_)));
        assert!(matches!(
            parse_service_record(&case.restart).expect("restart record"),
            ServiceRecord::RestartPolicy(_)
        ));
        assert!(matches!(
            parse_service_record(&case.decision).expect("restart decision record"),
            ServiceRecord::RestartDecision(_)
        ));
        assert!(matches!(
            parse_service_record(&case.lifecycle).expect("lifecycle record"),
            ServiceRecord::LifecycleReceipt(_)
        ));
        assert!(matches!(
            parse_service_record(&case.cleanup).expect("cleanup record"),
            ServiceRecord::CleanupReceipt(_)
        ));
    }
