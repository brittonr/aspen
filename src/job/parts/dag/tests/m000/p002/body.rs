
    fn passing_flow(case: &CopyCase, sync_ref: String) -> CopyFlow {
        let context_ref = install_job_execute_authority_context(&case.target, &case.installed_job.job_ref);
        let source_gate_ref = install_clean_octet_gate(&case.target);
        let admission_request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&context_ref),
            evidence_refs: &[sync_ref.clone(), source_gate_ref.clone()],
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("admission request");
        let admission = admission_loopback(&case.target, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        assert!(
            crate::preserves_rail::to_text(&admission.receipt_value)
                .expect("admission receipt")
                .contains("no-execution")
        );
        let admission_ref = crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("admission ref");
        CopyFlow {
            sync_ref,
            source_gate_ref,
            context_ref,
            admission,
            admission_ref,
        }
    }

    fn passing_execution(case: &CopyCase, flow: &CopyFlow) -> (IoValue, JobExecutionLoopback) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.context_ref),
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("execution request");
        let execution = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage"),
            cache_root: &case.root.join("cache"),
            chunk_root: &case.root.join("chunks"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("execution loopback");
        (request, execution)
    }

    fn assert_target_run(case: &CopyCase, flow: &CopyFlow) {
        let (request, execution) = passing_execution(case, flow);
        assert_eq!(execution.decision, "pass");
        assert!(
            crate::preserves_rail::to_text(&execution.run.as_ref().expect("run").output_value)
                .expect("execution output")
                .contains("x")
        );
        let equivalent_source_run = run_job_dag(
            &read_job_dag(&case.source, &case.installed_job.job_ref).expect("source job"),
            &JobRunOptions {
                registry_root: &case.source,
                storage_root: &case.root.join("source-storage"),
                cache_root: &case.root.join("source-cache"),
                chunk_root: &case.root.join("source-chunks"),
                ledger_root: None,
                output_request: None,
            },
        )
        .expect("equivalent source run");
        assert_eq!(execution.run.as_ref().expect("execution run").output_value, equivalent_source_run.output_value);
        assert_eq!(crate::ledger::artifact_kind(&request), "job-execution-request");
        assert_eq!(crate::ledger::artifact_kind(&execution.receipt_value), "job-execution-receipt");
        let execution_text = crate::preserves_rail::to_text(&execution.receipt_value).expect("execution receipt");
        assert!(execution_text.contains("job-execution-receipt-v1"));
        assert!(execution_text.contains("executed-on-target-state"));
        assert!(execution_text.contains(&flow.admission.plan.request.sync_ref));
        assert!(execution_text.contains(&flow.admission.plan.authority_receipt_refs[0]));
        assert!(execution_text.contains(&test_ref("resource-1")));
        assert!(execution_text.contains(&execution.run.as_ref().expect("execution run refs").stage_receipt_refs[0]));
        assert!(execution_text.contains(&execution.run.as_ref().expect("execution output refs").output_refs[0]));
    }

    fn assert_wrong_peer_execution(case: &CopyCase, flow: &CopyFlow) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:other",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.context_ref),
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("wrong peer request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-deny"),
            cache_root: &case.root.join("cache-deny"),
            chunk_root: &case.root.join("chunks-deny"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("denied execution receipt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.run.is_none());
        assert!(
            crate::preserves_rail::to_text(&denied.receipt_value)
                .expect("denied execution receipt")
                .contains("no-stage-execution-on-deny")
        );
    }

    fn assert_other_reference_denial(case: &CopyCase, flow: &CopyFlow) {
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &test_ref("other-job"),
            admission_ref: &flow.admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("wrong job request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-wrong-job"),
            cache_root: &case.root.join("cache-wrong-job"),
            chunk_root: &case.root.join("chunks-wrong-job"),
            admission_receipt_value: &flow.admission.receipt_value,
            request_value: &request,
        })
        .expect("wrong job execution denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match admission job")));
    }

    fn assert_stale_closure_execution(case: &CopyCase, flow: &CopyFlow) {
        let stale_ref = test_ref("stale-stage-artifact");
        let tampered_admission = crate::preserves_rail::parse_text(
            &crate::preserves_rail::to_text(&flow.admission.receipt_value).expect("admission text").replacen(
                &case.stage.artifact_ref,
                &stale_ref,
                1,
            ),
        )
        .expect("tampered admission parse");
        let tampered_admission_ref =
            crate::preserves_rail::canonical_hash(&tampered_admission).expect("tampered admission ref");
        let request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &tampered_admission_ref,
            stage_ids: &flow.admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("stale closure request");
        let denied = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-stale"),
            cache_root: &case.root.join("cache-stale"),
            chunk_root: &case.root.join("chunks-stale"),
            admission_receipt_value: &tampered_admission,
            request_value: &request,
        })
        .expect("stale closure denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("closure diverges")));
    }

    fn assert_missing_admission_inputs(case: &CopyCase, flow: &CopyFlow) {
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &flow.sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[],
            capability_refs: &[],
            evidence_refs: &[],
            resource_refs: &[],
        })
        .expect("missing authority request");
        let denied = admission_plan_value(&case.target, &request).expect("authority denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("policy")));
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("strict Octet source gate")));
        let admission = admission_loopback(&case.target, &request).expect("denied admission receipt");
        let admission_ref =
            crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("denied admission ref");
        let execution_request = job_execution_request_value(ExecutionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            admission_ref: &admission_ref,
            stage_ids: &admission.plan.stage_order,
            target_peer: "peer:loopback",
            storage_profile_ref: &test_ref("storage-profile"),
            cache_profile_ref: &test_ref("cache-profile"),
            chunk_profile_ref: &test_ref("chunk-profile"),
            policy_refs: &[],
            capability_refs: &[],
            resource_refs: &[],
        })
        .expect("denied execution request");
        let denied_by_admission = execution_loopback(ExecutionLoopbackInput {
            target_registry: &case.target,
            storage_root: &case.root.join("storage-denied-admission"),
            cache_root: &case.root.join("cache-denied-admission"),
            chunk_root: &case.root.join("chunks-denied-admission"),
            admission_receipt_value: &admission.receipt_value,
            request_value: &execution_request,
        })
        .expect("denied admission execution receipt");
        assert_eq!(denied_by_admission.decision, "deny");
        assert!(denied_by_admission.diagnostics.iter().any(|diagnostic| diagnostic.contains("admission decision")));
    }

    fn assert_unsatisfied_stage_denial(case: &CopyCase, flow: &CopyFlow) {
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &flow.sync_ref,
            stage_ids: &["map".to_string()],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&flow.context_ref),
            evidence_refs: &[flow.sync_ref.clone(), flow.source_gate_ref.clone()],
            resource_refs: &[test_ref("resource-1")],
        })
        .expect("unsatisfied request");
        let denied = admission_plan_value(&case.target, &request).expect("unsatisfied denial");
        assert_eq!(denied.decision, "deny");
        assert!(
            denied
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("unsatisfied selected-stage dependencies"))
        );
    }
