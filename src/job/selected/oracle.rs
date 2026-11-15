use super::*;
include!("reference/interpreter.rs");

#[test]
fn reference_interpreter_agrees_with_real_execution_and_rejections() {
    let mut export: serde_json::Value =
        serde_json::from_str(include_str!("../../../config/job-graphs/diamond.json")).expect("fixture");
    let workspace = crate::test_support::ProcessWorkspace::new("selected_oracle").expect("workspace");
    for source in [vec![], vec![0], vec![1, 2, 3, 4], vec![7, 8, 9]] {
        export["instances"][0]["values"] = serde_json::json!(source);
        let expected = interpret(&export).expect("oracle");
        let actual =
            lower_job_graph_export(&export.to_string(), "captured", "evaluator", "limiter", &OrderedMap::new())
                .expect("lower");
        let result = run(&actual.dag.value, &workspace);
        let result_values = result.output_value.collect_sequence().expect("result");
        let observed =
            selected_output_values(&crate::preserves_rail::value_to_iovalue(&result_values[0]), "join", "out")
                .expect("routed");
        let expected: Vec<_> = expected[&("join".into(), "out".into())]
            .iter()
            .copied()
            .map(crate::preserves_rail::u64_value)
            .collect();
        assert_eq!(observed, expected);
    }
    for change in 0..6 {
        let mut bad = export.clone();
        match change {
            0 => bad["instances"][1]["id"] = bad["instances"][0]["id"].clone(),
            1 => bad["connections"][0]["from"]["port"] = "unknown".into(),
            2 => bad["instances"][1]["operation"] = "unsupported".into(),
            3 => {
                bad["connections"].as_array_mut().expect("edges").remove(0);
            }
            4 => bad["connections"][0]["from"]["instance"] = "missing".into(),
            _ => {
                let duplicate = bad["connections"][0].clone();
                bad["connections"].as_array_mut().expect("edges").push(duplicate);
            }
        }
        assert!(interpret(&bad).is_err(), "oracle {change}");
        assert!(
            lower_job_graph_export(&bad.to_string(), "captured", "evaluator", "limiter", &OrderedMap::new()).is_err(),
            "native {change}"
        );
    }
}

#[test]
fn reference_and_runtime_agree_on_overflow_failure() {
    let dto = serde_json::json!({"profile":"selected-sequence-v2", "instances":[
        {"id":"source","operation":"source-u64","values":[u64::MAX,1]},
        {"id":"sum","operation":"sum-u64","values":[]}],
        "connections":[{"from":{"instance":"source","port":"out"},"to":{"instance":"sum","port":"in"}}],
        "outputs":["sum"],"origins":[]});
    assert_eq!(interpret(&dto).expect_err("oracle overflow"), "overflow");
    let graph = lower_job_graph_export(&dto.to_string(), "source", "evaluator", "limiter", &OrderedMap::new())
        .expect("admitted");
    let workspace = crate::test_support::ProcessWorkspace::new("selected_overflow").expect("workspace");
    let result = run_job_dag(&graph.dag, &JobRunOptions {
        registry_root: &workspace.join("registry"),
        storage_root: &workspace.join("storage"),
        cache_root: &workspace.join("cache"),
        chunk_root: &workspace.join("chunks"),
        ledger_root: None,
        output_request: None,
    });
    assert!(result.expect_err("runtime overflow").to_string().contains("overflow"));
}
