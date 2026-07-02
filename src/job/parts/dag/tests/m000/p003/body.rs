
    #[test]
    fn iroh_worker_recorded_loopback_executes_and_imports_results() {
        let fixture = worker_fixture("job-worker-pass");
        let worker = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("worker-storage"),
            cache_root: &fixture.root.join("worker-cache"),
            chunk_root: &fixture.root.join("worker-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: Some(&fixture.ledger),
        })
        .expect("worker execute");
        assert_eq!(worker.result.decision, "pass", "{:?}", worker.result.diagnostics);
        assert!(worker.execution.as_ref().expect("execution").run.is_some());
        let source_run = run_job_dag(
            &read_job_dag(&fixture.source, &fixture.installed_job.job_ref).expect("source job"),
            &JobRunOptions {
                registry_root: &fixture.source,
                storage_root: &fixture.root.join("source-storage"),
                cache_root: &fixture.root.join("source-cache"),
                chunk_root: &fixture.root.join("source-chunks"),
                ledger_root: None,
                output_request: None,
            },
        )
        .expect("source equivalent run");
        assert_eq!(
            worker.execution.as_ref().expect("execution").run.as_ref().expect("run").output_value,
            source_run.output_value
        );
        assert_eq!(crate::ledger::artifact_kind(&fixture.worker_request), "job-worker-request");
        assert_eq!(crate::ledger::artifact_kind(&worker.result.value), "job-worker-result");
        assert_eq!(crate::ledger::artifact_kind(&worker.receipt_value), "job-worker-receipt");
        let kinds = crate::ledger::list_artifacts(&fixture.ledger)
            .expect("worker ledger")
            .into_iter()
            .map(|entry| entry.artifact_kind)
            .collect::<OrderedSet<_>>();
        assert!(kinds.contains("job-worker-assignment"));
        assert!(kinds.contains("job-worker-status"));
        assert!(kinds.contains("job-worker-result"));
        assert!(kinds.contains("job-worker-receipt"));
        let receipt_text = crate::preserves_rail::to_text(&worker.receipt_value).expect("worker receipt text");
        let result_text = crate::preserves_rail::to_text(&worker.result.value).expect("worker result text");
        assert!(receipt_text.contains("transport-is-not-authority"));
        assert!(receipt_text.contains("recorded-delivery-log"));
        assert!(result_text.contains(&fixture.sync_ref));
        assert!(result_text.contains(&fixture.execution_request_ref));
    }

    #[test]
    fn worker_denies_missing_authority_stale_sync_target_mismatch_and_missing_artifact() {
        let fixture = worker_fixture("job-worker-deny");

        assert_missing_authority(&fixture);
        assert_missing_admission(&fixture);
        assert_denied_admission(&fixture);
        assert_stale_sync(&fixture);
        assert_target_mismatch(&fixture);
        assert_missing_artifact(&fixture);
    }

    fn assert_missing_authority(fixture: &WorkerFixture) {
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &fixture.admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: &[],
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.identity_ref),
            evidence_refs: &fixture.evidence_refs,
        })
        .expect("missing authority worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("missing-authority-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("missing-authority-storage"),
            cache_root: &fixture.root.join("missing-authority-cache"),
            chunk_root: &fixture.root.join("missing-authority-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing authority denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("authority")));
    }

    fn assert_missing_admission(fixture: &WorkerFixture) {
        let missing_admission_ref = test_ref("missing-worker-admission");
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &missing_admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: std::slice::from_ref(&fixture.context_ref),
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.identity_ref),
            evidence_refs: &[
                fixture.sync_ref.clone(),
                missing_admission_ref.clone(),
                fixture.execution_request_ref.clone(),
            ],
        })
        .expect("missing admission worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("missing-admission-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("missing-admission-storage"),
            cache_root: &fixture.root.join("missing-admission-cache"),
            chunk_root: &fixture.root.join("missing-admission-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing admission denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("admission receipt hashes")));
    }

    fn assert_denied_admission(fixture: &WorkerFixture) {
        let request_value = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            sync_ref: &fixture.sync_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[],
            capability_refs: &[],
            evidence_refs: &[],
            resource_refs: &[],
        })
        .expect("denied admission request");
        let admission = admission_loopback(&fixture.target, &request_value).expect("denied admission");
        assert_eq!(admission.plan.decision, "deny");
        let admission_ref =
            crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("denied admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:b",
            storage_profile_ref: &test_ref("denied-worker-storage-profile"),
            cache_profile_ref: &test_ref("denied-worker-cache-profile"),
            chunk_profile_ref: &test_ref("denied-worker-chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("denied execution request");
        let execution_request_ref =
            crate::preserves_rail::canonical_hash(&execution_request).expect("denied execution ref");
        let worker_request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &admission.plan.stage_order,
            sync_ref: &fixture.sync_ref,
            admission_ref: &admission_ref,
            execution_request_ref: &execution_request_ref,
            authority_refs: &[],
            resource_refs: &[],
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.identity_ref),
            evidence_refs: &[
                fixture.sync_ref.clone(),
                admission_ref.clone(),
                execution_request_ref.clone(),
            ],
        })
        .expect("denied worker request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("denied-admission-transport"), &worker_request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("denied-admission-storage"),
            cache_root: &fixture.root.join("denied-admission-cache"),
            chunk_root: &fixture.root.join("denied-admission-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &admission.receipt_value,
            execution_request_value: &execution_request,
            ledger_root: None,
        })
        .expect("denied worker");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
    }

    fn assert_stale_sync(fixture: &WorkerFixture) {
        let stale_sync = test_ref("stale-sync");
        let request = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &fixture.installed_job.job_ref,
            target_peer: "peer:b",
            stage_ids: &fixture.admission.plan.stage_order,
            sync_ref: &stale_sync,
            admission_ref: &fixture.admission_ref,
            execution_request_ref: &fixture.execution_request_ref,
            authority_refs: std::slice::from_ref(&fixture.context_ref),
            resource_refs: &fixture.resource_refs,
            peer_bootstrap_refs: std::slice::from_ref(&fixture.peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&fixture.identity_ref),
            evidence_refs: &[
                stale_sync.clone(),
                fixture.admission_ref.clone(),
                fixture.execution_request_ref.clone(),
            ],
        })
        .expect("stale sync request");
        let (delivery, log) =
            deliver_worker_request(&fixture.root.join("stale-sync-transport"), &request, "peer:b", true);
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("stale-storage"),
            cache_root: &fixture.root.join("stale-cache"),
            chunk_root: &fixture.root.join("stale-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("stale denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
        assert!(denied.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("sync ref")));
    }

    fn assert_target_mismatch(fixture: &WorkerFixture) {
        let envelope = crate::remote_dataspace::build_envelope(crate::remote_dataspace::EnvelopeInput {
            from_peer: "peer:a".to_string(),
            from_actor: "source-worker".to_string(),
            to_peer: "peer:c".to_string(),
            topic: "molten.job.worker".to_string(),
            operation: crate::remote_dataspace::Operation::Message,
            payload: fixture.worker_request.clone(),
            content_refs: Vec::new(),
            capability_refs: vec![fixture.context_ref.clone()],
            evidence_refs: fixture.evidence_refs.clone(),
        })
        .expect("target mismatch envelope");
        crate::remote_dataspace::publish_local_gossip(
            &fixture.root.join("target-mismatch-transport"),
            &envelope,
            "peer:a",
        )
        .expect("publish mismatch");
        let delivery = crate::remote_dataspace::deliver_local_gossip(
            &fixture.root.join("target-mismatch-transport"),
            "molten.job.worker",
            &envelope.envelope_ref,
            "peer:c",
        )
        .expect("deliver mismatch");
        let log = crate::remote_dataspace::delivery_log(std::slice::from_ref(&delivery), true)
            .expect("mismatch delivery log");
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("target-mismatch-storage"),
            cache_root: &fixture.root.join("target-mismatch-cache"),
            chunk_root: &fixture.root.join("target-mismatch-chunks"),
            delivery: &delivery,
            delivery_log: Some(&log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("target mismatch denial");
        assert_eq!(denied.result.decision, "deny");
        assert!(denied.execution.is_none());
    }
