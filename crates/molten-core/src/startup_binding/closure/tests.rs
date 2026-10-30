use super::*;

fn reference() -> String {
    format!("blake3:{}", "a".repeat(64))
}

fn fixture() -> Closure {
    let graphs = ROLES
        .iter()
        .map(|role| Graph {
            role: (*role).into(),
            graph_ref: reference(),
            paths: vec![format!("/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-{role}")],
        })
        .collect::<Vec<_>>();
    let mut paths = graphs
        .iter()
        .map(|graph| MeasuredPath {
            path: graph.paths[0].clone(),
            nar_ref: reference(),
        })
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| left.path.cmp(&right.path));
    Closure {
        schema: SCHEMA.into(),
        graphs,
        paths,
    }
}

// r[verify molten.startup_binding.producer]
#[test]
fn complete_distinct_compiler_cohorts_admit_and_shared_paths_are_measured_once() {
    let mut closure = fixture();
    assert_eq!(validate(&closure, &closure.graphs), Ok(()));
    let removed = closure.graphs[3].paths[0].clone();
    closure.graphs[3].paths = closure.graphs[0].paths.clone();
    closure.paths.retain(|path| path.path != removed);
    assert_eq!(validate(&closure, &closure.graphs), Ok(()));
}

// r[verify molten.startup_binding.producer]
#[test]
fn missing_gate_compiler_graph_or_measurement_cannot_satisfy_candidate_closure() {
    let closure = fixture();
    let mut missing_graph = closure.clone();
    missing_graph.graphs.remove(4);
    assert_eq!(validate(&missing_graph, &missing_graph.graphs), Err(Issue::MissingGraph));
    let mut missing_nar = closure.clone();
    let compiler_path = &closure.graphs[4].paths[0];
    missing_nar.paths.retain(|path| &path.path != compiler_path);
    assert_eq!(validate(&missing_nar, &closure.graphs), Err(Issue::IncompletePaths));
    let mut observed = closure.graphs.clone();
    observed[4].graph_ref = format!("blake3:{}", "b".repeat(64));
    assert_eq!(validate(&closure, &observed), Err(Issue::GraphMismatch));
}
