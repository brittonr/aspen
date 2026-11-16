
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


    // r[verify molten.octet_source_gate_receipt_validation.spec.unqualified_fixture_denied]
    // r[verify molten.operator_dogfood_node_workflow.spec.unqualified_fixture_gate]
    #[test]
    fn fixture_gate_is_denied_by_strict_admission_and_labelled_in_fixture_admission() {
        let case = copy_case();
        let sync_ref = assert_copy(&case);
        let context_ref = install_job_execute_authority_context(&case.target, &case.installed_job.job_ref);
        let fixture = crate::octet_gate::UnqualifiedFixtureGate::new("dogfood:local-node").expect("fixture gate");
        // Even registered under the Octet gate kind, the fixture is not an Octet gate receipt.
        crate::artifacts::install_artifact(&case.target, &crate::artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: fixture.value().clone(),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("octet-policy")],
            evidence_refs: vec![test_ref("octet-evidence")],
            installer_ref: test_ref("octet-installer"),
            capability_refs: vec![test_ref("octet-capability")],
        })
        .expect("install fixture under gate kind");
        let request = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&context_ref),
            evidence_refs: &[sync_ref.clone(), fixture.gate_ref().to_string()],
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("admission request");

        let strict = admission_loopback(&case.target, &request).expect("strict admission");
        assert_eq!(strict.plan.decision, "deny");
        assert!(strict.plan.diagnostics.iter().any(|diagnostic| diagnostic.contains("strict Octet source gate")));

        let admitted =
            admission_loopback_with_unqualified_fixture_gate(&case.target, &request, &fixture).expect("fixture admission");
        assert_eq!(admitted.plan.decision, "pass");
        assert!(admitted.plan.is_unqualified_fixture_gate);
        assert_eq!(admitted.plan.source_gate_validation_refs, vec![fixture.validation_ref().to_string()]);
        for value in [&admitted.plan.value, &admitted.receipt_value] {
            let text = crate::preserves_rail::to_text(value).expect("admission text");
            assert!(text.contains(r#"<check "strict-octet-source-gate-bound" "fail">"#), "{text}");
            assert!(text.contains(r#"<check "unqualified-fixture-source-gate" "pass">"#), "{text}");
        }

        let unbound = job_admission_request_value(AdmissionRequestValueInput {
            job_ref: &case.installed_job.job_ref,
            sync_ref: &sync_ref,
            stage_ids: &[],
            target_peer: "peer:loopback",
            policy_refs: &[test_ref("admit-policy")],
            capability_refs: std::slice::from_ref(&context_ref),
            evidence_refs: std::slice::from_ref(&sync_ref),
            resource_refs: &[test_ref("resource-1"), test_ref("resource-2")],
        })
        .expect("unbound request");
        let denied =
            admission_loopback_with_unqualified_fixture_gate(&case.target, &unbound, &fixture).expect("unbound fixture");
        assert_eq!(denied.plan.decision, "deny");
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
