//! Job DAG fixtures for operation dispatch tests.

pub(super) fn install_job_fixture(root: &std::path::Path) -> super::dispatches::JobFixture {
    let registry = root.join("registry");
    let stages = install_stage_pair(&registry);
    let dag_value = graph_value(&stages);
    let installed = crate::job_dag::install_job_dag(&registry, &dag_value).expect("install job dag");
    let admission = admit_graph(&registry, &installed.job_ref);
    let execution_request = execution_request_value(&installed.job_ref, &admission);
    super::dispatches::JobFixture {
        execution_request,
        admission_receipt: admission.receipt_value,
        job_ref: installed.job_ref,
    }
}

fn install_stage_pair(registry: &std::path::Path) -> super::dispatches::StagePair {
    let stage_schema = super::super::storage::local_ref("node-job-stage-schema", "ops").expect("stage schema");
    let stage_policy = super::super::storage::local_ref("node-job-stage-policy", "ops").expect("stage policy");
    let stage_evidence = super::super::storage::local_ref("node-job-stage-evidence", "ops").expect("stage evidence");
    let stage_installer = super::super::storage::local_ref("node-job-stage-installer", "ops").expect("stage installer");
    let stage_capability =
        super::super::storage::local_ref("node-job-stage-capability", "ops").expect("stage capability");
    let source_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("source").expect("source operation"),
        schema_refs: vec![stage_schema.clone()],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: vec![stage_policy.clone()],
        evidence_refs: vec![stage_evidence.clone()],
        installer_ref: stage_installer.clone(),
        capability_refs: vec![stage_capability.clone()],
    })
    .expect("install source stage");
    let map_stage = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "stage".to_string(),
        payload: crate::job_dag::builtin_stage_operation_value("identity").expect("identity operation"),
        schema_refs: vec![stage_schema],
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: vec![stage_policy],
        evidence_refs: vec![stage_evidence],
        installer_ref: stage_installer,
        capability_refs: vec![stage_capability],
    })
    .expect("install map stage");
    super::dispatches::StagePair {
        source_ref: source_stage.artifact_ref,
        map_ref: map_stage.artifact_ref,
    }
}

fn source_vertex_value(stage_ref: &str) -> preserves::IOValue {
    crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: Some(stage_ref),
        input_ports: &[],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![
            crate::preserves_rail::sequence(vec![crate::preserves_rail::string("node-job")]),
        ])]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("source node")
}

fn map_vertex_value(stage_ref: &str) -> preserves::IOValue {
    crate::job_dag::job_node_value(crate::job_dag::NodeValueInput {
        id: "map",
        kind: "map",
        stage_artifact_ref: Some(stage_ref),
        input_ports: &["in".to_string()],
        output_ports: &["out".to_string()],
        config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("map node")
}

fn fixture_edge_value() -> preserves::IOValue {
    crate::job_dag::job_edge_value(crate::job_dag::EdgeValueInput {
        from_node: "source",
        from_port: "out",
        to_node: "map",
        to_port: "in",
        schema_ref: None,
        partitioning: "single",
        materialization: "stream",
    })
    .expect("edge")
}

fn graph_value(stages: &super::dispatches::StagePair) -> preserves::IOValue {
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

fn admit_graph(registry: &std::path::Path, graph_ref: &str) -> super::dispatches::AdmissionParts {
    let authority_ref = install_job_authority(registry, graph_ref);
    let gate_ref = install_clean_gate(registry);
    let sync_ref = super::super::storage::local_ref("node-job-sync", graph_ref).expect("sync ref");
    let resource_refs = vec![super::super::storage::local_ref("node-job-resource", graph_ref).expect("resource ref")];
    let policy_refs = vec![super::super::storage::local_ref("node-job-policy", graph_ref).expect("policy ref")];
    let capability_refs = vec![authority_ref];
    let evidence_refs = vec![sync_ref.clone(), gate_ref];
    let admission_request = crate::job_dag::job_admission_request_value(crate::job_dag::AdmissionRequestValueInput {
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
    super::dispatches::AdmissionParts {
        receipt_ref: crate::preserves_rail::canonical_hash(&admission.receipt_value).expect("admission ref"),
        receipt_value: admission.receipt_value,
        stage_order: admission.plan.stage_order,
        policy_refs,
        capability_refs,
        resource_refs,
    }
}

fn execution_request_value(graph_ref: &str, admission: &super::dispatches::AdmissionParts) -> preserves::IOValue {
    crate::job_dag::job_execution_request_value(crate::job_dag::ExecutionRequestValueInput {
        job_ref: graph_ref,
        admission_ref: &admission.receipt_ref,
        stage_ids: &admission.stage_order,
        target_peer: "node:ops",
        storage_profile_ref: &super::super::storage::local_ref("node-job-storage", graph_ref).expect("storage ref"),
        cache_profile_ref: &super::super::storage::local_ref("node-job-cache", graph_ref).expect("cache ref"),
        chunk_profile_ref: &super::super::storage::local_ref("node-job-chunks", graph_ref).expect("chunks ref"),
        policy_refs: &admission.policy_refs,
        capability_refs: &admission.capability_refs,
        resource_refs: &admission.resource_refs,
    })
    .expect("execution request")
}

fn install_job_authority(registry: &std::path::Path, job_ref: &str) -> String {
    let subject_ref =
        super::super::storage::local_ref("node-job-authority-subject", job_ref).expect("authority subject");
    let policy_ref = super::super::storage::local_ref("node-job-authority-policy", job_ref).expect("authority policy");
    let evidence_ref =
        super::super::storage::local_ref("node-job-authority-evidence", job_ref).expect("authority evidence");
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
        installer_ref: super::super::storage::local_ref("node-job-authority-installer", job_ref)
            .expect("authority installer"),
        capability_refs: vec![
            super::super::storage::local_ref("node-job-authority-capability", job_ref).expect("authority capability"),
        ],
    })
    .expect("install authority context");
    assert_eq!(install.decision, "pass");
    context_ref
}

fn install_clean_gate(registry: &std::path::Path) -> String {
    let gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests().expect("clean gate");
    let gate_ref = crate::preserves_rail::canonical_hash(&gate_value).expect("gate ref");
    let install = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "octet-gate-receipt".to_string(),
        payload: gate_value,
        schema_refs: Vec::new(),
        dependency_refs: Vec::new(),
        effect_manifest_ref: None,
        policy_refs: vec![super::super::storage::local_ref("node-job-gate-policy", &gate_ref).expect("gate policy")],
        evidence_refs: vec![
            super::super::storage::local_ref("node-job-gate-evidence", &gate_ref).expect("gate evidence"),
        ],
        installer_ref: super::super::storage::local_ref("node-job-gate-installer", &gate_ref).expect("gate installer"),
        capability_refs: vec![
            super::super::storage::local_ref("node-job-gate-capability", &gate_ref).expect("gate capability"),
        ],
    })
    .expect("install gate");
    assert_eq!(install.decision, "pass");
    gate_ref
}

pub(super) fn temp_dir(name: &str) -> std::path::PathBuf {
    crate::test_support::cleanup_stale_molten_temp_dirs();
    static TEMP_DIR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = TEMP_DIR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
    if dir.exists() {
        super::fs::remove_dir_all(&dir).expect("remove stale temp dir");
    }
    super::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
