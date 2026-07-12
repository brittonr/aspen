
    #[test]
    fn control_service_heartbeats_continue_and_shutdown_stops() {
        let root = temp_dir("node-control-service-shutdown");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:service-shutdown",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let idle = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 2,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("idle serve");
        assert_eq!(idle.decision, "pass");
        assert_eq!(idle.heartbeat_receipt_refs.len(), 2);
        assert_eq!(idle.loop_receipt_refs.len(), 2);
        assert!(!idle.has_stopped);

        let shutdown = shutdown_request().expect("shutdown request");
        submit_control_request(&ControlSubmitInput {
            state_root: &root,
            request_value: &shutdown.value,
        })
        .expect("submit shutdown");
        let stopped = serve_control(&ControlServeInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_ticks: 4,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        })
        .expect("shutdown serve");
        assert_eq!(stopped.decision, "pass");
        assert!(stopped.has_stopped);
        assert_eq!(stopped.processed_request_refs.len(), 1);
        assert!(!root.join(CONTROL_LOCK_FILE).exists());
        assert!(!root.join(CONTROL_SERVICE_LOCK_FILE).exists());
    }

    #[test]
    fn control_operation_dispatch_installs_runs_and_gates_with_receipts() {
        let case = op_case();
        assert_install(&case);
        assert_gate(&case);
        assert_run(&case);
        assert_ledger(&case.root);
    }

    struct OpCase {
        root: PathBuf,
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn op_case() -> OpCase {
        let root = temp_dir("node-control-operations");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ops",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        OpCase {
            root,
            authority_refs: vec![local_ref("node-control-authority", "ops").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "ops").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "ops").expect("resource ref")],
        }
    }

    fn dispatch_value(case: &OpCase, value: &IoValue) -> crate::node_runtime::ControlReceipt {
        let submitted = submit_control_request(&ControlSubmitInput {
            state_root: &case.root,
            request_value: value,
        })
        .expect("submit request");
        let request_path = case.root.join(CONTROL_INBOX_DIR).join(&submitted.inbox_entry);
        let dispatched = dispatch_control_request(&ControlDispatchInput {
            state_root: &case.root,
            request_path: Some(&request_path),
        })
        .expect("dispatch request");
        crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value).expect("control receipt")
    }

    fn assert_install(case: &OpCase) {
        let payload_value =
            crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
                "payload",
            )]);
        let payload_ref = import_artifact(&case.root, &payload_value).expect("import payload");
        let payload_provenance =
            crate::provenance::synthetic_reviewed_record(&payload_ref).expect("payload provenance");
        let payload_provenance_ref =
            import_artifact(&case.root, &payload_provenance).expect("import payload provenance");
        let install_evidence_refs = vec![payload_provenance_ref];
        let install_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "install",
                target_ref: None,
                payload_ref: Some(&payload_ref),
                authority_refs: &case.authority_refs,
                policy_refs: &case.policy_refs,
                resource_refs: &case.resource_refs,
                evidence_refs: &install_evidence_refs,
            })
            .expect("install request");
        let install_receipt = dispatch_value(case, &install_value);
        assert_eq!(install_receipt.decision, "pass");
        let installed = crate::artifacts::list_artifacts(&case.root.join("registry"), Some("node-control-artifact"))
            .expect("list installed artifacts");
        assert_eq!(installed.len(), 1);
    }

    fn assert_gate(case: &OpCase) {
        let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("gate receipt");
        let gate_ref = import_artifact(&case.root, &gate_value).expect("import gate");
        let gate_target = local_ref("node-control-gate-target", "ops").expect("gate target");
        let gate_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "gate",
            target_ref: Some(&gate_target),
            payload_ref: Some(&gate_ref),
            authority_refs: &case.authority_refs,
            policy_refs: &case.policy_refs,
            resource_refs: &case.resource_refs,
            evidence_refs: &[],
        })
        .expect("gate request");
        let gate_receipt = dispatch_value(case, &gate_request);
        assert_eq!(gate_receipt.decision, "pass");
        assert!(
            gate_receipt
                .subreceipt_refs
                .iter()
                .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
        );
    }

    fn assert_run(case: &OpCase) {
        let job_fixture = install_job_fixture(&case.root);
        let execution_request_ref =
            import_artifact(&case.root, &job_fixture.execution_request).expect("import execution request");
        let admission_ref =
            import_artifact(&case.root, &job_fixture.admission_receipt).expect("import admission receipt");
        let job_provenance =
            crate::provenance::synthetic_reviewed_record(&job_fixture.job_ref).expect("job provenance");
        let job_provenance_ref = import_artifact(&case.root, &job_provenance).expect("import job provenance");
        let run_evidence_refs = vec![job_provenance_ref];
        let run_request = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "run",
            target_ref: Some(&admission_ref),
            payload_ref: Some(&execution_request_ref),
            authority_refs: &case.authority_refs,
            policy_refs: &case.policy_refs,
            resource_refs: &case.resource_refs,
            evidence_refs: &run_evidence_refs,
        })
        .expect("run request");
        let run_receipt = dispatch_value(case, &run_request);
        assert_eq!(run_receipt.decision, "pass");
    }

    fn assert_ledger(root: &Path) {
        let kinds = crate::ledger::list_artifacts(&root.join("ledger"))
            .expect("list operation ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<Vec<_>>();
        assert!(kinds.iter().any(|kind| kind == "artifact-registry-receipt"));
        assert!(kinds.iter().any(|kind| kind == "provenance-record"));
        assert!(kinds.iter().any(|kind| kind == "provenance-receipt"));
        assert!(kinds.iter().any(|kind| kind == "job-execution-receipt"));
        assert!(kinds.iter().any(|kind| kind == "octet-source-gate-validation"));
        assert!(kinds.iter().any(|kind| kind == "node-control-operation-receipt"));
    }

    struct JobFixture {
        execution_request: IoValue,
        admission_receipt: IoValue,
        job_ref: String,
    }

    struct StagePair {
        source_ref: String,
        map_ref: String,
    }

    struct AdmissionParts {
        receipt_value: IoValue,
        receipt_ref: String,
        stage_order: Vec<String>,
        policy_refs: Vec<String>,
        capability_refs: Vec<String>,
        resource_refs: Vec<String>,
    }

    fn install_job_fixture(root: &Path) -> JobFixture {
        let registry = root.join("registry");
        let stages = install_stage_pair(&registry);
        let dag_value = graph_value(&stages);
        let installed = crate::job_dag::install_job_dag(&registry, &dag_value).expect("install job dag");
        let admission = admit_graph(&registry, &installed.job_ref);
        let execution_request = execution_request_value(&installed.job_ref, &admission);
        JobFixture {
            execution_request,
            admission_receipt: admission.receipt_value,
            job_ref: installed.job_ref,
        }
    }

    fn install_stage_pair(registry: &Path) -> StagePair {
        let stage_schema = local_ref("node-job-stage-schema", "ops").expect("stage schema");
        let stage_policy = local_ref("node-job-stage-policy", "ops").expect("stage policy");
        let stage_evidence = local_ref("node-job-stage-evidence", "ops").expect("stage evidence");
        let stage_installer = local_ref("node-job-stage-installer", "ops").expect("stage installer");
        let stage_capability = local_ref("node-job-stage-capability", "ops").expect("stage capability");
        let source_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: crate::job_dag::builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![stage_schema.clone()],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy.clone()],
            evidence_refs: vec![stage_evidence.clone()],
            installer_ref: stage_installer.clone(),
            capability_refs: vec![stage_capability.clone()],
        })
        .expect("install source stage");
        let map_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: crate::job_dag::builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![stage_schema],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![stage_policy],
            evidence_refs: vec![stage_evidence],
            installer_ref: stage_installer,
            capability_refs: vec![stage_capability],
        })
        .expect("install map stage");
        StagePair {
            source_ref: source_stage.artifact_ref,
            map_ref: map_stage.artifact_ref,
        }
    }

    fn source_vertex_value(stage_ref: &str) -> IoValue {
        crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(stage_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("node-job")]),
            ])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node")
    }

    fn map_vertex_value(stage_ref: &str) -> IoValue {
        crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(stage_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node")
    }

    fn fixture_edge_value() -> IoValue {
        crate::job_dag::job_edge_value(crate::job_dag::EdgeValueInput {
            from_node: "source",
            from_port: "out",
            to_node: "map",
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
        .expect("edge")
    }
