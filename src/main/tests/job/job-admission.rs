    fn exercise_job_admission(setup: &JobSetup, sync: &JobSync) -> JobAdmission {
        let admit_plan_out = setup.dir.join("job-admit-plan.preserves");
        let admit_loopback_receipt = setup.dir.join("job-admit-loopback-receipt.preserves");
        run_job_command(JobCommand::AdmitPlan(cli_job::command::sync::AdmitPlan {
            job: setup.dag.job_ref.clone(),
            target_registry: setup.target_registry.clone(),
            sync_ref: Some(sync.sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![sync.admission_policy_ref.clone()],
            capability_refs: vec![sync.authority_context_ref.clone()],
            evidence_refs: vec![sync.sync_ref.clone(), sync.source_gate_ref.clone()],
            resource_refs: sync.worker_resource_refs.clone(),
            out: Some(admit_plan_out.clone()),
            receipt_out: Some(setup.dir.join("job-admit-plan-receipt.preserves")),
        }))
        .expect("job admit plan");
        run_job_command(JobCommand::AdmitLoopback(cli_job::command::sync::AdmitLoopback {
            job: setup.dag.job_ref.clone(),
            target_registry: setup.target_registry.clone(),
            sync_ref: Some(sync.sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![sync.admission_policy_ref.clone()],
            capability_refs: vec![sync.authority_context_ref.clone()],
            evidence_refs: vec![sync.sync_ref.clone(), sync.source_gate_ref.clone()],
            resource_refs: sync.worker_resource_refs.clone(),
            plan_out: Some(setup.dir.join("job-admit-loopback-plan.preserves")),
            receipt_out: Some(admit_loopback_receipt.clone()),
        }))
        .expect("job admit loopback");
        assert!(fs::read_to_string(&admit_plan_out).expect("read admit plan").contains("job-admission-plan-v1"));
        let missing_execution_receipt = setup.dir.join("job-execute-missing-admission-receipt.preserves");
        run_job_command(JobCommand::ExecuteLoopback(cli_job::command::sync::ExecuteLoopback {
            job: setup.dag.job_ref.clone(),
            target_registry: setup.target_registry.clone(),
            storage: setup.storage.clone(),
            cache: setup.cache.clone(),
            chunks: Some(setup.chunks.clone()),
            admission_receipt: setup.dir.join("missing-admission.preserves"),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            resource_refs: Vec::new(),
            request_out: Some(setup.dir.join("job-execute-missing-request.preserves")),
            out: None,
            receipt_out: Some(missing_execution_receipt.clone()),
        }))
        .expect_err("missing admission denies execution");
        assert_eq!(
            ledger::artifact_kind(&read_preserves_file(&missing_execution_receipt).expect("missing execution receipt")),
            "job-execution-receipt"
        );
        let worker_execution_request = setup.dir.join("job-worker-execution-request.preserves");
        run_job_command(JobCommand::ExecuteLoopback(cli_job::command::sync::ExecuteLoopback {
            job: setup.dag.job_ref.clone(),
            target_registry: setup.target_registry.clone(),
            storage: setup.storage.clone(),
            cache: setup.cache.clone(),
            chunks: Some(setup.chunks.clone()),
            admission_receipt: admit_loopback_receipt.clone(),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            policy_refs: vec![sync.admission_policy_ref.clone()],
            capability_refs: vec![sync.authority_context_ref.clone()],
            resource_refs: sync.worker_resource_refs.clone(),
            request_out: Some(worker_execution_request.clone()),
            out: Some(setup.dir.join("job-execute-loopback-output.preserves")),
            receipt_out: Some(setup.dir.join("job-execute-loopback-receipt.preserves")),
        }))
        .expect("job execute loopback pass");
        JobAdmission {
            admit_loopback_receipt,
            worker_execution_request,
        }
    }
