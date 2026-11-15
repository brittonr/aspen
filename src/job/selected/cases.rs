#[test]
// r[verify molten.job_dag.selected_sequence]
fn outputs_execute_and_cache_without_cross_port_leakage() {
    let workspace = crate::test_support::ProcessWorkspace::new("selected_ports").expect("workspace");
    let left = partition_graph("left");
    let first = run(&left, &workspace);
    let second = run(&left, &workspace);
    assert_eq!(first.output_value, second.output_value);
    let outputs = second.output_value.collect_sequence().expect("outputs");
    assert_eq!(selected_output_values(&crate::preserves_rail::value_to_iovalue(&outputs[0]), "consumer", "out").expect("left"),
        vec![crate::preserves_rail::u64_value(2), crate::preserves_rail::u64_value(4)]);
    let right = partition_graph("right");
    let changed = run(&right, &workspace);
    assert_ne!(first.job_ref, changed.job_ref);
    assert_ne!(first.output_value, changed.output_value);
    let left_dag = parse_job_dag_value(&left).expect("left dag");
    let right_dag = parse_job_dag_value(&right).expect("right dag");
    let left_request = default_output_request(&left_dag).expect("request");
    let right_request = default_output_request(&right_dag).expect("request");
    let left_key = stage_cache_key_input(&left_dag, &left_request, &left_dag.nodes[2], &[]).expect("key");
    let right_key = stage_cache_key_input(&right_dag, &right_request, &right_dag.nodes[2], &[]).expect("key");
    assert_ne!(left_key.input_ref, right_key.input_ref);
    assert_ne!(selected_stage_signature("identity-u64").expect("signature"), selected_stage_signature("concat-u64").expect("signature"));
    assert_ne!(first.stage_receipt_refs, second.stage_receipt_refs, "rerun emits memo-hit rather than stage receipts");
}

#[test]
fn admission_denies_direction_required_multiplicity_schema_profile_and_signature() {
    let base = parse_job_dag_value(&partition_graph("left")).expect("base");
    for mutate in 0..9 {
        let mut nodes = base.nodes.clone();
        let mut edges = base.edges.clone();
        match mutate {
            0 => edges[1].from_port = "unknown".into(),
            1 => edges[1].from_port = "in".into(),
            2 => { edges.pop(); },
            3 => edges.push(edges[1].clone()),
            4 => edges[1].schema_ref = Some(selected_control_schema().expect("schema")),
            5 => nodes[1].output_ports.reverse(),
            6 => nodes[1].kind = "filter".into(),
            7 => {
                let mut additional = edges[1].clone();
                additional.from_node = "source".into();
                additional.from_port = "out".into();
                edges.push(additional);
            }
            _ => nodes[1].config = crate::preserves_rail::record("selected-stage-v2", vec![
                crate::preserves_rail::string("partition-even-u64"), values(&[]), crate::preserves_rail::string("blake3:wrong")]),
        }
        assert!(validate_selected_graph(&nodes, &edges, &base.output_roots).is_err(), "mutation {mutate}");
    }
    assert!(selected_node_value("bad", "unsupported-profile", values(&[])).is_err());
    let mut dag = base;
    dag.nodes[0].id = "tampered".into();
    assert!(validate_selected_current(&dag).is_err());
}

#[test]
fn shared_diamond_has_one_producer_and_deterministic_fan_in() {
    let value = graph(vec![
        selected_node_value("source", "source-u64", values(&[9, 2])).expect("source"),
        selected_node_value("right", "identity-u64", values(&[])).expect("right"),
        selected_node_value("left", "identity-u64", values(&[])).expect("left"),
        selected_node_value("join", "concat-u64", values(&[])).expect("join"),
    ], vec![edge("source", "out", "right", "in"), edge("source", "out", "left", "in"),
        edge("right", "out", "join", "in"), edge("left", "out", "join", "in")], &["join".into()]).expect("diamond");
    let workspace = crate::test_support::ProcessWorkspace::new("selected_diamond").expect("workspace");
    let result = run(&value, &workspace);
    assert_eq!(result.stage_receipt_refs.len(), 4);
    let outputs = result.output_value.collect_sequence().expect("outputs");
    assert_eq!(selected_output_values(&crate::preserves_rail::value_to_iovalue(&outputs[0]), "join", "out").expect("values"),
        vec![crate::preserves_rail::u64_value(9), crate::preserves_rail::u64_value(2), crate::preserves_rail::u64_value(9), crate::preserves_rail::u64_value(2)]);
    assert_eq!(outputs.len(), 2, "one data publication and Unit control for the requested root");
}

#[test]
fn unit_dependencies_survive_planning_and_empty_data() {
    let value = graph(vec![
        selected_node_value("z_first", "source-u64", values(&[])).expect("first"),
        selected_node_value("a_second", "source-u64", values(&[7])).expect("second"),
    ], vec![edge("z_first", "done", "a_second", "after")], &["a_second".into()]).expect("control graph");
    let dag = parse_job_dag_value(&value).expect("dag");
    assert_eq!(plan_job_dag(&dag, None).expect("plan").stage_order, ["z_first", "a_second"]);
    let workspace = crate::test_support::ProcessWorkspace::new("selected_unit").expect("workspace");
    assert_eq!(run(&value, &workspace).stage_receipt_refs.len(), 2);
}

#[test]
fn bounds_cover_expansion_source_and_routed_fan_in() {
    assert!(graph(vec![selected_node_value("source", "source-u64", values(&vec![1; MAX_JOB_STAGE_VALUES + 1])).expect("encoded")],
        vec![], &["source".into()]).is_err());
    let value = graph(vec![
        selected_node_value("a", "source-u64", values(&vec![1; MAX_JOB_STAGE_VALUES])).expect("a"),
        selected_node_value("b", "source-u64", values(&[2])).expect("b"),
        selected_node_value("join", "concat-u64", values(&[])).expect("join"),
    ], vec![edge("a", "out", "join", "in"), edge("b", "out", "join", "in")], &["join".into()]).expect("bounded graph");
    let dag = parse_job_dag_value(&value).expect("dag");
    let plan = trellis_execution_plan(&dag.nodes, &dag.edges).expect("plan");
    let mut outputs = vec![None; 3];
    for node in &dag.nodes[..2] { outputs[plan.node_index[&node.id]] = Some(execute_selected_stage(node, &[]).expect("source")); }
    assert!(gather_selected_inputs(&dag.nodes[2], &dag.edges, &outputs, &plan.node_index).is_err());
}