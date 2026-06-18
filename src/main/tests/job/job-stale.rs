    fn exercise_stale_job_worker_schedule(
        setup: &JobSetup,
        sync: &JobSync,
        admission: &JobAdmission,
        worker_request: PathBuf,
    ) {
        let stale_schedule_out = setup.dir.join("job-worker-stale-schedule");
        run_job_command(JobCommand::WorkerScheduleLocal(cli_job::command::worker::ScheduleLocal {
            request: worker_request,
            target_registry: setup.target_registry.clone(),
            storage: setup.dir.join("stale-worker-storage"),
            cache: setup.dir.join("stale-worker-cache"),
            chunks: Some(setup.dir.join("stale-worker-chunks")),
            admission_receipt: admission.admit_loopback_receipt.clone(),
            execution_request: admission.worker_execution_request.clone(),
            transport_root: setup.dir.join("stale-worker-transport"),
            queue_key: "queue:job-worker".to_string(),
            lease_key: None,
            scheduler_session: "scheduler".to_string(),
            worker_session: "worker-a".to_string(),
            lease_token: Some(0),
            from_peer: "peer:source".to_string(),
            from_actor: "source-worker".to_string(),
            topic: "molten.job.worker".to_string(),
            coordination_authority_refs: vec![sync.authority_context_ref.clone()],
            coordination_resource_refs: sync.worker_resource_refs.clone(),
            coordination_policy_refs: vec![cli_synthetic_ref("job-worker-stale-policy").expect("stale policy")],
            ledger: None,
            out: stale_schedule_out.clone(),
        }))
        .expect_err("stale schedule token denies before worker");
        let stale_receipt = job_dag::parse_job_worker_schedule_receipt_value(
            &read_preserves_file(&stale_schedule_out.join("schedule-receipt.preserves")).expect("stale receipt"),
        )
        .expect("parse stale schedule receipt");
        assert_eq!(stale_receipt.decision, "deny");
        assert!(stale_receipt.diagnostics.join(";").contains("stale fencing token"));
        assert!(!stale_schedule_out.join("worker").join("worker-receipt.preserves").exists());
    }
