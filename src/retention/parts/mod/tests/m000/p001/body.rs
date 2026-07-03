
    #[test]
    fn destructive_admission_accepts_matching_remote_gc_refs() {
        let root = temp_dir("retention-admission-remote");
        let fixture = store_passing_plan_fixture(&root, "admission-remote");
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root: &root,
            evidence: &fixture.evidence,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("admission pass");
        assert_eq!(admission.decision, "pass");
        assert!(admission.has_delete_authority);
        assert!(admission.has_remote_gc_clearance);
        let evaluation = evaluate(EvaluationInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            requester_ref: &fixture.requester_ref,
            is_reference_index_complete: true,
            retained_refs: &[],
            remote_refs: &fixture.evidence.remote_refs,
            policy_refs: &fixture.evidence.policy_refs,
            evidence_refs: &fixture.evidence.evidence_refs,
            has_delete_authority: admission.has_delete_authority,
            has_remote_gc_clearance: admission.has_remote_gc_clearance,
        })
        .expect("evaluate remote clearance");
        assert_eq!(evaluation.receipt.decision, "pass");
    }

    #[test]
    fn gc_plan_lists_gates_and_avoids_receipts_or_tombstones() {
        let root = temp_dir("retention-gc-plan-pass");
        let fixture = store_passing_plan_fixture(&root, "plan-pass");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "chunk-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store plan");
        assert_eq!(plan.decision, "pass");
        assert_eq!(store_file_count(&receipts_dir(&root)), 0);
        assert_eq!(store_file_count(&tombstones_dir(&root)), 0);
        assert_eq!(store_file_count(&gc_plans_dir(&root)), 1);
        let gate_names = plan.gates.iter().map(|gate| gate.name.as_str()).collect::<Vec<_>>();
        assert!(gate_names.contains(&"policy"));
        assert!(gate_names.contains(&"authority"));
        assert!(gate_names.contains(&"reference-index"));
        assert!(gate_names.contains(&"remote-gc"));
        assert!(gate_names.contains(&"remote-clearance"));
        let parsed = parse_gc_plan(&plan.value).expect("parse plan");
        assert_eq!(parsed.plan_ref, plan.plan_ref);
    }

    #[test]
    fn gc_plan_rejects_requester_evidence_mismatch() {
        let root = temp_dir("retention-gc-plan-requester-mismatch");
        let fixture = store_passing_plan_fixture(&root, "plan-requester-mismatch");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store requester-bound plan");
        let mut mismatched_evidence = fixture.evidence.clone();
        mismatched_evidence.requester_ref = Some(fake_ref("wrong-plan-requester"));
        let evidence_value = destructive_evidence_value(&mismatched_evidence).expect("mismatched evidence value");
        let index = reference_index_for_object(ReferenceIndexForObjectInput {
            root: &root,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retained_refs: &fixture.evidence.retained_refs,
            remote_refs: &fixture.evidence.remote_refs,
            is_complete: fixture.evidence.is_reference_index_complete,
        })
        .expect("reference index");
        let tampered = gc_plan_value(&GcPlanValueInput {
            decision: &plan.decision,
            subsystem: &plan.subsystem,
            action: &plan.action,
            object_ref: &plan.object_ref,
            object_kind: &plan.object_kind,
            retention_class: &plan.retention_class,
            requester_ref: plan.requester_ref.as_deref(),
            index: &index,
            evidence_value: &evidence_value,
            gates: &plan.gates,
            diagnostics: &plan.diagnostics,
        })
        .expect("tampered plan value");
        let error = parse_gc_plan(&tampered).expect_err("requester mismatch must fail closed");
        assert!(format!("{error}").contains("requester evidence mismatch"));
    }

    #[test]
    fn gc_plan_denies_missing_clearance_and_is_not_clearance() {
        let root = temp_dir("retention-gc-plan-deny");
        let fixture = store_passing_plan_fixture(&root, "plan-deny");
        let mut evidence = fixture.evidence.clone();
        evidence.remote_clearance_refs.clear();
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
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-evidence-missing"));
        let mut plan_as_clearance = evidence;
        plan_as_clearance.remote_clearance_refs = vec![plan.plan_ref];
        let admission = admit_destructive_evidence(DestructiveAdmissionInput {
            root: &root,
            evidence: &plan_as_clearance,
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
        })
        .expect("plan ref is not clearance");
        assert_eq!(admission.decision, "deny");
        assert!(admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("remote-clearance-unreadable")));
    }

    #[test]
    fn gc_apply_from_plan_writes_apply_receipt_and_tombstone() {
        let root = temp_dir("retention-gc-apply-pass");
        let fixture = store_passing_plan_fixture(&root, "apply-pass");
        let plan = store_gc_plan(GcPlanInput {
            root: &root,
            subsystem: "ledger-gc",
            object_ref: &fixture.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            evidence: &fixture.evidence,
        })
        .expect("store apply plan");
        let apply = apply_gc_plan(GcApplyFromPlanInput {
            root: &root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply plan");
        assert_eq!(apply.decision, "pass");
        assert_eq!(apply.plan_ref, plan.plan_ref);
        assert_eq!(apply.recomputed_plan_ref, plan.plan_ref);
        assert!(apply.retention_receipt_ref.is_some());
        assert!(apply.tombstone_ref.is_some());
        assert_eq!(store_file_count(&gc_applies_dir(&root)), 1);
        assert_eq!(store_file_count(&tombstones_dir(&root)), 1);
        let parsed = parse_gc_apply(&apply.value).expect("parse apply");
        assert_eq!(parsed.apply_ref, apply.apply_ref);
        let receipt = read_receipt(&root, parsed.retention_receipt_ref.as_deref().expect("receipt ref"))
            .expect("read retention receipt");
        assert!(receipt.tombstone_ref.is_none());
        let tombstone = read_tombstone(&root, parsed.tombstone_ref.as_deref().expect("tombstone ref"))
            .expect("read retention tombstone");
        assert_eq!(tombstone.receipt_ref, receipt.receipt_ref);
    }

    #[test]
    fn gc_audit_rejects_tombstone_bound_to_pending_receipt() {
        let root = temp_dir("retention-gc-pending-tombstone");
        let fixture = store_passing_plan_fixture(&root, "pending-tombstone");
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
        let pending_receipt_ref = synthetic_ref("pending-retention-receipt").expect("pending ref");
        let tombstone_value = crate::preserves_rail::record("retention-tombstone-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::RETENTION_TOMBSTONE_SCHEMA),
            object_value(&fixture.object_ref, "chunk"),
            crate::preserves_rail::record("class", vec![crate::preserves_rail::string(CLASS_DURABLE_VALUE)]),
            crate::preserves_rail::record("action", vec![crate::preserves_rail::string(ACTION_DELETE)]),
            crate::preserves_rail::record("receipt", vec![crate::preserves_rail::string(&pending_receipt_ref)]),
            crate::preserves_rail::record("policy", vec![strings_sequence(&fixture.evidence.policy_refs)]),
            crate::preserves_rail::record("evidence", vec![strings_sequence(&fixture.evidence.evidence_refs)]),
            crate::preserves_rail::record("public-metadata", vec![crate::preserves_rail::sequence(vec![
                crate::preserves_rail::record("object-kind", vec![crate::preserves_rail::string("chunk")]),
                crate::preserves_rail::record("class", vec![crate::preserves_rail::string(CLASS_DURABLE_VALUE)]),
                crate::preserves_rail::record("content", vec![crate::preserves_rail::string("redacted-or-deleted")]),
            ])]),
            checks_value(&[
                ("audit-visible-tombstone", "pass"),
                ("secret-content-not-leaked", "pass"),
                ("deletion-not-hidden", "pass"),
            ]),
        ]);
        let tombstone = parse_tombstone(&tombstone_value).expect("parse pending tombstone");
        write_store_value(&tombstone_path(&root, &tombstone.tombstone_ref).expect("tombstone path"), &tombstone.value)
            .expect("write pending tombstone");
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
            tombstone_ref: Some(&tombstone.tombstone_ref),
            diagnostics: &[],
        })
        .expect("execution value");
        let execution = parse_gc_execution_gate(&execution_value).expect("parse execution");
        write_store_value(&gc_execute_path(&root, &execution.execution_ref).expect("execution path"), &execution.value)
            .expect("write execution");

        let audit = audit_gc_execution(GcAuditInput {
            root: &root,
            execution_ref: &execution.execution_ref,
        })
        .expect("audit forged execution");
        assert_eq!(audit.decision, "deny");
        assert!(audit
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic == "retention-gc-audit-tombstone-receipt-mismatch"));
    }

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
        let tampered_path = bundle_dir
            .join("artifacts/gc-plans")
            .join(format!("{}.preserves", ref_file_name(&flow.plan.plan_ref).expect("plan file name")));
        write_store_value(
            &tampered_path,
            &crate::preserves_rail::record("tampered", vec![crate::preserves_rail::string("plan")]),
        )
        .expect("tamper bundle plan");
        let tampered = verify_candidate_bundle(CandidateBundleVerifyInput {
            bundle_dir: &bundle_dir,
        })
        .expect("verify tampered retention candidate bundle");
        assert_eq!(tampered.decision, "deny");
        assert!(
            tampered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("retention-bundle-tampered-file:gc-plans"))
        );
    }
