    struct JobSenderSource {
        sender: PathBuf,
        receiver: PathBuf,
        generation: u64,
    }

    impl JobSenderSource {
        fn new(setup: &JobSetup, request_path: &Path) -> Self {
            let sender = setup.dir.join("worker-sender-state");
            let receiver = setup.dir.join("worker-receiver-state");
            for (root, node_id) in [(&sender, "peer:source"), (&receiver, "peer:loopback")] {
                molten::node_daemon::init_local(&molten::node_daemon::InitInput { state_root: root, node_id })
                    .expect("initialize genuine worker node root");
            }
            let sender_root = molten::node_state::DirectoryRoot::open_existing(&sender).expect("sender capability");
            let receiver_root = molten::node_state::DirectoryRoot::open_existing(&receiver).expect("receiver capability");
            let owner = molten::remote_dataspace::RecordedSessionStore::open(&receiver_root).expect("receiver source owner");
            let admitted = owner.admit_local(&sender_root, "molten.job.worker").expect("admit actual local session");
            let request = molten::job_dag::parse_job_worker_request_value(
                &read_preserves_file(request_path).expect("actual worker request"),
            ).expect("parse worker request");
            // Introduce actual workload operands, not caller-carried proof or authority metadata.
            let introduction = molten::preserves_rail::record("job-worker-operands-v1", vec![
                molten::preserves_rail::string(&request.job_ref),
                molten::preserves_rail::string(&request.execution_request_ref),
            ]);
            owner.commit("peer:source", "molten.job.worker", admitted.generation,
                molten::remote_dataspace::Operation::Assert, &introduction).expect("receiver commits workload introduction");
            Self { sender, receiver, generation: admitted.generation }
        }

        fn args(&self) -> crate::cli_remote::session::SourceArgs {
            crate::cli_remote::session::SourceArgs {
                sender_state_root: Some(self.sender.clone()), receiver_state_root: Some(self.receiver.clone()),
                session_generation: Some(self.generation),
            }
        }
    }

    fn exercise_job_worker(setup: &JobSetup, sync: &JobSync, admission: &JobAdmission) {
        let worker_request = setup.dir.join("job-worker-request.preserves");
        let peer_bootstrap_ref = cli_synthetic_ref("job-worker-peer-bootstrap").expect("peer bootstrap");
        let identity_ref = cli_synthetic_ref("job-worker-node-identity").expect("node identity");
        run_job_command(JobCommand::WorkerRequest(cli_job::command::worker::Request {
            admission_receipt: admission.admit_loopback_receipt.clone(),
            execution_request: admission.worker_execution_request.clone(),
            sync_ref: Some(sync.sync_ref.clone()),
            target_peer: "peer:loopback".to_string(),
            stages: Vec::new(),
            authority_refs: vec![sync.context_ref.clone()],
            resource_refs: sync.worker_resource_refs.clone(),
            peer_bootstrap_refs: vec![peer_bootstrap_ref],
            node_identity_refs: vec![identity_ref],
            evidence_refs: Vec::new(),
            out: Some(worker_request.clone()),
        }))
        .expect("job worker request");
        let source = JobSenderSource::new(setup, &worker_request);
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
            source: source.args(),
            ledger: Some(setup.ledger_root.clone()),
            out: worker_out.clone(),
        }))
        .expect("job worker local run");
        let worker_receipt = read_preserves_file(&worker_out.join("worker-receipt.preserves")).expect("worker receipt");
        assert_eq!(molten::ledger::artifact_kind(&worker_receipt), "job-worker-receipt");
        assert!(fs::read_to_string(worker_out.join("output.preserves")).expect("worker output").contains("3"));
        let worker_receipt_ref = canonical_hash(&worker_receipt).expect("worker receipt ref");
        run_job_command(JobCommand::ReceiptShow(cli_job::command::refs::ReceiptShow {
            receipt_ref: worker_receipt_ref,
            ledger: setup.ledger_root.clone(),
        }))
        .expect("job worker receipt show");
        exercise_job_worker_schedule(setup, sync, admission, worker_request, &source);
    }

    fn exercise_job_worker_schedule(
        setup: &JobSetup,
        sync: &JobSync,
        admission: &JobAdmission,
        worker_request: PathBuf,
        source: &JobSenderSource,
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
            source: source.args(),
            coordination_authority_refs: vec![sync.context_ref.clone()],
            coordination_resource_refs: sync.worker_resource_refs.clone(),
            coordination_policy_refs: vec![cli_synthetic_ref("job-worker-schedule-policy").expect("schedule policy")],
            ledger: Some(setup.ledger_root.clone()),
            out: schedule_out.clone(),
        }))
        .expect("job worker scheduled local run");
        let schedule_receipt =
            read_preserves_file(&schedule_out.join("schedule-receipt.preserves")).expect("schedule receipt");
        assert_eq!(molten::ledger::artifact_kind(&schedule_receipt), "job-worker-schedule-receipt");
        assert!(
            fs::read_to_string(schedule_out.join("worker").join("output.preserves"))
                .expect("scheduled worker output")
                .contains("3")
        );
        let enqueue_value = read_preserves_file(&schedule_out.join("coordination").join("enqueue-receipt.preserves"))
            .expect("enqueue receipt");
        let enqueue_ref = canonical_hash(&enqueue_value).expect("enqueue ref");
        let duplicate_enqueue = molten::coordination::parse_coordination_receipt(
            &read_preserves_file(&schedule_out.join("coordination").join("enqueue-duplicate-receipt.preserves"))
                .expect("duplicate enqueue receipt"),
        )
        .expect("parse duplicate enqueue receipt");
        assert_eq!(duplicate_enqueue.transition_kind, COORDINATION_DUPLICATE_REPLAY_TRANSITION);
        assert_eq!(duplicate_enqueue.prior_receipt_ref.as_deref(), Some(enqueue_ref.as_str()));
        assert!(duplicate_enqueue.output_refs.iter().any(|output_ref| output_ref == &enqueue_ref));
        let schedule_receipt_ref = canonical_hash(&schedule_receipt).expect("schedule receipt ref");
        run_job_command(JobCommand::ReceiptShow(cli_job::command::refs::ReceiptShow {
            receipt_ref: schedule_receipt_ref,
            ledger: setup.ledger_root.clone(),
        }))
        .expect("job worker schedule receipt show");
        exercise_stale_job_worker_schedule(setup, sync, admission, worker_request, source);
    }
