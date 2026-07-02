
    #[test]
    fn reduce_records_deterministic_output() {
        let root = temp_dir("job-reduce");
        let registry = root.join("registry");
        let storage = root.join("storage");
        let cache = root.join("cache");
        let chunks = root.join("chunks");
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![
                    crate::preserves_rail::u64_value(1),
                    crate::preserves_rail::u64_value(2),
                    crate::preserves_rail::u64_value(3),
                ]),
            ])]),
        )
        .expect("source");
        let reduce = test_node_value(
            "sum",
            "reduce",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string("sum-u64")]),
        )
        .expect("reduce");
        let edge = stream_edge_value("source", "sum").expect("edge");
        let dag_value = test_dag_value(vec![source, reduce], vec![edge], &["sum".to_string()]).expect("dag");
        let dag = parse_job_dag_value(&dag_value).expect("parse dag");
        let options = JobRunOptions {
            registry_root: &registry,
            storage_root: &storage,
            cache_root: &cache,
            chunk_root: &chunks,
            ledger_root: None,
            output_request: None,
        };
        let run = run_job_dag(&dag, &options).expect("run");
        assert_eq!(crate::preserves_rail::to_text(&run.output_value).expect("output"), "[6]");
    }

    #[test]
    fn trellis_topology_orders_by_canonical_indices_and_rejects_cycles() {
        let a = test_node_value(
            "a",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        )
        .expect("a");
        let b = test_node_value(
            "b",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        )
        .expect("b");
        let a_node = parse_job_node_value(&a).expect("parse a");
        let b_node = parse_job_node_value(&b).expect("parse b");
        let ordered = execution_order(&[b_node, a_node], &[]).expect("independent order");
        assert_eq!(ordered, vec!["a".to_string(), "b".to_string()]);
        let ab = stream_edge_value("a", "b").expect("ab");
        let ba = stream_edge_value("b", "a").expect("ba");
        let cyclic = test_dag_value(vec![a, b], vec![ab, ba], &["b".to_string()]).expect("cyclic dag value");
        assert!(parse_job_dag_value(&cyclic).expect_err("cycle rejected").to_string().contains("trellis"));
    }

    #[test]
    fn sync_plan_and_loopback_copy_dependency_closure_without_execution() {
        let case = copy_case();
        let sync_ref = assert_copy(&case);
        let flow = passing_flow(&case, sync_ref);
        assert_target_run(&case, &flow);
        assert_wrong_peer_execution(&case, &flow);
        assert_other_reference_denial(&case, &flow);
        assert_stale_closure_execution(&case, &flow);
        assert_missing_admission_inputs(&case, &flow);
        assert_unsatisfied_stage_denial(&case, &flow);
    }

    struct CopyArtifacts {
        base: crate::artifacts::ArtifactInstall,
        source_stage: crate::artifacts::ArtifactInstall,
        stage: crate::artifacts::ArtifactInstall,
    }

    struct CopyCase {
        root: std::path::PathBuf,
        source: std::path::PathBuf,
        target: std::path::PathBuf,
        base: crate::artifacts::ArtifactInstall,
        source_stage: crate::artifacts::ArtifactInstall,
        stage: crate::artifacts::ArtifactInstall,
        installed_job: JobInstall,
        request: IoValue,
    }

    struct CopyFlow {
        sync_ref: String,
        source_gate_ref: String,
        context_ref: String,
        admission: JobAdmissionLoopback,
        admission_ref: String,
    }

    fn install_case_artifact(
        registry: &FilePath,
        kind: &str,
        payload: IoValue,
        dependency_refs: Vec<String>,
        label: &str,
    ) -> crate::artifacts::ArtifactInstall {
        crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: kind.to_string(),
            payload,
            schema_refs: vec![test_ref("schema")],
            dependency_refs,
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("evidence")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect(label)
    }

    fn copy_artifacts(source: &FilePath) -> CopyArtifacts {
        let base = install_case_artifact(
            source,
            "schema",
            crate::preserves_rail::record("schema", vec![crate::preserves_rail::string("base")]),
            Vec::new(),
            "install base",
        );
        let source_stage = install_case_artifact(
            source,
            "stage",
            builtin_stage_operation_value("source").expect("source stage op"),
            Vec::new(),
            "install source stage",
        );
        let stage = install_case_artifact(
            source,
            "stage",
            builtin_stage_operation_value("identity").expect("stage op"),
            vec![base.artifact_ref.clone()],
            "install stage",
        );
        CopyArtifacts {
            base,
            source_stage,
            stage,
        }
    }

    fn copy_case() -> CopyCase {
        let root = temp_dir("job-sync");
        let source = root.join("source");
        let target = root.join("target");
        let CopyArtifacts {
            base,
            source_stage,
            stage,
        } = copy_artifacts(&source);
        let source_node = job_node_value(NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("x")]),
            ])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("source node");
        let map = job_node_value(NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("map node");
        let edge = stream_edge_value("source", "map").expect("edge");
        let dag_value = test_dag_value(vec![source_node, map], vec![edge], &["map".to_string()]).expect("dag value");
        let installed_job = install_job_dag(&source, &dag_value).expect("install job");
        let request = job_sync_request_value(SyncRequestValueInput {
            job_ref: &installed_job.job_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("sync-policy")],
            capability_refs: &[test_ref("sync-capability")],
            evidence_refs: &[test_ref("sync-evidence")],
        })
        .expect("sync request");
        CopyCase {
            root,
            source,
            target,
            base,
            source_stage,
            stage,
            installed_job,
            request,
        }
    }

    fn assert_copy(case: &CopyCase) -> String {
        let plan = sync_plan_value(&case.source, &case.target, &case.request).expect("sync plan");
        assert!(plan.missing_refs.contains(&case.base.artifact_ref));
        assert!(plan.missing_refs.contains(&case.stage.artifact_ref));
        let denied = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &[],
            build_verification_values: &[],
        })
        .expect("sync without provenance emits deny receipt");
        assert_eq!(denied.decision, "deny");
        assert!(denied.installed_refs.is_empty());
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance")));
        assert!(crate::artifacts::list_artifacts(&case.target, None).expect("target artifacts").is_empty());
        let sync_provenance = reviewed_provenance_values(&[
            case.base.artifact_ref.clone(),
            case.source_stage.artifact_ref.clone(),
            case.stage.artifact_ref.clone(),
            case.installed_job.artifact_ref.clone(),
        ]);
        let synced = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("sync loopback");
        assert!(synced.installed_refs.contains(&case.base.artifact_ref));
        assert!(synced.installed_refs.contains(&case.source_stage.artifact_ref));
        assert!(synced.installed_refs.contains(&case.stage.artifact_ref));
        assert_eq!(
            crate::artifacts::read_artifact(&case.target, &case.base.artifact_ref).expect("target base").value,
            case.base.artifact.value
        );
        let second = sync_loopback(SyncLoopbackInput {
            source_registry: &case.source,
            target_registry: &case.target,
            request_value: &case.request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("sync no-op");
        assert!(second.installed_refs.is_empty());
        assert!(second.already_present_refs.contains(&case.base.artifact_ref));
        assert!(
            crate::preserves_rail::to_text(&second.receipt_value)
                .expect("receipt text")
                .contains("no-execution")
        );
        crate::preserves_rail::canonical_hash(&synced.receipt_value).expect("sync receipt ref")
    }
