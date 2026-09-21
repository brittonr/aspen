
    fn admission_request(operation: &str, authority: &[String], policy: &[String], resource: &[String]) -> ControlRequest {
        let request_value = control_request_value(&ControlRequestValueInput {
            operation,
            target_ref: None,
            payload_ref: None,
            authority_refs: authority,
            policy_refs: policy,
            resource_refs: resource,
            evidence_refs: &[],
        })
        .expect("control request");
        parse_control_request(&request_value).expect("parse control request")
    }

    fn admission_adapters() -> Vec<NodeAdapterReceiptRef> {
        REQUIRED_RUNTIME_ADAPTERS
            .iter()
            .map(|name| NodeAdapterReceiptRef {
                name: (*name).to_string(),
                receipt_ref: test_ref(&format!("{name}-start")),
            })
            .collect()
    }

    #[test]
    fn shutdown_admission_pass_builds_reverse_order_plan() {
        // r[verify molten.audit_f01.admission]
        // r[verify molten.audit_f01.observed_effects]
        let request = admission_request(
            "shutdown",
            &[test_ref("authority")],
            &[test_ref("policy")],
            &[test_ref("resource")],
        );
        let adapters = admission_adapters();
        let startup_ref = test_ref("startup");
        let admission = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: &startup_ref,
            adapter_receipts: &adapters,
            has_active_lock: true,
        })
        .expect("admit shutdown");

        assert_eq!(admission.decision, "pass");
        assert!(admission.diagnostics.is_empty());
        let plan = admission.plan.expect("admitted plan");
        assert_eq!(plan.startup_receipt_ref, startup_ref);
        let mut expected = adapters.clone();
        expected.reverse();
        assert_eq!(plan.adapter_receipts, expected);
    }

    struct AdmissionDenialCase {
        operation: &'static str,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        expected_diagnostic: &'static str,
    }

    fn admission_denial_cases() -> [AdmissionDenialCase; 4] {
        [
            AdmissionDenialCase {
                operation: "shutdown",
                authority_refs: Vec::new(),
                policy_refs: vec![test_ref("policy")],
                resource_refs: vec![test_ref("resource")],
                expected_diagnostic: "authority refs missing",
            },
            AdmissionDenialCase {
                operation: "shutdown",
                authority_refs: vec![test_ref("authority")],
                policy_refs: Vec::new(),
                resource_refs: vec![test_ref("resource")],
                expected_diagnostic: "policy refs missing",
            },
            AdmissionDenialCase {
                operation: "shutdown",
                authority_refs: vec![test_ref("authority")],
                policy_refs: vec![test_ref("policy")],
                resource_refs: Vec::new(),
                expected_diagnostic: "resource refs missing",
            },
            AdmissionDenialCase {
                operation: "status",
                authority_refs: vec![test_ref("authority")],
                policy_refs: vec![test_ref("policy")],
                resource_refs: vec![test_ref("resource")],
                expected_diagnostic: "requires shutdown operation",
            },
        ]
    }

    fn assert_denial_cases() {
        for case in admission_denial_cases() {
            let request =
                admission_request(case.operation, &case.authority_refs, &case.policy_refs, &case.resource_refs);
            let admission = admit_node_shutdown(&ShutdownAdmissionInput {
                request: &request,
                startup_receipt_ref: &test_ref("startup"),
                adapter_receipts: &admission_adapters(),
                has_active_lock: true,
            })
            .expect("admit shutdown");
            assert_eq!(admission.decision, "deny", "operation {} must deny", case.operation);
            assert!(admission.plan.is_none(), "denied admission cannot carry a plan");
            assert!(
                admission
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains(case.expected_diagnostic)),
                "expected diagnostic {} in {:?}",
                case.expected_diagnostic,
                admission.diagnostics
            );
        }
    }

    fn assert_lock_and_adapter_denials() {
        let request = admission_request(
            "shutdown",
            &[test_ref("authority")],
            &[test_ref("policy")],
            &[test_ref("resource")],
        );
        let missing_lock = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: &test_ref("startup"),
            adapter_receipts: &admission_adapters(),
            has_active_lock: false,
        })
        .expect("admit shutdown");
        assert_eq!(missing_lock.decision, "deny");
        assert!(missing_lock.plan.is_none());
        assert!(missing_lock.diagnostics.iter().any(|diagnostic| diagnostic.contains("active node lock")));

        let missing_adapters = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: &test_ref("startup"),
            adapter_receipts: &[],
            has_active_lock: true,
        })
        .expect("admit shutdown");
        assert_eq!(missing_adapters.decision, "deny");
        assert!(missing_adapters.plan.is_none());
        assert!(missing_adapters
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("startup adapter evidence")));
    }

    #[test]
    fn shutdown_admission_denies_missing_evidence_without_plan() {
        // r[verify molten.audit_f01.admission]
        // r[verify molten.audit_f01.validation]
        assert_denial_cases();
        assert_lock_and_adapter_denials();
    }

    #[test]
    fn shutdown_admission_denies_malformed_bindings_and_startup_refs() {
        // r[verify molten.audit_f01.admission]
        // r[verify molten.audit_f01.validation]
        let request = admission_request(
            "shutdown",
            &[test_ref("authority")],
            &[test_ref("policy")],
            &[test_ref("resource")],
        );
        let malformed_name = vec![NodeAdapterReceiptRef {
            name: "not/an/adapter".to_string(),
            receipt_ref: test_ref("ledger-start"),
        }];
        let denied = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: &test_ref("startup"),
            adapter_receipts: &malformed_name,
            has_active_lock: true,
        })
        .expect("admit shutdown");
        assert_eq!(denied.decision, "deny");
        assert!(denied.plan.is_none());
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("binding invalid")));

        let malformed_ref = vec![NodeAdapterReceiptRef {
            name: "ledger".to_string(),
            receipt_ref: "blake3:short".to_string(),
        }];
        let denied_ref = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: &test_ref("startup"),
            adapter_receipts: &malformed_ref,
            has_active_lock: true,
        })
        .expect("admit shutdown");
        assert_eq!(denied_ref.decision, "deny");
        assert!(denied_ref.diagnostics.iter().any(|diagnostic| diagnostic.contains("receipt ref invalid")));

        let denied_startup = admit_node_shutdown(&ShutdownAdmissionInput {
            request: &request,
            startup_receipt_ref: "blake3:short",
            adapter_receipts: &admission_adapters(),
            has_active_lock: true,
        })
        .expect("admit shutdown");
        assert_eq!(denied_startup.decision, "deny");
        assert!(denied_startup.diagnostics.iter().any(|diagnostic| diagnostic.contains("startup receipt ref invalid")));
    }
