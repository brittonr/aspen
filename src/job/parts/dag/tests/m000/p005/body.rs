
    fn pipeline_value() -> Result<IoValue> {
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![
                    crate::preserves_rail::record("keep", vec![crate::preserves_rail::string("a")]),
                    crate::preserves_rail::record("drop", vec![crate::preserves_rail::string("b")]),
                    crate::preserves_rail::record("keep", vec![crate::preserves_rail::string("c")]),
                ]),
            ])]),
        )?;
        let filter = test_node_value(
            "filter",
            "filter",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![
                crate::preserves_rail::string("match-record"),
                crate::preserves_rail::string("keep"),
            ]),
        )?;
        let map = test_node_value(
            "map",
            "map",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![
                crate::preserves_rail::string("wrap"),
                crate::preserves_rail::string("item"),
            ]),
        )?;
        let materialize = test_node_value(
            "out",
            "materialize",
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("materialize", vec![crate::preserves_rail::string("inline")]),
        )?;
        let e1 = stream_edge_value("source", "filter")?;
        let e2 = stream_edge_value("filter", "map")?;
        let e3 = stream_edge_value("map", "out")?;
        test_dag_value(vec![source, filter, map, materialize], vec![e1, e2, e3], &["out".to_string()])
    }

    fn fixture_value(operation: &str) -> IoValue {
        let source = test_node_value(
            "source",
            "source",
            &[],
            &["out".to_string()],
            crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![
                    crate::preserves_rail::string("a"),
                    crate::preserves_rail::string("b"),
                ]),
            ])]),
        )
        .expect("source");
        let stage_kind = if operation == "count" { "reduce" } else { "map" };
        let stage = test_node_value(
            "stage",
            stage_kind,
            &["in".to_string()],
            &["out".to_string()],
            crate::preserves_rail::record("op", vec![crate::preserves_rail::string(operation)]),
        )
        .expect("stage");
        let edge = stream_edge_value("source", "stage").expect("edge");
        test_dag_value(vec![source, stage], vec![edge], &["stage".to_string()]).expect("dag")
    }

    fn test_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("job-dag-test-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("test ref")
    }

    fn reviewed_provenance_values(artifact_refs: &[String]) -> Vec<IoValue> {
        artifact_refs
            .iter()
            .map(|artifact_ref| {
                crate::provenance::synthetic_reviewed_record(artifact_ref).expect("reviewed provenance")
            })
            .collect()
    }

    fn install_clean_octet_gate(registry: &FilePath) -> String {
        let gate_value =
            crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean octet gate fixture");
        let gate_ref = crate::preserves_rail::canonical_hash(&gate_value).expect("octet gate ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("octet-policy")],
            evidence_refs: vec![test_ref("octet-evidence")],
            installer_ref: test_ref("octet-installer"),
            capability_refs: vec![test_ref("octet-capability")],
        })
        .expect("install octet gate");
        assert_eq!(install.decision, "pass");
        gate_ref
    }

    fn install_job_execute_authority_context(registry: &FilePath, job_ref: &str) -> String {
        let subject_ref = test_ref("target-peer-subject");
        let context_value = crate::authority::context_value(crate::authority::ContextValueInput {
            subject_ref: &subject_ref,
            capabilities: &[crate::authority::Capability {
                capability: "job:execute".to_string(),
                scope: job_ref.to_string(),
                attenuation: "scoped".to_string(),
            }],
            delegation_refs: &[],
            not_before: None,
            expires_at: None,
            revocation_refs: &[],
            key_refs: &[],
            policy_refs: &[test_ref("authority-policy")],
            evidence_refs: &[test_ref("authority-evidence")],
        })
        .expect("authority context");
        let context_ref = crate::preserves_rail::canonical_hash(&context_value).expect("authority context ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("authority-policy")],
            evidence_refs: vec![test_ref("authority-evidence")],
            installer_ref: test_ref("authority-installer"),
            capability_refs: vec![test_ref("authority-install-capability")],
        })
        .expect("install authority context");
        assert_eq!(install.decision, "pass");
        context_ref
    }

    struct WorkerFixture {
        root: std::path::PathBuf,
        source: std::path::PathBuf,
        target: std::path::PathBuf,
        ledger: std::path::PathBuf,
        installed_job: JobInstall,
        admission: JobAdmissionLoopback,
        sync_ref: String,
        admission_ref: String,
        execution_request_ref: String,
        execution_request: IoValue,
        context_ref: String,
        resource_refs: Vec<String>,
        peer_bootstrap_ref: String,
        identity_ref: String,
        evidence_refs: Vec<String>,
        worker_request: IoValue,
        delivery: crate::remote_dataspace::Delivery,
        delivery_log: crate::remote_dataspace::DeliveryLog,
    }

    struct SeedArtifacts {
        base: crate::artifacts::ArtifactInstall,
        source_stage: crate::artifacts::ArtifactInstall,
        map_stage: crate::artifacts::ArtifactInstall,
    }

    struct FlowParts {
        context_ref: String,
        resource_refs: Vec<String>,
        admission: JobAdmissionLoopback,
        admission_ref: String,
        execution_request: IoValue,
        execution_request_ref: String,
    }

    struct RequestParts {
        peer_bootstrap_ref: String,
        identity_ref: String,
        evidence_refs: Vec<String>,
        worker_request: IoValue,
    }

    fn seed_artifacts(source: &FilePath) -> SeedArtifacts {
        let base = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
            kind: "schema".to_string(),
            payload: crate::preserves_rail::record("schema", vec![crate::preserves_rail::string("worker-base")]),
            schema_refs: vec![test_ref("worker-schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-policy")],
            evidence_refs: vec![test_ref("worker-evidence")],
            installer_ref: test_ref("worker-installer"),
            capability_refs: vec![test_ref("worker-capability")],
        })
        .expect("install worker base");
        let source_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: builtin_stage_operation_value("source").expect("source operation"),
            schema_refs: vec![test_ref("worker-stage-schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-stage-policy")],
            evidence_refs: vec![test_ref("worker-stage-evidence")],
            installer_ref: test_ref("worker-stage-installer"),
            capability_refs: vec![test_ref("worker-stage-capability")],
        })
        .expect("install worker source stage");
        let map_stage = crate::artifacts::install_artifact(source, &crate::artifacts::ArtifactInstallInput {
            kind: "stage".to_string(),
            payload: builtin_stage_operation_value("identity").expect("identity operation"),
            schema_refs: vec![test_ref("worker-stage-schema")],
            dependency_refs: vec![base.artifact_ref.clone()],
            effect_manifest_ref: None,
            policy_refs: vec![test_ref("worker-stage-policy")],
            evidence_refs: vec![test_ref("worker-stage-evidence")],
            installer_ref: test_ref("worker-stage-installer"),
            capability_refs: vec![test_ref("worker-stage-capability")],
        })
        .expect("install worker map stage");
        SeedArtifacts {
            base,
            source_stage,
            map_stage,
        }
    }

    fn seed_graph(source: &FilePath, seed: &SeedArtifacts) -> JobInstall {
        let source_node = job_node_value(NodeValueInput {
            id: "source",
            kind: "source",
            stage_artifact_ref: Some(&seed.source_stage.artifact_ref),
            input_ports: &[],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
                crate::preserves_rail::sequence(vec![crate::preserves_rail::string("remote-x")]),
            ])]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("worker source node");
        let map_node = job_node_value(NodeValueInput {
            id: "map",
            kind: "map",
            stage_artifact_ref: Some(&seed.map_stage.artifact_ref),
            input_ports: &["in".to_string()],
            output_ports: &["out".to_string()],
            config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("worker map node");
        let edge = stream_edge_value("source", "map").expect("worker edge");
        let dag_value =
            test_dag_value(vec![source_node, map_node], vec![edge], &["map".to_string()]).expect("worker dag value");
        install_job_dag(source, &dag_value).expect("install worker job")
    }

    fn synced_ref(source: &FilePath, target: &FilePath, installed: &JobInstall, seed: &SeedArtifacts) -> String {
        let sync_request = job_sync_request_value(SyncRequestValueInput {
            job_ref: &installed.job_ref,
            stage_ids: &[],
            target_peer: "peer:b",
            policy_refs: &[test_ref("worker-sync-policy")],
            capability_refs: &[test_ref("worker-sync-capability")],
            evidence_refs: &[test_ref("worker-sync-evidence")],
        })
        .expect("worker sync request");
        let sync_provenance = reviewed_provenance_values(&[
            seed.base.artifact_ref.clone(),
            seed.source_stage.artifact_ref.clone(),
            seed.map_stage.artifact_ref.clone(),
            installed.artifact_ref.clone(),
        ]);
        let synced = sync_loopback(SyncLoopbackInput {
            source_registry: source,
            target_registry: target,
            request_value: &sync_request,
            provenance_values: &sync_provenance,
            build_verification_values: &[],
        })
        .expect("worker sync loopback");
        crate::preserves_rail::canonical_hash(&synced.receipt_value).expect("worker sync ref")
    }
