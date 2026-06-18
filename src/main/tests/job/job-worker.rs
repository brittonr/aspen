    fn exercise_job_worker(setup: &JobSetup, sync: &JobSync, admission: &JobAdmission) {
        let worker_request = setup.dir.join("job-worker-request.preserves");
        let peer_bootstrap_ref = cli_synthetic_ref("job-worker-peer-bootstrap").expect("peer bootstrap");
        let node_identity_ref = cli_synthetic_ref("job-worker-node-identity").expect("node identity");
        run_job_command(JobCommand::WorkerRequest(cli_job::command::worker::Request {
            admission_receipt: admission.admit_loopback_receipt.clone(),
            execution_request: admission.worker_execution_request.clone(),
            sync_ref: Some(sync.sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            authority_refs: vec![sync.authority_context_ref.clone()],
            resource_refs: sync.worker_resource_refs.clone(),
            peer_bootstrap_refs: vec![peer_bootstrap_ref],
            node_identity_refs: vec![node_identity_ref],
            evidence_refs: Vec::new(),
            out: Some(worker_request.clone()),
        }))
        .expect("job worker request");
        let worker_out = setup.dir.join("job-worker-local");
        run_job_command(JobCommand::WorkerRunLocal(cli_job::command::worker::RunLocal {
            request: worker_request.clone(),
            target_registry: setup.target_registry.clone(),
            storage: setup.dir.join("worker-storage"),
            cache: setup.dir.join("worker-cache"),
            chunks: Some(setup.dir.join("worker-chunks")),
            admission_receipt: admission.admit_loopback_receipt.clone(),
            execution_request: admission.worker_execution_request.clone(),
            transport_root: setup.dir.join("worker-transport"),
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            ledger: Some(setup.ledger_root.clone()),
            out: worker_out.clone(),
        }))
        .expect("job worker local run");
        let worker_receipt = read_preserves_file(&worker_out.join("worker-receipt.preserves")).expect("worker receipt");
        assert_eq!(ledger::artifact_kind(&worker_receipt), "job-worker-receipt");
        assert!(fs::read_to_string(worker_out.join("output.preserves")).expect("worker output").contains("3"));
        let worker_receipt_ref = canonical_hash(&worker_receipt).expect("worker receipt ref");
        run_job_command(JobCommand::ReceiptShow(cli_job::command::refs::ReceiptShow {
            receipt_ref: worker_receipt_ref,
            ledger: setup.ledger_root.clone(),
        }))
        .expect("job worker receipt show");
        exercise_job_worker_schedule(setup, sync, admission, worker_request);
    }

    fn exercise_job_worker_schedule(
        setup: &JobSetup,
        sync: &JobSync,
        admission: &JobAdmission,
        worker_request: PathBuf,
    ) {
        let schedule_out = setup.dir.join("job-worker-scheduled");
        run_job_command(JobCommand::WorkerScheduleLocal(cli_job::command::worker::ScheduleLocal {
            request: worker_request.clone(),
            target_registry: setup.target_registry.clone(),
            storage: setup.dir.join("scheduled-worker-storage"),
            cache: setup.dir.join("scheduled-worker-cache"),
            chunks: Some(setup.dir.join("scheduled-worker-chunks")),
            admission_receipt: admission.admit_loopback_receipt.clone(),
            execution_request: admission.worker_execution_request.clone(),
            transport_root: setup.dir.join("scheduled-worker-transport"),
            queue_key: "queue:job-worker".to_string(),
            lease_key: None,
            scheduler_session: "scheduler".to_string(),
            worker_session: "worker-a".to_string(),
            lease_token: None,
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            coordination_authority_refs: vec![sync.authority_context_ref.clone()],
            coordination_resource_refs: sync.worker_resource_refs.clone(),
            coordination_policy_refs: vec![cli_synthetic_ref("job-worker-schedule-policy").expect("schedule policy")],
            ledger: Some(setup.ledger_root.clone()),
            out: schedule_out.clone(),
        }))
        .expect("job worker scheduled local run");
        let schedule_receipt =
            read_preserves_file(&schedule_out.join("schedule-receipt.preserves")).expect("schedule receipt");
        assert_eq!(ledger::artifact_kind(&schedule_receipt), "job-worker-schedule-receipt");
        assert!(
            fs::read_to_string(schedule_out.join("worker").join("output.preserves"))
                .expect("scheduled worker output")
                .contains("3")
        );
        let enqueue_ref = canonical_hash(
            &read_preserves_file(&schedule_out.join("coordination").join("enqueue-receipt.preserves"))
                .expect("enqueue receipt"),
        )
        .expect("enqueue ref");
        let duplicate_enqueue_ref = canonical_hash(
            &read_preserves_file(&schedule_out.join("coordination").join("enqueue-duplicate-receipt.preserves"))
                .expect("duplicate enqueue receipt"),
        )
        .expect("duplicate enqueue ref");
        assert_eq!(enqueue_ref, duplicate_enqueue_ref);
        let schedule_receipt_ref = canonical_hash(&schedule_receipt).expect("schedule receipt ref");
        run_job_command(JobCommand::ReceiptShow(cli_job::command::refs::ReceiptShow {
            receipt_ref: schedule_receipt_ref,
            ledger: setup.ledger_root.clone(),
        }))
        .expect("job worker schedule receipt show");
        exercise_stale_job_worker_schedule(setup, sync, admission, worker_request);
    }
