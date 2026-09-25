fn source_scope_in_configured_inventory(consumer: &str, source_scope: &[String]) -> bool {
    let configured_paths = match consumer {
        "node-startup" | "node-control-gate" => NODE_SOURCE_GATE_SCOPE_PATHS,
        _ => SOURCE_GATE_SOURCE_SCOPE_PATHS,
    };
    source_scope.iter().all(|required| {
        configured_paths
            .iter()
            .any(|configured| configured == &required.as_str())
    })
}

pub fn default_source_scope(consumer: &str) -> Result<Vec<String>> {
    let scope = match consumer {
        "node-startup" => vec![
            "crates/molten-node-runtime/src/bin/molten-node.rs",
            "crates/molten-node-runtime/src/node/runtime.rs",
            "crates/molten-node-runtime/src/source_gate.rs",
        ],
        "job-remote-admission" => vec!["src/job/dag.rs", "src/main.rs", "src/octet/gate.rs"],
        "upgrade-plan" => vec!["src/main.rs", "src/octet/gate.rs", "src/upgrades/mod.rs"],
        "node-control-gate" => vec![
            "crates/molten-node-runtime/src/bin/molten-node.rs",
            "crates/molten-node-runtime/src/node/daemon.rs",
            "crates/molten-node-runtime/src/node/runtime.rs",
            "crates/molten-node-runtime/src/source_gate.rs",
        ],
        other => return Err(Failure::invalid_harness(format!("unsupported octet source-gate consumer {other}"))),
    };
    Ok(scope.into_iter().map(ToOwned::to_owned).collect())
}
