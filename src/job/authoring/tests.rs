use super::*;

fn exported_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../config/job-graphs/diamond.json")).expect("fixture")
}

#[test]
// r[verify molten.job_dag.captured_authoring]
fn captured_dto_lowering_is_normal_admission_and_preserves_sharing() {
    let fixture = exported_fixture();
    let lowered = lower_job_graph_export(&fixture.to_string(), "captured", "evaluator", "limiter", &OrderedMap::new())
        .expect("lower");
    assert_eq!(lowered.dag.nodes.len(), 5);
    assert_eq!(lowered.dag.edges.len(), 5);
    assert_eq!(lowered.dag, parse_job_dag_value(&lowered.dag.value).expect("normal admission"));
    let mut reordered = fixture.clone();
    reordered["instances"].as_array_mut().expect("instances").reverse();
    reordered["connections"].as_array_mut().expect("connections").reverse();
    let reordered =
        lower_job_graph_export(&reordered.to_string(), "captured", "evaluator", "limiter", &OrderedMap::new())
            .expect("reordered");
    assert_eq!(lowered.dag.value, reordered.dag.value);
    assert_ne!(lowered.provenance, reordered.provenance, "raw exported representation remains provenance-bound");
}

#[test]
fn captured_rejects_unsupported_values_origins_cycles_and_expansion() {
    for change in 0..8 {
        let mut fixture = exported_fixture();
        match change {
            0 => fixture["instances"][0]["values"][0] = serde_json::json!(1.5),
            1 => fixture["instances"][0]["values"][0] = serde_json::json!(-1),
            2 => {
                fixture["origins"] =
                    serde_json::json!([{ "instance": "source", "template": "source", "module": "missing" }])
            }
            3 => fixture["connections"].as_array_mut().expect("connections").push(
                serde_json::json!({ "from": {"instance":"join","port":"out"}, "to":{"instance":"split","port":"in"}}),
            ),
            4 => {
                fixture["instances"] = serde_json::json!(
                    (0..257)
                        .map(|n| serde_json::json!({"id":format!("node{n}"),"operation":"source-u64","values":[]}))
                        .collect::<Vec<_>>()
                )
            }
            5 => fixture["profile"] = "unsupported".into(),
            6 => fixture["instances"][0]["live_handle"] = true.into(),
            _ => {
                fixture["origins"] = serde_json::json!([{ "instance": "source", "template": "source", "module": "main", "port":"unknown" }])
            }
        }
        assert!(
            lower_job_graph_export(&fixture.to_string(), "captured", "evaluator", "limiter", &OrderedMap::new())
                .is_err(),
            "mutation {change}"
        );
    }
    assert!(captured_source_check("import \"ambient.ncl\"").is_err());
    assert!(captured_source_check("\"%{effect}\"").is_err());
    assert!(job_nickel_literal(&serde_json::json!(0.5), 0).is_err());
    assert!(job_nickel_literal(&serde_json::json!(-1), 0).is_err());
    assert!(bound_job_parameters(&serde_json::json!("x".repeat(MAX_JOB_AUTHORING_BYTES)), 0, &mut 0).is_err());
}

include!("live/tests.rs");

#[test]
fn captured_origins_and_provenance_bind_exact_inputs_without_granting_authority() {
    let mut fixture = exported_fixture();
    fixture["origins"] =
        serde_json::json!([{ "instance":"source", "template":"inline", "module":"main", "port":"out" }]);
    let first = lower_job_graph_export(&fixture.to_string(), "source-a", "evaluator-a", "limiter", &OrderedMap::new())
        .expect("origin");
    let second = lower_job_graph_export(&fixture.to_string(), "source-b", "evaluator-a", "limiter", &OrderedMap::new())
        .expect("source changed");
    let third = lower_job_graph_export(&fixture.to_string(), "source-a", "evaluator-b", "limiter", &OrderedMap::new())
        .expect("evaluator changed");
    assert_eq!(first.origins[0].port.as_deref(), Some("out"));
    assert_eq!(first.dag.value, second.dag.value);
    assert_eq!(first.dag.value, third.dag.value);
    assert_ne!(first.provenance, second.provenance);
    assert_ne!(first.provenance, third.provenance);
}

#[test]
fn captured_parameter_literals_retain_exact_order_escaping_and_scalar_rules() {
    let value = serde_json::json!({"array":[null,true,false,0,u64::MAX,"quoted\"\n"], "empty":{}});
    assert_eq!(
        job_nickel_literal(&value, 0).expect("literal"),
        "{\"array\" = [null,true,false,0,18446744073709551615,\"quoted\\\"\\n\"],\"empty\" = {}}"
    );
    assert!(job_nickel_literal(&serde_json::json!({"%{key}":0}), 0).is_err());
    assert!(job_nickel_literal(&serde_json::json!(["%{value}"]), 0).is_err());
}

#[test]
fn captured_parameter_worklists_keep_depth_and_work_admission() {
    let mut value = serde_json::json!(1);
    for depth in 0..=33 {
        if depth <= 32 {
            let literal = job_nickel_literal(&value, 0).expect("admitted depth");
            assert_eq!(literal, format!("{}1{}", "[".repeat(depth), "]".repeat(depth)));
        } else {
            assert!(bound_job_parameters(&value, 0, &mut 0).is_err());
            assert!(job_nickel_literal(&value, 0).is_err());
        }
        value = serde_json::json!([value]);
    }
    let wide = serde_json::json!(vec![0; MAX_JOB_STAGE_VALUES + 1]);
    assert!(job_nickel_literal(&wide, 0).is_err());
    assert!(bound_job_parameters(&serde_json::json!([0]), 0, &mut (MAX_JOB_AUTHORING_BYTES - 1)).is_err());
}

#[test]
fn captured_origin_port_defaults_to_explicit_absence() {
    let origin: JobGraphOrigin = serde_json::from_value(serde_json::json!({
        "instance":"source", "template":"inline", "module":"main"
    }))
    .expect("origin without optional port");
    assert_eq!(origin.port, None);
}
