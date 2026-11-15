use super::*;

#[test]
fn legacy_v1_labels_and_duplicate_edges_retain_stream_semantics() {
    let source = job_node_value(NodeValueInput {
        id: "source",
        kind: "source",
        stage_artifact_ref: None,
        input_ports: &[],
        output_ports: &["declared".into()],
        config: crate::preserves_rail::record("source", vec![crate::preserves_rail::record("values", vec![values(
            &[5],
        )])]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("legacy source");
    let consumer = job_node_value(NodeValueInput {
        id: "consumer",
        kind: "map",
        stage_artifact_ref: None,
        input_ports: &["declared".into()],
        output_ports: &["declared".into()],
        config: crate::preserves_rail::record("op", vec![crate::preserves_rail::string("identity")]),
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("legacy consumer");
    let edge = job_edge_value(EdgeValueInput {
        from_node: "source",
        from_port: "undeclared-output",
        to_node: "consumer",
        to_port: "undeclared-input",
        schema_ref: None,
        partitioning: "partitioned",
        materialization: "typed-ref",
    })
    .expect("legacy metadata");
    let value = job_dag_value(DagValueInput {
        nodes: vec![source, consumer],
        edges: vec![edge.clone(), edge],
        output_roots: &["consumer".into()],
        schema_refs: &[],
        effect_manifest_refs: &[],
        policy_refs: &[],
        evidence_refs: &[],
    })
    .expect("legacy graph");
    let workspace = crate::test_support::ProcessWorkspace::new("legacy_port_contract").expect("workspace");
    assert_eq!(run(&value, &workspace).output_value, values(&[5, 5]));
}

#[test]
fn effect_requirements_and_duplicate_publications_deny_before_stages() {
    let value = partition_graph("left");
    let dag = parse_job_dag_value(&value).expect("dag");
    let request = default_output_request(&dag).expect("request");
    let mut duplicate = request;
    duplicate.roots = vec!["consumer".into(), "consumer".into()];
    assert!(requested_output_roots(&dag, &duplicate).is_err());
    let fields = simple_record(&value, "job-dag-v2", 10).expect("fields");
    let mut changed: Vec<_> = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect();
    let effect = selected_sequence_schema().expect("well-formed reference");
    changed[6] = crate::preserves_rail::record("effect-manifests", vec![refs_sequence(&[effect])]);
    let changed = crate::preserves_rail::record("job-dag-v2", changed);
    assert!(parse_job_dag_value(&changed).is_err(), "source descriptors cannot authorize effectful execution");
    let workspace = crate::test_support::ProcessWorkspace::new("selected_effect_denial").expect("workspace");
    let options = JobRunOptions {
        registry_root: &workspace.join("registry"),
        storage_root: &workspace.join("storage"),
        cache_root: &workspace.join("cache"),
        chunk_root: &workspace.join("chunks"),
        ledger_root: None,
        output_request: None,
    };
    assert!(run_job_dag_value(&changed, &options).is_err());
    assert!(!workspace.join("cache").exists());
}

#[test]
fn conflicting_port_schema_metadata_denies() {
    let node = selected_node_value("source", "source-u64", values(&[1])).expect("source");
    let fields = simple_record(&node, "job-node-v1", 11).expect("node fields");
    let mut fields: Vec<_> = fields.fields_iter().map(crate::preserves_rail::value_to_iovalue).collect();
    fields[5] = crate::preserves_rail::record("outputs", vec![crate::preserves_rail::sequence(vec![
        crate::preserves_rail::record("port", vec![
            crate::preserves_rail::string("out"),
            crate::preserves_rail::string(selected_sequence_schema().expect("schema")),
        ]),
        crate::preserves_rail::record("port", vec![
            crate::preserves_rail::string("done"),
            crate::preserves_rail::record("none", Vec::new()),
        ]),
    ])]);
    assert!(graph(vec![crate::preserves_rail::record("job-node-v1", fields)], Vec::new(), &["source".into()]).is_err());
}

#[test]
fn unsupported_direct_profile_denies() {
    let mut dag = parse_job_dag_value(&partition_graph("left")).expect("dag");
    dag.version = "unsupported".into();
    assert!(validate_selected_current(&dag).is_err());
    assert!(plan_job_dag(&dag, None).is_err());
}
