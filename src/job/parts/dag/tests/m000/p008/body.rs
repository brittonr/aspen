
    struct RemoteExecutionFixture {
        root_ref: String,
        dependency_ref: String,
        extra_ref: String,
        effect_ref: String,
        capability_ref: String,
        policy_ref: String,
        provenance_ref: String,
        source_gate_ref: String,
        resource_ref: String,
        reply_route_ref: String,
        handler_admission_ref: String,
        descriptor_value: IoValue,
        request_value: IoValue,
        plan: RemoteExecutionClosurePlan,
    }

    fn remote_execution_fixture(label: &str) -> RemoteExecutionFixture {
        let root_ref = test_ref(&format!("remote-root-{label}"));
        let dependency_ref = test_ref(&format!("remote-dependency-{label}"));
        let effect_ref = test_ref(&format!("remote-effect-{label}"));
        let capability_ref = test_ref(&format!("remote-capability-{label}"));
        let policy_ref = test_ref(&format!("remote-policy-{label}"));
        let provenance_ref = test_ref(&format!("remote-provenance-{label}"));
        let source_gate_ref = test_ref(&format!("remote-source-gate-{label}"));
        let resource_ref = test_ref(&format!("remote-resource-{label}"));
        let reply_route_ref = test_ref(&format!("remote-reply-{label}"));
        let handler_admission_ref = test_ref(&format!("remote-handler-admission-{label}"));
        let extra_ref = test_ref(&format!("remote-extra-{label}"));
        let descriptor_value = remote_execution_closure_descriptor_value(&RemoteExecutionClosureDescriptorInput {
            root_artifact_ref: root_ref.clone(),
            dependency_refs: vec![root_ref.clone(), dependency_ref.clone()],
            closure_digest_ref: Some(test_ref(&format!("remote-closure-digest-{label}"))),
            artifact_kind: "stage".to_string(),
            size_bound_ref: test_ref(&format!("remote-size-bound-{label}")),
            effect_manifest_ref: effect_ref.clone(),
            handler_profile: "local-echo-v1".to_string(),
            policy_refs: vec![policy_ref.clone()],
            evidence_refs: vec![provenance_ref.clone()],
            replay_nonce_ref: test_ref(&format!("remote-replay-nonce-{label}")),
        })
        .expect("remote descriptor");
        let request_value = remote_execution_request_value(&RemoteExecutionRequestInput {
            execution_id: format!("remote-execution-{label}"),
            root_artifact_ref: root_ref.clone(),
            closure_descriptor: descriptor_value.clone(),
            entrypoint_id: "main".to_string(),
            argument: crate::preserves_rail::record("argument-ref", vec![crate::preserves_rail::string(test_ref(
                &format!("remote-argument-{label}"),
            ))]),
            effect_manifest_ref: effect_ref.clone(),
            handler_profile: "local-echo-v1".to_string(),
            capability_refs: vec![capability_ref.clone()],
            policy_refs: vec![policy_ref.clone()],
            provenance_refs: vec![provenance_ref.clone()],
            source_gate_refs: vec![source_gate_ref.clone()],
            resource_refs: vec![resource_ref.clone()],
            reply_route_ref: reply_route_ref.clone(),
            evidence_refs: vec![source_gate_ref.clone(), resource_ref.clone()],
        })
        .expect("remote request");
        let plan = plan_remote_execution_closure(RemoteExecutionClosurePlanInput {
            closure_descriptor: descriptor_value.clone(),
            receiver_present_refs: vec![root_ref.clone()],
            sender_payload_refs: vec![dependency_ref.clone()],
        })
        .expect("remote closure plan");
        RemoteExecutionFixture {
            root_ref,
            dependency_ref,
            extra_ref,
            effect_ref,
            capability_ref,
            policy_ref,
            provenance_ref,
            source_gate_ref,
            resource_ref,
            reply_route_ref,
            handler_admission_ref,
            descriptor_value,
            request_value,
            plan,
        }
    }

    fn admitted_remote_execution(fixture: &RemoteExecutionFixture) -> RemoteExecutionAdmissionReceipt {
        admit_remote_execution(RemoteExecutionAdmissionInput {
            request: fixture.request_value.clone(),
            closure_plan: fixture.plan.value.clone(),
            fetched_refs: vec![fixture.dependency_ref.clone()],
            verified_artifact_refs: vec![fixture.dependency_ref.clone()],
            admitted_capability_refs: vec![fixture.capability_ref.clone()],
            handler_profile_admission_ref: fixture.handler_admission_ref.clone(),
            local_policy_refs: vec![fixture.policy_ref.clone()],
            provenance_receipt_refs: vec![fixture.provenance_ref.clone()],
            source_gate_receipt_refs: vec![fixture.source_gate_ref.clone()],
            resource_receipt_refs: vec![fixture.resource_ref.clone()],
            evidence_refs: vec![fixture.provenance_ref.clone(), fixture.source_gate_ref.clone()],
        })
        .expect("remote admission")
    }

    #[test]
    fn remote_execution_ref_request_plans_and_admits_verified_closure() {
        let fixture = remote_execution_fixture("pass");
        let parsed_request = parse_remote_execution_request(&fixture.request_value).expect("parse request");
        assert_eq!(parsed_request.root_artifact_ref, fixture.root_ref);
        assert_eq!(parsed_request.effect_manifest_ref, fixture.effect_ref);
        assert_eq!(parsed_request.reply_route_ref, fixture.reply_route_ref);
        assert_eq!(fixture.plan.missing_refs, vec![fixture.dependency_ref.clone()]);
        assert!(fixture.plan.sender_extra_refs.is_empty());

        let admitted = admitted_remote_execution(&fixture);
        assert_eq!(admitted.decision, "pass", "{:?}", admitted.diagnostics);
        assert_eq!(admitted.root_artifact_ref, fixture.root_ref);
        assert_eq!(admitted.fetched_refs, vec![fixture.dependency_ref.clone()]);
        let parsed = parse_remote_execution_admission_receipt(&admitted.value).expect("parse admission");
        assert_eq!(parsed.decision, "pass");
        let receipt_text = crate::preserves_rail::to_text(&admitted.value).expect("admission text");
        assert!(receipt_text.contains("receiver-closure-complete"));
        assert!(receipt_text.contains("handler-profile-admission-bound"));
    }

    #[test]
    fn remote_execution_request_rejects_mobile_payloads_and_profile_mismatch() {
        let fixture = remote_execution_fixture("request-deny");
        let mobile = remote_execution_request_value(&RemoteExecutionRequestInput {
            execution_id: "remote-mobile".to_string(),
            root_artifact_ref: fixture.root_ref.clone(),
            closure_descriptor: fixture.descriptor_value.clone(),
            entrypoint_id: "main".to_string(),
            argument: crate::preserves_rail::record("raw-closure", vec![crate::preserves_rail::string("heap")]),
            effect_manifest_ref: fixture.effect_ref.clone(),
            handler_profile: "local-echo-v1".to_string(),
            capability_refs: vec![fixture.capability_ref.clone()],
            policy_refs: vec![fixture.policy_ref.clone()],
            provenance_refs: vec![fixture.provenance_ref.clone()],
            source_gate_refs: vec![fixture.source_gate_ref.clone()],
            resource_refs: vec![fixture.resource_ref.clone()],
            reply_route_ref: fixture.reply_route_ref.clone(),
            evidence_refs: vec![fixture.source_gate_ref.clone()],
        })
        .expect_err("mobile closure payload denies");
        assert!(mobile.to_string().contains("mobile/ambient"));

        let mismatch = remote_execution_request_value(&RemoteExecutionRequestInput {
            execution_id: "remote-mismatch".to_string(),
            root_artifact_ref: fixture.root_ref.clone(),
            closure_descriptor: fixture.descriptor_value.clone(),
            entrypoint_id: "main".to_string(),
            argument: crate::preserves_rail::string("ok"),
            effect_manifest_ref: fixture.effect_ref.clone(),
            handler_profile: "wrong-profile".to_string(),
            capability_refs: vec![fixture.capability_ref.clone()],
            policy_refs: vec![fixture.policy_ref.clone()],
            provenance_refs: vec![fixture.provenance_ref.clone()],
            source_gate_refs: vec![fixture.source_gate_ref.clone()],
            resource_refs: vec![fixture.resource_ref.clone()],
            reply_route_ref: fixture.reply_route_ref.clone(),
            evidence_refs: vec![fixture.source_gate_ref.clone()],
        })
        .expect_err("handler mismatch denies");
        assert!(mismatch.to_string().contains("handler profile"));
    }

    #[test]
    fn remote_execution_admission_denies_incomplete_unverified_and_unselected_refs() {
        let fixture = remote_execution_fixture("closure-deny");
        let missing = admit_remote_execution(RemoteExecutionAdmissionInput {
            request: fixture.request_value.clone(),
            closure_plan: fixture.plan.value.clone(),
            fetched_refs: Vec::new(),
            verified_artifact_refs: Vec::new(),
            admitted_capability_refs: vec![fixture.capability_ref.clone()],
            handler_profile_admission_ref: fixture.handler_admission_ref.clone(),
            local_policy_refs: vec![fixture.policy_ref.clone()],
            provenance_receipt_refs: vec![fixture.provenance_ref.clone()],
            source_gate_receipt_refs: vec![fixture.source_gate_ref.clone()],
            resource_receipt_refs: vec![fixture.resource_ref.clone()],
            evidence_refs: vec![fixture.provenance_ref.clone()],
        })
        .expect("missing dep receipt");
        assert_eq!(missing.decision, "deny");
        assert!(missing.diagnostics.iter().any(|diagnostic| diagnostic.contains("not fetched")));

        let unverified = admit_remote_execution(RemoteExecutionAdmissionInput {
            request: fixture.request_value.clone(),
            closure_plan: fixture.plan.value.clone(),
            fetched_refs: vec![fixture.dependency_ref.clone()],
            verified_artifact_refs: Vec::new(),
            admitted_capability_refs: vec![fixture.capability_ref.clone()],
            handler_profile_admission_ref: fixture.handler_admission_ref.clone(),
            local_policy_refs: vec![fixture.policy_ref.clone()],
            provenance_receipt_refs: vec![fixture.provenance_ref.clone()],
            source_gate_receipt_refs: vec![fixture.source_gate_ref.clone()],
            resource_receipt_refs: vec![fixture.resource_ref.clone()],
            evidence_refs: vec![fixture.provenance_ref.clone()],
        })
        .expect("unverified dep receipt");
        assert_eq!(unverified.decision, "deny");
        assert!(unverified.diagnostics.iter().any(|diagnostic| diagnostic.contains("hash verification")));

        let extra_plan = plan_remote_execution_closure(RemoteExecutionClosurePlanInput {
            closure_descriptor: fixture.descriptor_value.clone(),
            receiver_present_refs: vec![fixture.root_ref.clone()],
            sender_payload_refs: vec![fixture.dependency_ref.clone(), fixture.extra_ref.clone()],
        })
        .expect("extra plan");
        assert!(extra_plan.diagnostics.iter().any(|diagnostic| diagnostic.contains(&fixture.extra_ref)));
    }

    #[test]
    fn remote_execution_admission_denies_missing_capability_and_local_policy() {
        let fixture = remote_execution_fixture("binding-deny");
        let denied = admit_remote_execution(RemoteExecutionAdmissionInput {
            request: fixture.request_value.clone(),
            closure_plan: fixture.plan.value.clone(),
            fetched_refs: vec![fixture.dependency_ref.clone()],
            verified_artifact_refs: vec![fixture.dependency_ref.clone()],
            admitted_capability_refs: Vec::new(),
            handler_profile_admission_ref: fixture.handler_admission_ref.clone(),
            local_policy_refs: Vec::new(),
            provenance_receipt_refs: Vec::new(),
            source_gate_receipt_refs: Vec::new(),
            resource_receipt_refs: Vec::new(),
            evidence_refs: vec![fixture.provenance_ref.clone()],
        })
        .expect("binding denial receipt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing admitted capability")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("local policy")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("resource")));
    }
