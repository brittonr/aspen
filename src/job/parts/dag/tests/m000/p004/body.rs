
    fn assert_missing_artifact(fixture: &WorkerFixture) {
        let denied = execute_worker_delivery(JobWorkerExecuteInput {
            target_registry: &fixture.root.join("empty-target"),
            storage_root: &fixture.root.join("missing-artifact-storage"),
            cache_root: &fixture.root.join("missing-artifact-cache"),
            chunk_root: &fixture.root.join("missing-artifact-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("missing target artifact denial");
        assert_eq!(denied.result.decision, "deny", "{:?}", denied.result.diagnostics);
        assert!(
            denied.execution.as_ref().is_some_and(|execution| execution.run.is_none()),
            "{:?}",
            denied.result.diagnostics
        );
    }

    #[test]
    fn live_unrecorded_worker_run_is_diagnostic_only() {
        let fixture = worker_fixture("job-worker-live");
        let live = live_unrecorded_worker_result(JobWorkerExecuteInput {
            target_registry: &fixture.target,
            storage_root: &fixture.root.join("live-storage"),
            cache_root: &fixture.root.join("live-cache"),
            chunk_root: &fixture.root.join("live-chunks"),
            delivery: &fixture.delivery,
            delivery_log: Some(&fixture.delivery_log),
            admission_receipt_value: &fixture.admission.receipt_value,
            execution_request_value: &fixture.execution_request,
            ledger_root: None,
        })
        .expect("live diagnostic worker");
        assert_eq!(live.result.decision, "non-replayable", "{:?}", live.result.diagnostics);
        assert!(live.execution.as_ref().is_some_and(|execution| execution.decision == "pass"));
        assert!(live.result.diagnostics.iter().any(|diagnostic| diagnostic.contains("delivery log")));
    }

    #[hegel::test(test_cases = 4)]
    fn hegel_worker_request_identity_recorded_replay_and_no_source_state(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(10_000));
        let job_ref = test_ref(&format!("worker-job-{salt}"));
        let sync_ref = test_ref(&format!("worker-sync-{salt}"));
        let admission_ref = test_ref(&format!("worker-admission-{salt}"));
        let execution_request_ref = test_ref(&format!("worker-exec-{salt}"));
        let authority_ref = test_ref(&format!("worker-authority-{salt}"));
        let resource_ref = test_ref(&format!("worker-resource-{salt}"));
        let peer_bootstrap_ref = test_ref(&format!("worker-bootstrap-{salt}"));
        let identity_ref = test_ref(&format!("worker-node-{salt}"));
        let evidence = vec![sync_ref.clone(), admission_ref.clone(), execution_request_ref.clone()];
        let first = job_worker_request_value(JobWorkerRequestValueInput {
            job_ref: &job_ref,
            target_peer: "peer:b",
            stage_ids: &["stage".to_string()],
            sync_ref: &sync_ref,
            admission_ref: &admission_ref,
            execution_request_ref: &execution_request_ref,
            authority_refs: std::slice::from_ref(&authority_ref),
            resource_refs: std::slice::from_ref(&resource_ref),
            peer_bootstrap_refs: std::slice::from_ref(&peer_bootstrap_ref),
            node_identity_refs: std::slice::from_ref(&identity_ref),
            evidence_refs: &evidence,
        })
        .expect("first request");
        let second = crate::preserves_rail::parse_text(&crate::preserves_rail::to_text(&first).expect("request text"))
            .expect("reparse request");
        let parsed = parse_job_worker_request_value(&second).expect("parse worker request");
        assert_eq!(crate::preserves_rail::canonical_hash(&first).expect("first ref"), parsed.request_ref);
        let request_text = crate::preserves_rail::to_text(&first).expect("worker request text");
        assert!(!request_text.contains("source-registry"));
        assert!(request_text.contains("target-state-only"));
        assert!(request_text.contains(&sync_ref));
    }

    #[hegel::test(test_cases = 4)]
    fn hegel_blob_ref_submission_rejects_inline_tokens_and_records_pin_lifecycle(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(10_000));
        let token_selector = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(2));
        let token = match token_selector {
            0 => "inline-bytes",
            1 => "inline-executable",
            _ => "inline-dataset",
        };
        let operation_id = test_ref(&format!("job-ref-hegel-operation-{salt}"));
        let authority_ref = test_ref(&format!("job-ref-hegel-authority-{salt}"));
        let inline_value = crate::preserves_rail::record("job-ref-submission-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::JOB_REF_SUBMISSION_SCHEMA),
            crate::preserves_rail::record("job-id", vec![crate::preserves_rail::string(format!(
                "job-ref-hegel-{salt}"
            ))]),
            crate::preserves_rail::record("operation-id", vec![crate::preserves_rail::string(&operation_id)]),
            crate::preserves_rail::record("executable", vec![crate::preserves_rail::record(token, vec![
                crate::preserves_rail::string("bytes"),
            ])]),
            crate::preserves_rail::record("inputs", vec![crate::preserves_rail::sequence(vec![])]),
            crate::preserves_rail::record("output-mode", vec![crate::preserves_rail::string("chunk-manifest")]),
            crate::preserves_rail::record("input-schemas", vec![refs_sequence(&[])]),
            crate::preserves_rail::record("output-schemas", vec![refs_sequence(&[])]),
            crate::preserves_rail::record("effects", vec![refs_sequence(&[])]),
            crate::preserves_rail::record("handler-profile", vec![crate::preserves_rail::string("local-echo-v1")]),
            crate::preserves_rail::record("authority", vec![crate::preserves_rail::string(&authority_ref)]),
            crate::preserves_rail::record("policy", vec![refs_sequence(&[])]),
            crate::preserves_rail::record("provenance", vec![refs_sequence(&[])]),
            crate::preserves_rail::record("evidence", vec![refs_sequence(&[])]),
            checks_value(&["content-refs-only", "no-inline-large-bytes"]),
        ]);
        assert!(parse_job_ref_submission_value(&inline_value).is_err());

        let root = temp_dir(&format!("job-ref-hegel-{salt}"));
        let chunks = root.join("chunks");
        let executable = crate::chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let input_bytes = format!("input-{salt}");
        let input =
            crate::chunk_store::put_bytes(&chunks, "job-input", input_bytes.as_bytes(), DEFAULT_FIXED_V1_CHUNK_SIZE)
                .expect("put input");
        let policy_ref = test_ref(&format!("job-ref-hegel-policy-{salt}"));
        let provenance_ref = test_ref(&format!("job-ref-hegel-provenance-{salt}"));
        let effect_ref = test_ref(&format!("job-ref-hegel-effect-{salt}"));
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: &format!("job-ref-hegel-{salt}"),
            operation_id: &operation_id,
            executable: JobContentRef {
                content_ref: executable.manifest_ref.clone(),
                size: executable.total_len,
                format: "elf-executable".to_string(),
                schema_ref: None,
            },
            inputs: vec![JobContentRef {
                content_ref: input.manifest_ref.clone(),
                size: input.total_len,
                format: "bytes".to_string(),
                schema_ref: None,
            }],
            output_mode: "chunk-manifest",
            input_schema_refs: &[],
            output_schema_refs: &[],
            effect_manifest_refs: std::slice::from_ref(&effect_ref),
            handler_profile: "local-echo-v1",
            context_ref: &authority_ref,
            policy_refs: std::slice::from_ref(&policy_ref),
            provenance_refs: std::slice::from_ref(&provenance_ref),
            evidence_refs: &[],
        })
        .expect("submission");
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: None,
        })
        .expect("execute");
        assert_eq!(executed.decision, "pass");
        let receipt_text = crate::preserves_rail::to_text(&executed.receipt_value).expect("receipt text");
        assert!(receipt_text.contains("content-verification-before-run"));
        assert!(receipt_text.contains("retention-pins"));
        assert!(receipt_text.contains("cleanup-receipts"));
    }

    #[test]
    fn remote_admission_denies_missing_targets_and_non_artifact_stages() {
        let root = temp_dir("job-admit-deny");
        let registry = root.join("registry");
        let dag_value = pipeline_value().expect("dag");
        let installed = install_job_dag(&registry, &dag_value).expect("install dag");
        let sync_ref = test_ref("sync-receipt");
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &installed.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("policy")],
            capability_refs: &[test_ref("capability")],
            evidence_refs: &[sync_ref.clone(), test_ref("octet-gate")],
            resource_refs: &[
                test_ref("resource-a"),
                test_ref("resource-b"),
                test_ref("resource-c"),
                test_ref("resource-d"),
            ],
        })
        .expect("request");
        let denied = admission_plan_value(&registry, &request).expect("admission denial");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("artifact-backed executable")));

        let missing = admission_plan_value(&root.join("empty-target"), &request).expect("missing target denial");
        assert_eq!(missing.decision, "deny");
        assert!(missing.diagnostics.iter().any(|diagnostic| diagnostic.contains("target job not available")));
    }

    #[test]
    fn planning_profile_and_fusion_preview_are_canonical_and_conservative() {
        let root = temp_dir("job-planning");
        let dag_value = pipeline_value().expect("dag");
        let dag = parse_job_dag_value(&dag_value).expect("parse dag");
        let plan = plan_job_dag(&dag, None).expect("plan");
        assert_eq!(plan.stage_order.first(), Some(&"source".to_string()));
        assert!(crate::preserves_rail::to_text(&plan.value).expect("plan text").contains("trellis-topo-order"));
        let profile = profile_job_dag(&dag, None, Some(&root.join("cache"))).expect("profile");
        assert_eq!(profile.stage_count, 4);
        assert!(crate::preserves_rail::to_text(&profile.value).expect("profile text").contains("no-wall-clock-time"));
        let fusion = fusion_preview_job_dag(&dag, None).expect("fusion");
        assert!(fusion.chains.iter().any(|chain| chain == &vec!["filter".to_string(), "map".to_string()]));

        let policy_ref = test_ref("fusion-policy");
        let left = job_node_value(NodeValueInput {
            id: "left",
            kind: "map",
            stage_artifact_ref: None,
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: &[],
        })
        .expect("left");
        let right = test_node_value(
            "right",
            "filter",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string("keep-all")]),
        )
        .expect("right");
        let edge = stream_edge_value("left", "right").expect("edge");
        let boundary = test_dag_value(vec![left, right], vec![edge], &["right".to_string()]).expect("boundary dag");
        let boundary = parse_job_dag_value(&boundary).expect("boundary parse");
        assert!(fusion_preview_job_dag(&boundary, None).expect("boundary fusion").chains.is_empty());
    }

    #[test]
    fn planning_and_profile_reject_output_requests_for_missing_roots() {
        let root = temp_dir("job-output-root-missing");
        let dag_value = pipeline_value().expect("dag");
        let dag = parse_job_dag_value(&dag_value).expect("parse dag");
        let missing_roots = vec!["missing".to_string()];
        let request = job_output_request_value(OutputRequestValueInput {
            dag_ref: &dag.job_ref,
            roots: &missing_roots,
            materialization: "inline",
            policy_refs: &[],
            handler_profile_ref: None,
            seed_config_ref: None,
        })
        .expect("syntactically valid output request");

        let plan_error = plan_job_dag(&dag, Some(&request)).expect_err("plan rejects missing output root");
        assert!(plan_error.to_string().contains("job output root missing is not a node"));
        let profile_error = profile_job_dag(&dag, Some(&request), Some(&root.join("cache")))
            .expect_err("profile rejects missing output root");
        assert!(profile_error.to_string().contains("job output root missing is not a node"));
        let run_error = run_job_dag(&dag, &JobRunOptions {
            registry_root: &root.join("registry"),
            storage_root: &root.join("storage"),
            cache_root: &root.join("cache"),
            chunk_root: &root.join("chunks"),
            ledger_root: None,
            output_request: Some(request),
        })
        .expect_err("run rejects missing output root before stage execution");
        assert!(run_error.to_string().contains("job output root missing is not a node"));
    }

    #[test]
    fn raw_closure_config_denies_before_execution() {
        // r[verify molten.preserves_value_inspection.ambient_token_denial]
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("ok")]),
            ])]),
        )
        .expect("source");
        let bad = test_node_value(
            "bad",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("host-path", vec![crate::preserves_rail::string("/bin/echo")]),
        );
        assert!(bad.expect_err("bad config").to_string().contains("mobile/ambient"));
        let edge = stream_edge_value("source", "bad").expect("edge");
        let bad_node = crate::preserves_rail::record("job-node-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::JOB_DAG_NODE_SCHEMA),
            crate::preserves_rail::record("id", vec![crate::preserves_rail::string("bad")]),
            crate::preserves_rail::record("kind", vec![crate::preserves_rail::string("map")]),
            crate::preserves_rail::record("stage-artifact", vec![crate::preserves_rail::record("none", Vec::new())]),
            crate::preserves_rail::record("inputs", vec![ports_sequence(&["in".to_string()])]),
            crate::preserves_rail::record("outputs", vec![ports_sequence(&["out".to_string()])]),
            crate::preserves_rail::record("config", vec![crate::preserves_rail::record("host-path", vec![
                crate::preserves_rail::string("/bin/echo"),
            ])]),
            crate::preserves_rail::record("effects", vec![crate::preserves_rail::sequence(Vec::new())]),
            crate::preserves_rail::record("policy", vec![crate::preserves_rail::sequence(Vec::new())]),
            crate::preserves_rail::record("evidence", vec![crate::preserves_rail::sequence(Vec::new())]),
            checks_value(&["stage-artifact-not-closure"]),
        ]);
        let dag = test_dag_value(vec![source, bad_node], vec![edge], &["bad".to_string()]).expect("dag");
        assert!(parse_job_dag_value(&dag).expect_err("parse rejects").to_string().contains("mobile/ambient"));
    }

    #[test]
    fn rendered_ambient_looking_string_is_not_a_structural_token() {
        let node = test_node_value(
            "string-only",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::string("<host-path \"diagnostic-looking\">")
        )
        .expect("rendered-looking string is inert");
        let text = crate::preserves_rail::to_text(&node).expect("node text");
        assert!(text.contains("host-path"));
    }

    #[hegel::test(test_cases = 10)]
    fn hegel_dag_hash_and_memo_key_are_stable(tc: hegel::TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dag = fixture_value(if salt.is_multiple_of(2) { "identity" } else { "count" });
        let first = parse_job_dag_value(&dag).expect("first");
        let second = parse_job_dag_value(
            &crate::preserves_rail::parse_text(&crate::preserves_rail::to_text(&dag).expect("text"))
                .expect("parse text"),
        )
        .expect("second");
        assert_eq!(first.job_ref, second.job_ref);
        assert_eq!(
            execution_order(&first.nodes, &first.edges).expect("order"),
            execution_order(&second.nodes, &second.edges).expect("order")
        );
    }
