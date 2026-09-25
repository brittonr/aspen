
    #[test]
    fn gc_audit_binds_plan_apply_execution_receipt_and_tombstone() {
        let root = temp_dir("retention-gc-audit-pass");
        let fixture = store_passing_plan_fixture(&root, "audit-pass");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store audit plan");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply audit plan");
        let execution = store_gc_execution_gate(GcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store execution gate");
        assert_eq!(execution.decision, "pass");
        let audit = audit_gc_execution(GcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit execution");
        assert_eq!(audit.decision, "pass");
        assert_eq!(audit.plan_ref.as_deref(), Some(plan.plan_ref.as_str()));
        assert_eq!(audit.apply_ref.as_deref(), Some(apply.apply_ref.as_str()));
        assert_eq!(audit.execution_ref, execution.execution_ref);
        assert_eq!(audit.retention_receipt_ref, apply.retention_receipt_ref);
        assert_eq!(audit.tombstone_ref, apply.tombstone_ref);
        assert_eq!(store_file_count(&gc_audits_dir(&root)), 1);
        assert_summary_contains(&audit.value, "retention gc audit");
        let lifecycle = evaluate_gc_lifecycle(RetentionGcLifecycleInput {
            plan: Some(&plan),
            apply: Some(&apply),
            execution: Some(&execution),
            audit: Some(&audit),
        });
        assert_eq!(lifecycle.decision, "pass");
    }

    #[test]
    fn gc_lifecycle_core_denies_broken_chain_links() {
        let root = temp_dir("retention-gc-lifecycle-broken");
        let fixture = store_passing_plan_fixture(&root, "lifecycle-broken");
        let flow = passing_flow(&root, &fixture, "ledger-gc");
        let mut wrong_execution = flow.execution.clone();
        wrong_execution.plan_ref = Some(fake_ref("wrong-plan-link"));
        let broken_execution = evaluate_gc_lifecycle(RetentionGcLifecycleInput {
            plan: Some(&flow.plan),
            apply: Some(&flow.apply),
            execution: Some(&wrong_execution),
            audit: Some(&flow.audit),
        });
        assert_eq!(broken_execution.decision, "deny");
        assert!(broken_execution
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-lifecycle-execution-plan-mismatch"));

        let mut wrong_audit = flow.audit.clone();
        wrong_audit.apply_ref = Some(fake_ref("wrong-apply-link"));
        let broken_audit = evaluate_gc_lifecycle(RetentionGcLifecycleInput {
            plan: Some(&flow.plan),
            apply: Some(&flow.apply),
            execution: Some(&flow.execution),
            audit: Some(&wrong_audit),
        });
        assert_eq!(broken_audit.decision, "deny");
        assert!(broken_audit
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-lifecycle-audit-apply-mismatch"));
    }

    #[test]
    fn gc_execution_scope_mismatch_denies_before_subsystem_mutation() {
        let root = temp_dir("retention-gc-execution-scope-mismatch");
        let fixture = store_passing_plan_fixture(&root, "execution-scope-mismatch");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        let wrong_object_ref = fake_ref("wrong-execution-object");
        let execution = store_gc_execution_gate(GcExecutionGateInput {
            root: &root,
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &wrong_object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
        })
        .expect("store mismatched execution gate");
        assert_eq!(execution.decision, "deny");
        assert!(execution
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-execute-apply-scope-mismatch"));
        let audit = audit_gc_execution(GcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit mismatched execution");
        assert_eq!(audit.decision, "deny");
        let lifecycle = evaluate_gc_lifecycle(RetentionGcLifecycleInput {
            plan: Some(&plan),
            apply: Some(&apply),
            execution: Some(&execution),
            audit: Some(&audit),
        });
        assert_eq!(lifecycle.decision, "deny");
        assert!(lifecycle
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-lifecycle-apply-execution-scope-mismatch"));
    }

    #[test]
    fn gc_audit_and_lifecycle_deny_missing_tombstone_evidence() {
        let root = temp_dir("retention-gc-missing-tombstone");
        let fixture = store_passing_plan_fixture(&root, "missing-tombstone");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        assert!(apply.tombstone_ref.is_some());
        let execution_value = execution_gate_value(&ExecutionGateValueInput {
            decision: "pass",
            subsystem: "ledger-gc",
            action: ACTION_DELETE,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            apply_ref: Some(&apply.apply_ref),
            plan_ref: Some(&plan.plan_ref),
            recomputed_plan_ref: Some(&plan.plan_ref),
            retention_receipt_ref: apply.retention_receipt_ref.as_deref(),
            tombstone_ref: None,
            diagnostics: &[],
        })
        .expect("execution value without tombstone");
        let execution = parse_gc_execution_gate(&execution_value).expect("parse forged execution");
        write_store_value(&gc_execute_path(&root, &execution.execution_ref).expect("execution path"), &execution.value)
            .expect("write forged execution");
        let audit = audit_gc_execution(GcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit forged missing tombstone");
        assert_eq!(audit.decision, "deny");
        assert!(audit
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-audit-tombstone-missing"));
        let lifecycle = evaluate_gc_lifecycle(RetentionGcLifecycleInput {
            plan: Some(&plan),
            apply: Some(&apply),
            execution: Some(&execution),
            audit: Some(&audit),
        });
        assert_eq!(lifecycle.decision, "deny");
        assert!(lifecycle
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-lifecycle-execution-tombstone-mismatch"));
    }

    #[test]
    fn candidate_explain_lists_known_gc_evidence() {
        let root = temp_dir("retention-candidate-explain");
        let fixture = store_passing_plan_fixture(&root, "explain-pass");
        let flow = passing_flow(&root, &fixture, "ledger-gc");
        let explain = explain_candidate(CandidateExplainInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: Some("chunk"),
            retention_class: Some(CLASS_DURABLE_VALUE),
            action: Some(ACTION_DELETE),
            subsystem: Some("ledger-gc"),
        })
        .expect("explain retention candidate");
        assert_eq!(explain.pin_refs.len(), 0);
        assert_eq!(explain.admission_refs.len(), 5);
        assert_eq!(explain.remote_clearance_refs.len(), 1);
        assert_eq!(explain.gc_plan_refs, vec![flow.plan.plan_ref.clone()]);
        assert_eq!(explain.gc_apply_refs, vec![flow.apply.apply_ref.clone()]);
        assert_eq!(explain.gc_execution_refs, vec![flow.execution.execution_ref.clone()]);
        assert_eq!(explain.gc_audit_refs, vec![flow.audit.audit_ref.clone()]);
        assert_eq!(explain.retention_receipt_refs.len(), 1);
        assert_eq!(explain.tombstone_refs.len(), 1);
        assert!(explain.diagnostics.is_empty());
        assert_summary_contains(&explain.value, "retention candidate explain");
        let bundle_dir = root.join("bundle");
        let bundle = export_candidate_bundle(CandidateBundleExportInput {
            root: &root,
            explain_value: &explain.value,
            out: &bundle_dir,
            profile: CandidateBundleExportProfile::Internal,
        })
        .expect("export retention candidate bundle");
        assert_eq!(bundle.explain_ref, explain.explain_ref);
        assert_eq!(bundle.artifact_refs.len(), 6);
        assert!(bundle.diagnostics.is_empty());
        assert!(bundle_dir.join("bundle.preserves").exists());
        assert!(bundle_dir.join("explain.preserves").exists());
        assert!(bundle_dir.join("artifacts/gc-plans").exists());
        assert_summary_contains(&bundle.value, "retention candidate bundle");
        let verify = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir: &bundle_dir,
        })
        .expect("verify intact retention candidate bundle");
        assert_eq!(verify.decision, "pass");
        assert_eq!(verify.bundle_ref, bundle.bundle_ref);
        assert_eq!(verify.explain_ref, explain.explain_ref);
        assert_eq!(verify.artifact_refs.len(), 6);
        assert_eq!(verify.file_refs.len(), 6);
        assert!(verify.diagnostics.is_empty());
        assert_summary_contains(&verify.value, "retention candidate bundle verify");
        assert_tampered_bundle_denied(&bundle_dir, &flow.plan.plan_ref);
    }
