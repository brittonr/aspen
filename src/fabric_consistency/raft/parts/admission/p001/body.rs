fn validate_port_bindings(mut bindings: Vec<ReplicaPortBinding>) -> crate::error::Result<Vec<ReplicaPortBinding>> {
    if bindings.len() != REQUIRED_REPLICA_PORTS.len() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "live Raft startup requires exactly {} admitted fabric port bindings",
            REQUIRED_REPLICA_PORTS.len()
        )));
    }
    let mut unique = std::collections::BTreeSet::new();
    for binding in &bindings {
        molten_raft_core::denial::refs::validate_identifier(
            &binding.implementation_profile,
            "live Raft port implementation profile",
        )
        .map_err(super::replica_failure)?;
        molten_raft_core::denial::refs::validate_labelled_content_ref(&binding.binding_ref, "live Raft port binding ref")
            .map_err(super::replica_failure)?;
        if !unique.insert((binding.port_id.as_str(), binding.version.as_str())) {
            return Err(crate::error::Failure::invalid_harness(
                "live Raft startup contains a duplicate fabric port binding",
            ));
        }
    }
    for (port_id, version) in REQUIRED_REPLICA_PORTS {
        if !unique.contains(&(port_id, version)) {
            return Err(crate::error::Failure::invalid_harness(format!(
                "live Raft startup is missing required fabric port {port_id}@{version}"
            )));
        }
    }
    bindings.sort_by(|left, right| (&left.port_id, &left.version).cmp(&(&right.port_id, &right.version)));
    Ok(bindings)
}
