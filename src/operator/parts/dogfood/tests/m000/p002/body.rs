
    #[test]
    fn missing_redaction_or_stale_operator_policy_denies_report() {
        let request_ref = dogfood_ref("request").expect("request ref");
        let receipt_ref = dogfood_ref("receipt").expect("receipt ref");
        let step = operator_step_value(&OperatorStepInput {
            name: "gate-evidence",
            request_ref: Some(&request_ref),
            receipt_ref: Some(&receipt_ref),
            decision: "pass",
            replay_status: "deterministic",
            mandatory: true,
            artifact_refs: &[],
            diagnostics: &[],
        })
        .expect("step");
        let workflow = operator_workflow_value(&OperatorWorkflowInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            steps: &[step],
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[dogfood_ref("resource").expect("resource")],
            replay_profile: "deterministic",
        })
        .expect("workflow");
        let checkpoint = operator_checkpoint_value(&OperatorCheckpointInput {
            workflow_id: LOCAL_NODE_WORKFLOW_ID,
            sequence: 0,
            step_ref: &dogfood_ref("step").expect("step"),
            request_ref: Some(&request_ref),
            receipt_ref: Some(&receipt_ref),
            result_ref: Some(&receipt_ref),
            state_root_ref: &dogfood_ref("state").expect("state"),
        })
        .expect("checkpoint");
        let report = dogfood_report_value(&DogfoodReportInput {
            workflow_value: &workflow,
            checkpoint_values: &[checkpoint],
            gate_receipt_refs: &[dogfood_ref("gate").expect("gate")],
            repro_bundle_refs: &[],
            final_state_ref: &dogfood_ref("final-state").expect("final state"),
            diagnostics: &[],
        })
        .expect("report");
        let parsed = parse_dogfood_report(&report).expect("parse report");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("operator policy/capability")));
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("sealed/redacted repro")));
    }

    #[test]
    fn dirty_state_root_denies_without_release_gate() {
        let root = temp_dir("operator-dogfood-dirty");
        std::fs::write(root.join("leftover"), "dirty").expect("write dirty marker");
        let run = run_local_node_dogfood(&LocalNodeDogfoodInput { state_root: &root }).expect("dirty report");
        assert_eq!(run.decision, "deny");
        assert!(run.release_gate_value.is_none());
        let report = parse_dogfood_report(&run.report_value).expect("parse dirty report");
        assert!(report.diagnostics.iter().any(|diagnostic| diagnostic.contains("clean empty state root")));
    }

    #[test]
    fn release_evidence_remains_evidence_only_for_subsystem_gates() {
        let release_ref = dogfood_ref("release-boundary").expect("release ref");
        let authority_ref = dogfood_ref("release-boundary-authority").expect("authority ref");
        let policy_ref = dogfood_ref("release-boundary-policy").expect("policy ref");
        let provenance_ref = dogfood_ref("release-boundary-provenance").expect("provenance ref");
        let source_gate_ref = dogfood_ref("release-boundary-source-gate").expect("source gate ref");
        let retention_ref = dogfood_ref("release-boundary-retention").expect("retention ref");
        let resource_ref = dogfood_ref("release-boundary-resource").expect("resource ref");
        let transport_ref = dogfood_ref("release-boundary-transport").expect("transport ref");
        let destructive_ref = dogfood_ref("release-boundary-destructive").expect("destructive ref");

        let denied = evaluate_release_evidence_only_boundary(&ReleaseEvidenceBoundaryInput {
            operation: "privileged-release-misuse",
            release_receipt_refs: std::slice::from_ref(&release_ref),
            authority_refs: &[],
            policy_refs: &[],
            provenance_refs: &[],
            source_gate_refs: &[],
            retention_refs: &[],
            resource_refs: &[],
            transport_refs: &[],
            destructive_operation_refs: &[],
        })
        .expect("release boundary deny");
        assert_eq!(denied.decision, "deny");
        for gate in RELEASE_EVIDENCE_BOUNDARY_GATES {
            assert!(
                denied
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains(&format!("{gate} trust"))),
                "missing release evidence-only diagnostic for {gate}: {:?}",
                denied.diagnostics
            );
        }

        let admitted = evaluate_release_evidence_only_boundary(&ReleaseEvidenceBoundaryInput {
            operation: "fully-gated-release-review",
            release_receipt_refs: std::slice::from_ref(&release_ref),
            authority_refs: std::slice::from_ref(&authority_ref),
            policy_refs: std::slice::from_ref(&policy_ref),
            provenance_refs: std::slice::from_ref(&provenance_ref),
            source_gate_refs: std::slice::from_ref(&source_gate_ref),
            retention_refs: std::slice::from_ref(&retention_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            transport_refs: std::slice::from_ref(&transport_ref),
            destructive_operation_refs: std::slice::from_ref(&destructive_ref),
        })
        .expect("release boundary pass with subsystem gates");
        assert_eq!(admitted.decision, "pass");
    }

    fn temp_dir(label: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
