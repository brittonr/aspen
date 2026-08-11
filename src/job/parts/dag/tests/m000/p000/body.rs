    use super::*;

    fn test_node_value(
        id: &str,
        kind: &str,
        input_ports: &[String],
        output_ports: &[String],
        config: IoValue,
    ) -> Result<IoValue> {
        job_node_value(NodeValueInput {
            id,
            kind,
            stage_artifact_ref: None,
            input_ports,
            output_ports,
            config,
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
    }

    fn stream_edge_value(from_node: &str, to_node: &str) -> Result<IoValue> {
        job_edge_value(EdgeValueInput {
            from_node,
            from_port: "out",
            to_node,
            to_port: "in",
            schema_ref: None,
            partitioning: "single",
            materialization: "stream",
        })
    }

    fn test_dag_value(nodes: Vec<IoValue>, edges: Vec<IoValue>, output_roots: &[String]) -> Result<IoValue> {
        job_dag_value(DagValueInput {
            nodes,
            edges,
            output_roots,
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
    }

    #[test]
    fn dag_identity_is_stable_and_ignores_names() {
        let dag = fixture_value("identity");
        let parsed = parse_job_dag_value(&dag).expect("parse dag");
        let reparsed = parse_job_dag_value(
            &crate::preserves_rail::parse_text(&crate::preserves_rail::to_text(&dag).expect("text"))
                .expect("text parse"),
        )
        .expect("reparse");
        assert_eq!(parsed.job_ref, reparsed.job_ref);
        let changed = fixture_value("count");
        let changed = parse_job_dag_value(&changed).expect("changed parse");
        assert_ne!(parsed.job_ref, changed.job_ref);
    }

    #[test]
    fn local_pipeline_runs_and_memoizes() {
        let root = temp_dir("job-pipeline");
        let registry = root.join("registry");
        let storage = root.join("storage");
        let cache = root.join("cache");
        let chunks = root.join("chunks");
        let ledger = root.join("ledger");
        let dag_value = pipeline_value().expect("dag");
        let install = install_job_dag(&registry, &dag_value).expect("install");
        assert_eq!(install.decision, "pass");
        let dag = read_job_dag(&registry, &install.job_ref).expect("read dag");
        let options = JobRunOptions {
            registry_root: &registry,
            storage_root: &storage,
            cache_root: &cache,
            chunk_root: &chunks,
            ledger_root: Some(&ledger),
            output_request: None,
        };
        let first = run_job_dag(&dag, &options).expect("first run");
        let second = run_job_dag(&dag, &options).expect("second run");
        assert_eq!(first.output_refs, second.output_refs);
        let second_text = crate::preserves_rail::to_text(&second.receipt_value).expect("receipt text");
        assert!(["memo-hit", "stage-receipts-bound"].iter().any(|needle| second_text.contains(needle)));
        let output_text = crate::preserves_rail::to_text(&second.output_value).expect("output text");
        assert!(output_text.contains("wrapped"));
    }

    #[test]
    // r[verify molten.blob_ref_jobs.payload_model]
    // r[verify molten.blob_ref_jobs.local_worker]
    // r[verify molten.blob_ref_jobs.content_verification]
    // r[verify molten.blob_ref_jobs.provenance_policy]
    // r[verify molten.blob_ref_jobs.retention_pins]
    // r[verify molten.blob_ref_jobs.receipts]
    // r[verify molten.blob_ref_jobs.local_tests]
    fn blob_ref_job_submission_worker_verifies_and_outputs_manifest() {
        let root = temp_dir("job-ref-worker");
        let chunks = root.join("chunks");
        let ledger = root.join("ledger");
        let executable = crate::chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let input = crate::chunk_store::put_bytes(&chunks, "job-input", b"hello", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put input");
        let operation_id = local_ref("job-ref-operation", "one").expect("operation id");
        let policy_ref = local_ref("job-ref-policy", "one").expect("policy ref");
        let provenance_ref = local_ref("job-ref-provenance", "one").expect("provenance ref");
        let effect_ref = local_ref("job-ref-effect", "one").expect("effect ref");
        let authority_ref = local_ref("job-ref-authority", "one").expect("authority ref");
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: "job-ref-worker",
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
        .expect("submission value");
        let parsed_submission = parse_job_ref_submission_value(&submission_value).expect("parse submission");
        assert_eq!(parsed_submission.inputs.len(), 1);
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: Some(&ledger),
        })
        .expect("execute blob ref job");
        assert_eq!(executed.decision, "pass");
        let output_ref = executed.output_manifest_ref.as_deref().expect("output ref");
        let output = crate::chunk_store::read_object(&chunks, output_ref).expect("read output");
        assert_eq!(output.bytes, b"hello");
        let receipt = parse_blob_ref_job_receipt_value(&executed.receipt_value).expect("parse receipt");
        assert_eq!(receipt.decision, "pass");
        assert_eq!(receipt.output_refs, vec![output_ref.to_string()]);
        assert!(
            crate::ledger::list_artifacts(&ledger)
                .expect("ledger artifacts")
                .iter()
                .any(|artifact| artifact.artifact_kind == "job-ref-receipt")
        );
    }

    #[test]
    fn blob_ref_job_submission_denies_missing_ref_before_run() {
        let root = temp_dir("job-ref-missing");
        let chunks = root.join("chunks");
        let executable = crate::chunk_store::put_bytes(&chunks, "job-executable", b"echo", DEFAULT_FIXED_V1_CHUNK_SIZE)
            .expect("put executable");
        let operation_id = local_ref("job-ref-operation", "missing").expect("operation id");
        let policy_ref = local_ref("job-ref-policy", "missing").expect("policy ref");
        let provenance_ref = local_ref("job-ref-provenance", "missing").expect("provenance ref");
        let effect_ref = local_ref("job-ref-effect", "missing").expect("effect ref");
        let authority_ref = local_ref("job-ref-authority", "missing").expect("authority ref");
        let missing_ref = local_ref("job-ref-missing-input", "missing").expect("missing input ref");
        let submission_value = job_ref_submission_value(BlobRefJobSubmissionValueInput {
            job_id: "job-ref-missing",
            operation_id: &operation_id,
            executable: JobContentRef {
                content_ref: executable.manifest_ref.clone(),
                size: executable.total_len,
                format: "elf-executable".to_string(),
                schema_ref: None,
            },
            inputs: vec![JobContentRef {
                content_ref: missing_ref,
                size: 5,
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
        .expect("submission value");
        let executed = execute_blob_ref_job(BlobRefJobExecuteInput {
            chunk_root: &chunks,
            submission_value: &submission_value,
            ledger_root: None,
        })
        .expect("deny execution still emits receipt");
        assert_eq!(executed.decision, "deny");
        assert!(executed.output_manifest_ref.is_none());
        assert!(!executed.diagnostics.is_empty());
        let receipt = parse_blob_ref_job_receipt_value(&executed.receipt_value).expect("parse deny receipt");
        assert_eq!(receipt.decision, "deny");
    }

    #[test]
    fn blob_ref_job_submission_rejects_malformed_content_refs() {
        let operation_id = local_ref("job-ref-operation", "malformed").expect("operation id");
        let authority_ref = local_ref("job-ref-authority", "malformed").expect("authority ref");
        for invalid in [
            "blake3:fixture",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
        ] {
            let error = job_ref_submission_value(BlobRefJobSubmissionValueInput {
                job_id: "job-ref-malformed",
                operation_id: &operation_id,
                executable: JobContentRef {
                    content_ref: invalid.to_string(),
                    size: 4,
                    format: "elf-executable".to_string(),
                    schema_ref: None,
                },
                inputs: Vec::new(),
                output_mode: "chunk-manifest",
                input_schema_refs: &[],
                output_schema_refs: &[],
                effect_manifest_refs: &[],
                handler_profile: "local-echo-v1",
                context_ref: &authority_ref,
                policy_refs: &[],
                provenance_refs: &[],
                evidence_refs: &[],
            })
            .expect_err("malformed executable ref denied");
            assert!(error.to_string().contains("canonical blake3 content ref"), "unexpected error: {error}");
        }
    }

    #[test]
    // r[verify molten.blob_ref_jobs.no_inline_large_bytes]
    fn blob_ref_job_submission_rejects_inline_large_bytes() {
        let operation_id = local_ref("job-ref-operation", "inline").expect("operation id");
        let authority_ref = local_ref("job-ref-authority", "inline").expect("authority ref");
        let value = crate::preserves_rail::record("job-ref-submission-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::JOB_REF_SUBMISSION_SCHEMA),
            crate::preserves_rail::record("job-id", vec![crate::preserves_rail::string("job-ref-inline")]),
            crate::preserves_rail::record("operation-id", vec![crate::preserves_rail::string(&operation_id)]),
            crate::preserves_rail::record("executable", vec![crate::preserves_rail::record("inline-bytes", vec![
                crate::preserves_rail::string("not-a-content-ref"),
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
        assert!(
            parse_job_ref_submission_value(&value)
                .expect_err("inline bytes rejected")
                .to_string()
                .contains("inline")
        );
    }
