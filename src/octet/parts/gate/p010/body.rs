fn source_scope_in_configured_inventory(source_scope: &[String]) -> bool {
    source_scope.iter().all(|required| {
        SOURCE_GATE_SOURCE_SCOPE_PATHS
            .iter()
            .any(|configured| configured == &required.as_str())
    })
}

pub fn default_source_scope(consumer: &str) -> Result<Vec<String>> {
    let scope = match consumer {
        "node-startup" => vec!["src/main.rs", "src/node/runtime.rs", "src/octet/gate.rs"],
        "job-remote-admission" => vec!["src/job/dag.rs", "src/main.rs", "src/octet/gate.rs"],
        "upgrade-plan" => vec!["src/main.rs", "src/octet/gate.rs", "src/upgrades/mod.rs"],
        "node-control-gate" => vec![
            "src/main.rs",
            "src/node/daemon.rs",
            "src/node/runtime.rs",
            "src/octet/gate.rs",
        ],
        other => return Err(MoltenError::invalid_harness(format!("unsupported octet source-gate consumer {other}"))),
    };
    Ok(scope.into_iter().map(ToOwned::to_owned).collect())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/octet/parts/gate/tests/m000/p000/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/octet/parts/gate/tests/m000/p001/body.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/octet/parts/gate/tests/m000/p002/body.rs"));
}
