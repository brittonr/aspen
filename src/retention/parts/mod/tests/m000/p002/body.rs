
    #[test]
    fn candidate_bundle_reports_missing_local_artifacts() {
        let root = temp_dir("retention-bundle-missing");
        let missing_ref = fake_ref("missing-plan");
        let object_ref = fake_ref("bundle-object");
        let explain_value = candidate_explain_value(&CandidateExplainValueInput {
            object_ref: &object_ref,
            object_kind: Some("encrypted-ref"),
            retention_class: Some("private-secret-ref"),
            action: Some("delete"),
            subsystem: Some("ledger-gc"),
            pin_refs: &[],
            admission_refs: &[],
            remote_clearance_refs: &[],
            remote_clearance_import_refs: &[],
            gc_plan_refs: std::slice::from_ref(&missing_ref),
            gc_apply_refs: &[],
            gc_execution_refs: &[],
            gc_audit_refs: &[],
            retention_receipt_refs: &[],
            tombstone_refs: &[],
            diagnostics: &[],
        })
        .expect("explain value");
        let bundle = export_candidate_bundle(CandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &root.join("bundle"),
            profile: CandidateBundleExportProfile::Internal,
        })
        .expect("bundle with missing artifact diagnostic");
        assert!(bundle.artifact_refs.is_empty());
        assert_eq!(bundle.diagnostics, vec![format!("retention-bundle-missing-artifact:{missing_ref}")]);
        let verify = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir: &root.join("bundle"),
        })
        .expect("verify missing artifact bundle");
        assert_eq!(verify.decision, "deny");
        assert!(
            verify
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-missing-file:gc-plans"))
        );
    }

    #[test]
    fn candidate_bundle_profiles_deny_or_redact_sensitive_handoff() {
        let root = temp_dir("retention-bundle-profile");
        let object_ref = fake_ref("bundle-profile-object");
        let plan_ref = fake_ref("bundle-profile-plan");
        let explain_value = sensitive_explain_value(&object_ref, &plan_ref);
        let public_dir = root.join("public");
        let public_bundle = export_candidate_bundle(CandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &public_dir,
            profile: CandidateBundleExportProfile::Public,
        })
        .expect("public profile bundle export");
        let public_profile = parse_candidate_bundle_profile(
            &read_store_value(&public_dir.join(BUNDLE_PROFILE_FILE)).expect("read public bundle profile"),
        )
        .expect("parse public profile");
        assert_eq!(public_profile.bundle_ref, public_bundle.bundle_ref);
        assert_eq!(public_profile.profile, "public");
        assert_eq!(public_profile.decision, "deny");
        assert!(!public_profile.marker_refs.is_empty());
        assert!(
            public_profile
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-public-sensitive-markers"))
        );

        let diagnostic_dir = root.join("diagnostic");
        let diagnostic_bundle = export_candidate_bundle(CandidateBundleExportInput {
            root: &root,
            explain_value: &explain_value,
            out: &diagnostic_dir,
            profile: CandidateBundleExportProfile::Diagnostic,
        })
        .expect("diagnostic profile bundle export");
        let diagnostic_profile = parse_candidate_bundle_profile(
            &read_store_value(&diagnostic_dir.join(BUNDLE_PROFILE_FILE)).expect("read diagnostic bundle profile"),
        )
        .expect("parse diagnostic profile");
        assert_eq!(diagnostic_profile.bundle_ref, diagnostic_bundle.bundle_ref);
        assert_eq!(diagnostic_profile.profile, "diagnostic");
        assert_eq!(diagnostic_profile.decision, "pass");
        assert!(!diagnostic_profile.marker_refs.is_empty());
        let redacted_explain = fs::read_to_string(diagnostic_dir.join(BUNDLE_REDACTED_DIR).join("explain.preserves"))
            .expect("read redacted explain");
        assert!(!redacted_explain.contains(CLASS_PRIVATE_SECRET_REF));
        assert!(!redacted_explain.contains("encrypted-ref"));
        let verify = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir: &diagnostic_dir,
        })
        .expect("verify diagnostic source bundle");
        assert_eq!(verify.decision, "deny");
    }

    #[test]
    fn gc_audit_denies_missing_chain_links_without_authority() {
        let root = temp_dir("retention-gc-audit-deny");
        let object_ref = fake_ref("audit-missing-object");
        let execution = store_gc_execution_gate(GcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: None,
        })
        .expect("store denied execution gate");
        let audit = audit_gc_execution(GcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit missing links");
        assert_eq!(audit.decision, "deny");
        assert!(audit.plan_ref.is_none());
        assert!(audit.apply_ref.is_none());
        assert!(audit.retention_receipt_ref.is_none());
        assert!(audit.tombstone_ref.is_none());
        assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-apply-missing"));
        assert!(audit.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-audit-plan-missing"));
    }

    #[test]
    fn gc_apply_from_plan_denies_drift_before_tombstone() {
        let root = temp_dir("retention-gc-apply-drift");
        let fixture = store_passing_plan_fixture(&root, "apply-drift");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "chunk-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store drift plan");
        let pin = pin_object(&root, PinInput {
            object_ref: fixture.object_ref.clone(),
            object_kind: "chunk".to_string(),
            retention_class: CLASS_DURABLE_VALUE.to_string(),
            source: SOURCE_OPERATOR_HOLD.to_string(),
            reason: "operator hold after plan".to_string(),
            owner_ref: fixture.requester_ref.clone(),
            expiry_ref: None,
            policy_refs: fixture.evidence.policy_refs.clone(),
            evidence_refs: fixture.evidence.evidence_refs.clone(),
            has_authority: true,
        })
        .expect("pin after plan");
        assert_eq!(pin.receipt.decision, "pass");
        let receipt_count = store_file_count(&receipts_dir(&root));
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply drift plan");
        assert_eq!(apply.decision, "deny");
        assert!(apply.retention_receipt_ref.is_none());
        assert!(apply.tombstone_ref.is_none());
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-drift"));
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "active-pins-present"));
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
        assert_eq!(store_file_count(&receipts_dir(&root)), receipt_count);
    }

    #[test]
    fn gc_apply_from_denied_plan_writes_only_apply_receipt() {
        let root = temp_dir("retention-gc-apply-denied-plan");
        let fixture = store_passing_plan_fixture(&root, "apply-denied-plan");
        let mut evidence = fixture.evidence;
        evidence.remote_clearance_refs = Vec::new();
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &evidence,
        })
        .expect("store denied plan");
        assert_eq!(plan.decision, "deny");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply denied plan");
        assert_eq!(apply.decision, "deny");
        assert!(apply.retention_receipt_ref.is_none());
        assert!(apply.tombstone_ref.is_none());
        assert!(apply.diagnostics.iter().any(|diagnostic| diagnostic == "retention-gc-apply-plan-not-pass"));
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
    }

    #[test]
    fn destructive_admission_rejects_unreconciled_remote_clearance() {
        let root = temp_dir("retention-admission-remote-deny");
        let case = deny_case(&root);
        let refs = denial_refs(&root, &case);

        let mut partial = case.base();
        partial.remote_clearance_refs = vec![refs.partial];
        assert_denial(&root, &case, &partial, "partial remote denial", &["missing-remote", "missing-peer"]);

        let wrong_peer = case.scoped(refs.wrong_peer);
        assert_denial(&root, &case, &wrong_peer, "wrong peer denial", &["missing-peer"]);

        let stale = case.scoped(refs.stale);
        assert_denial(&root, &case, &stale, "stale remote denial", &["stale", "revoked"]);

        let retained = case.scoped(refs.retained);
        assert_denial(&root, &case, &retained, "retained remote denial", &["retained"]);

        let forged = case.scoped(fake_ref("forged-clearance"));
        assert_denial(&root, &case, &forged, "forged remote denial", &["unreadable"]);
    }

    #[test]
    fn remote_clearance_workflow_imports_peer_clearance_and_denies_wrong_request() {
        let root = temp_dir("retention-remote-clearance-workflow");
        let case = live_case(&root, "workflow");
        let pair = pair_with_label(&root, &case, "workflow-peer-evidence");

        let import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
            root: &root,
            request_value: &pair.request_value,
            response_value: &pair.response_value,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("import clearance");
        assert_eq!(import.decision, "pass");
        let clearance_ref = import.clearance_ref.clone().expect("clearance imported");
        assert_case_pass(&root, &case, clearance_ref);

        let wrong_request = store_remote_gc_clearance_request(&root, &RemoteGcClearanceRequestInput {
            requester_ref: &case.requester,
            peer_ref: &case.peer,
            object_ref: &fake_ref("workflow-wrong-object"),
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &case.remote,
            policy_ref: &fake_ref("workflow-wrong-policy"),
            authority_ref: &fake_ref("workflow-wrong-authority"),
            evidence_refs: &[],
        })
        .expect("store wrong request");
        let wrong_import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
            root: &root,
            request_value: &wrong_request.value,
            response_value: &pair.response_value,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("deny wrong request import");
        assert_eq!(wrong_import.decision, "deny");
        assert!(wrong_import.clearance_ref.is_none());
        assert!(wrong_import.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-wrong-request"));

        let tampered_response =
            crate::preserves_rail::record("not-a-remote-clearance-response", vec![crate::preserves_rail::string(
                "tampered",
            )]);
        let tampered_import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
            root: &root,
            request_value: &pair.request_value,
            response_value: &tampered_response,
            expected_peer_ref: Some(&case.peer),
            expected_remote_ref: Some(&case.remote),
        })
        .expect("deny tampered response import");
        assert_eq!(tampered_import.decision, "deny");
        assert!(tampered_import.clearance_ref.is_none());
        assert!(
            tampered_import
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("remote-clearance-tampered-response"))
        );
    }
