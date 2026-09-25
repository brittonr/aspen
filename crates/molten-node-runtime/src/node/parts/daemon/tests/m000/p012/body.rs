
    fn graph_value(stages: &StagePair) -> IoValue {
        crate::job_dag::job_dag_value(crate::job_dag::DagValueInput {
            nodes: vec![
                source_vertex_value(&stages.source_ref),
                map_vertex_value(&stages.map_ref),
            ],
            edges: vec![fixture_edge_value()],
            output_roots: &["map".to_string()],
            schema_refs: &[],
            effect_manifest_refs: &[],
            policy_refs: &[],
            evidence_refs: &[],
        })
        .expect("dag value")
    }

    fn admit_graph(registry: &Path, graph_ref: &str) -> AdmissionParts {
        let authority_ref = install_job_authority(registry, graph_ref);
        let gate_ref = install_clean_gate(registry);
        let sync_ref = local_ref("node-job-sync", graph_ref).expect("sync ref");
        let resource_refs = vec![local_ref("node-job-resource", graph_ref).expect("resource ref")];
        let policy_refs = vec![local_ref("node-job-policy", graph_ref).expect("policy ref")];
        let capability_refs = vec![authority_ref];
        let evidence_refs = vec![sync_ref.clone(), gate_ref];
        let admission_request =
            crate::job_dag::job_admission_request_value(crate::job_dag::AdmissionRequestValueInput {
                job_ref: graph_ref,
                sync_ref: &sync_ref,
                stage_ids: &[],
                target_peer: "node:ops",
                policy_refs: &policy_refs,
                capability_refs: &capability_refs,
                evidence_refs: &evidence_refs,
                resource_refs: &resource_refs,
            })
            .expect("admission request");
        let admission = crate::job_dag::admission_loopback(registry, &admission_request).expect("admission loopback");
        assert_eq!(admission.plan.decision, "pass");
        AdmissionParts {
            receipt_ref: crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("admission ref"),
            receipt_value: admission.receipt_value,
            stage_order: admission.plan.stage_order,
            policy_refs,
            capability_refs,
            resource_refs,
        }
    }

    fn execution_request_value(graph_ref: &str, admission: &AdmissionParts) -> IoValue {
        crate::job_dag::job_execution_request_value(crate::job_dag::ExecutionRequestValueInput {
            job_ref: graph_ref,
            admission_ref: &admission.receipt_ref,
            stage_ids: &admission.stage_order,
            target_peer: "node:ops",
            storage_profile_ref: &local_ref("node-job-storage", graph_ref).expect("storage ref"),
            cache_profile_ref: &local_ref("node-job-cache", graph_ref).expect("cache ref"),
            chunk_profile_ref: &local_ref("node-job-chunks", graph_ref).expect("chunks ref"),
            policy_refs: &admission.policy_refs,
            capability_refs: &admission.capability_refs,
            resource_refs: &admission.resource_refs,
        })
        .expect("execution request")
    }

    fn install_job_authority(registry: &Path, job_ref: &str) -> String {
        let subject_ref = local_ref("node-job-authority-subject", job_ref).expect("authority subject");
        let policy_ref = local_ref("node-job-authority-policy", job_ref).expect("authority policy");
        let evidence_ref = local_ref("node-job-authority-evidence", job_ref).expect("authority evidence");
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
            policy_refs: std::slice::from_ref(&policy_ref),
            evidence_refs: std::slice::from_ref(&evidence_ref),
        })
        .expect("authority context");
        let context_ref = crate::preserves_rail::canonical_hash(&context_value).expect("authority context ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "authority-context".to_string(),
            payload: context_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![policy_ref],
            evidence_refs: vec![evidence_ref],
            installer_ref: local_ref("node-job-authority-installer", job_ref).expect("authority installer"),
            capability_refs: vec![local_ref("node-job-authority-capability", job_ref).expect("authority capability")],
        })
        .expect("install authority context");
        assert_eq!(install.decision, "pass");
        context_ref
    }

    fn install_clean_gate(registry: &Path) -> String {
        let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate");
        let gate_ref = crate::preserves_rail::canonical_hash(&gate_value).expect("gate ref");
        let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
            kind: "octet-gate-receipt".to_string(),
            payload: gate_value,
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: vec![local_ref("node-job-gate-policy", &gate_ref).expect("gate policy")],
            evidence_refs: vec![local_ref("node-job-gate-evidence", &gate_ref).expect("gate evidence")],
            installer_ref: local_ref("node-job-gate-installer", &gate_ref).expect("gate installer"),
            capability_refs: vec![local_ref("node-job-gate-capability", &gate_ref).expect("gate capability")],
        })
        .expect("install gate");
        assert_eq!(install.decision, "pass");
        gate_ref
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: Counter = Counter::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, RELAXED);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
