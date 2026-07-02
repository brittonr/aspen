
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
