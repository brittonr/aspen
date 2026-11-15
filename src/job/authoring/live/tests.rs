fn live_lower(source: &str, modules: &[CapturedJobModule], parameters: &serde_json::Value, workspace: &FilePath) -> Result<LoweredNickelJob> {
    let nickel = std::env::var_os("MOLTEN_GRAPH_NICKEL").expect("select absolute pinned Nickel 1.17.0 executable");
    let limiter = std::env::var_os("MOLTEN_GRAPH_PRLIMIT").expect("select absolute Linux prlimit executable");
    let evaluator_ref = job_evaluator_identity(FilePath::new(&nickel))?;
    let limiter_ref = job_evaluator_identity(FilePath::new(&limiter))?;
    lower_captured_nickel_job(CapturedNickelJobInput { source_name: "main", source, modules, parameters,
        nickel: FilePath::new(&nickel), prlimit: FilePath::new(&limiter), workspace,
        expected_evaluator_ref: &evaluator_ref, expected_limiter_ref: &limiter_ref })
}

#[test]
#[ignore = "requires explicit pinned Nickel and Linux prlimit paths; see docs/job-dag-composition.md"]
fn captured_nickel_live_examples_execute_like_direct_graphs() {
    let workspace = crate::test_support::ProcessWorkspace::new("captured_live").expect("workspace");
    let source = include_str!("../../../../config/job-graphs/diamond.ncl");
    let lowered = live_lower(source, &[], &serde_json::json!({"values":[1,2,3,4]}), &workspace).expect("evaluate actual Nickel");
    let direct = lower_job_graph_export(&exported_fixture().to_string(), "direct", "evaluator", "limiter", &OrderedMap::new()).expect("direct");
    assert_eq!(lowered.dag.value, direct.dag.value);
    assert_eq!(run(&lowered.dag.value, &workspace).output_value, run(&direct.dag.value, &workspace).output_value);
    let modules = [CapturedJobModule { key: "source".into(), source: include_str!("../../../../config/job-graphs/source-template.ncl").into() }];
    let parameterized = live_lower(include_str!("../../../../config/job-graphs/instances.ncl"), &modules,
        &serde_json::json!({"first":[1,2],"second":[3,4]}), &workspace).expect("captured reusable module");
    assert_eq!(parameterized.dag.nodes.len(), 3, "two explicit occurrences are not deduplicated");
    let result = run(&parameterized.dag.value, &workspace);
    let values = result.output_value.collect_sequence().expect("output");
    assert_eq!(selected_output_values(&crate::preserves_rail::value_to_iovalue(&values[0]), "join", "out").expect("values"),
        vec![crate::preserves_rail::u64_value(1),crate::preserves_rail::u64_value(2),crate::preserves_rail::u64_value(3),crate::preserves_rail::u64_value(4)]);
    let unit = live_lower(include_str!("../../../../config/job-graphs/unit-sequence.ncl"), &[], &serde_json::json!({}), &workspace).expect("Unit sequence");
    assert_eq!(plan_job_dag(&unit.dag, None).expect("plan").stage_order, ["z_first", "a_second"]);
    let sequence = live_lower(include_str!("../../../../config/job-graphs/sequence.ncl"), &[], &serde_json::json!({}), &workspace).expect("sequence");
    assert_eq!(run(&sequence.dag.value, &workspace).stage_receipt_refs.len(), 2);
}

#[test]
#[ignore = "requires explicit pinned Nickel and Linux prlimit paths; see docs/job-dag-composition.md"]
fn captured_nickel_live_negative_controls_fail_without_stage_effects() {
    let workspace = crate::test_support::ProcessWorkspace::new("captured_negative").expect("workspace");
    for source in [include_str!("../../../../config/job-graphs/invalid-fraction.ncl"),
        include_str!("../../../../config/job-graphs/invalid-import.ncl"),
        include_str!("../../../../config/job-graphs/diverges.ncl")] {
        assert!(live_lower(source, &[], &serde_json::json!({}), &workspace).is_err());
    }
    assert!(!workspace.join("cache").exists(), "lowering must not execute/cache workload stages");
    assert!(live_lower(include_str!("../../../../config/job-graphs/diamond.ncl"), &[], &serde_json::json!({}), &workspace).is_err(), "missing parameters deny");
}

#[test]
fn captured_graph_matches_independently_authored_preserves_fixture() {
    let dto = serde_json::json!({"profile":"selected-sequence-v2", "instances":[{"id":"source","operation":"source-u64","values":[1,2]}],
        "connections":[],"outputs":["source"],"origins":[]});
    let lowered = lower_job_graph_export(&dto.to_string(), "source", "evaluator", "limiter", &OrderedMap::new()).expect("lower");
    let fixture = include_str!("../../../../config/job-graphs/source.preserves.in")
        .replace("@SOURCE_SIGNATURE@", &selected_stage_signature("source-u64").expect("selected implementation"));
    let expected = crate::preserves_rail::parse_text(&fixture).expect("independent Preserves fixture");
    assert_eq!(lowered.dag.value, expected);
    assert_eq!(lowered.dag.job_ref, crate::preserves_rail::canonical_hash(&expected).expect("canonical identity"));
}

#[test]
#[ignore = "requires explicit pinned Nickel and Linux prlimit paths; see docs/job-dag-composition.md"]
fn captured_nickel_live_inline_and_explicit_module_match_canonical_fixture() {
    let workspace = crate::test_support::ProcessWorkspace::new("captured_canonical").expect("workspace");
    let inline = r#"fun modules => fun parameters => { profile = "selected-sequence-v2", instances = [{id = "source", operation = "source-u64", values = parameters.values}], connections = [], outputs = ["source"], origins = [] }"#;
    let modular = r#"fun modules => fun parameters => { profile = "selected-sequence-v2", instances = [modules.source "source" parameters.values], connections = [], outputs = ["source"], origins = [] }"#;
    let parameters = serde_json::json!({"values":[1,2]});
    let first = live_lower(inline, &[], &parameters, &workspace).expect("inline");
    let modules = [CapturedJobModule { key: "source".into(), source: include_str!("../../../../config/job-graphs/source-template.ncl").into() }];
    let second = live_lower(modular, &modules, &parameters, &workspace).expect("module template");
    let expected = include_str!("../../../../config/job-graphs/source.preserves.in")
        .replace("@SOURCE_SIGNATURE@", &selected_stage_signature("source-u64").expect("selected implementation"));
    assert_eq!(first.dag.value, crate::preserves_rail::parse_text(&expected).expect("Preserves fixture"));
    assert_eq!(first.dag.value, second.dag.value);
    assert_ne!(first.provenance, second.provenance);
}

#[test]
#[ignore = "requires explicit pinned Nickel and Linux prlimit paths; see docs/job-dag-composition.md"]
fn captured_nickel_live_generation_and_parameter_invalidation() {
    let workspace = crate::test_support::ProcessWorkspace::new("captured_generation").expect("workspace");
    let source = include_str!("../../../../config/job-graphs/generated.ncl");
    let first = live_lower(source, &[], &serde_json::json!({"instances":[{"key":"a","payload":[1]},{"key":"b","payload":[2]}]}), &workspace).expect("generated");
    let changed = live_lower(source, &[], &serde_json::json!({"instances":[{"key":"a","payload":[3]},{"key":"b","payload":[2]}]}), &workspace).expect("changed parameters");
    assert_ne!(first.dag.job_ref, changed.dag.job_ref);
    assert_ne!(first.provenance, changed.provenance);
    assert_eq!(first.dag.nodes.len(), 2);
    let modules = [CapturedJobModule { key:"unused".into(), source:"null".into() }];
    let captured = live_lower(source, &modules, &serde_json::json!({"instances":[{"key":"a","payload":[1]},{"key":"b","payload":[2]}]}), &workspace).expect("captured unused module");
    assert_eq!(first.dag.job_ref, captured.dag.job_ref);
    assert_ne!(first.provenance, captured.provenance);
    let oversized = serde_json::json!({"instances":(0..257).map(|n| serde_json::json!({"key":format!("n{n}"),"payload":[]})).collect::<Vec<_>>()});
    assert!(live_lower(source, &[], &oversized, &workspace).is_err());
}