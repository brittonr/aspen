
fn source_metadata_ref(source_class: &str, backend_ref: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&record("node-identity-source-metadata", vec![
        record("class", vec![string(source_class)]),
        record("backend-ref", vec![string(backend_ref)]),
        record("path-class", vec![string("node-state-redacted")]),
        record("namespace", vec![string(IDENTITY_NAMESPACE_LABEL)]),
    ]))
}

pub fn admitted_rotation_receipt_ref(
    previous_endpoint_id: &str,
    next_endpoint_id: &str,
    policy_refs: &[String],
) -> Result<String> {
    validate_endpoint_id(previous_endpoint_id, "previous endpoint id")?;
    validate_endpoint_id(next_endpoint_id, "next endpoint id")?;
    validate_refs(policy_refs, "node identity rotation policy ref")?;
    crate::preserves_rail::canonical_hash(&record("node-identity-rotation-admission-v1", vec![
        record("previous-endpoint-id", vec![string(previous_endpoint_id)]),
        record("next-endpoint-id", vec![string(next_endpoint_id)]),
        record("policy", vec![crate::preserves_rail::sequence(policy_refs.iter().map(string).collect())]),
        record("checks", vec![crate::preserves_rail::sequence(vec![
            record("check", vec![string("operator-authority-required"), string("pass")]),
            record("check", vec![string("peer-refresh-obligation-recorded"), string("pass")]),
        ])]),
    ]))
}

fn validate_identity_namespace(root: &crate::node_state::NodeStateNamespace) -> Result<()> {
    match root.kind() {
        crate::node_state::NodeStateNamespaceKind::Identity | crate::node_state::NodeStateNamespaceKind::Secrets => {
            Ok(())
        }
        other => Err(MoltenError::invalid_harness(format!(
            "node identity requires identity or secrets namespace, got {other:?}"
        ))),
    }
}

fn validate_config(config: &Config) -> Result<()> {
    if config.node_id.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node id must not be empty"));
    }
    if config.display_name.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node display name must not be empty"));
    }
    if config.data_dir.as_os_str().is_empty() {
        return Err(MoltenError::invalid_harness("node data dir must not be empty"));
    }
    if let Some(backend_ref) = config.secret_backend_ref.as_deref() {
        require_ref(backend_ref, "managed secret backend ref")?;
    }
    if let Some(rotation_receipt_ref) = config.rotation_receipt_ref.as_deref() {
        require_ref(rotation_receipt_ref, "node identity rotation receipt ref")?;
    }
    validate_refs(&config.policy_refs, "node identity policy ref")
}

fn write_secret_restricted(
    root: &crate::node_state::NodeStateNamespace,
    path: &crate::node_state::NodeStatePath,
    secret_record: &[u8],
) -> Result<()> {
    root.write_restricted(path, secret_record, OWNER_ONLY_SECRET_FILE_MODE)
}

fn read_observed_secret(observation: crate::node_state::NodeStateFileObservation) -> Result<Vec<u8>> {
    let crate::node_state::NodeStateFileObservation::Regular(file) = observation else {
        return Err(MoltenError::invalid_harness(
            "persisted endpoint secret changed after source selection",
        ));
    };
    file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)
}

fn secret_file_permission_status(
    observation: &crate::node_state::NodeStateFileObservation,
) -> IrohSecretPermissionStatus {
    match observation {
        crate::node_state::NodeStateFileObservation::Missing => IrohSecretPermissionStatus::NotPresent,
        crate::node_state::NodeStateFileObservation::NonRegular(_) => IrohSecretPermissionStatus::Unsafe,
        crate::node_state::NodeStateFileObservation::Regular(file) => {
            #[cfg(unix)]
            {
                file.unix_mode().map_or(IrohSecretPermissionStatus::Unsupported, |mode| {
                    if mode & GROUP_OR_OTHER_SECRET_PERMISSION_BITS == 0 {
                        IrohSecretPermissionStatus::Restricted
                    } else {
                        IrohSecretPermissionStatus::Unsafe
                    }
                })
            }
            #[cfg(not(unix))]
            {
                let _ = file;
                IrohSecretPermissionStatus::Unsupported
            }
        }
    }
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn validate_endpoint_id(endpoint_id: &str, field: &str) -> Result<()> {
    if endpoint_id.starts_with(IROH_ENDPOINT_PREFIX) && endpoint_id.len() > IROH_ENDPOINT_PREFIX.len() {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("expected Iroh endpoint id for {field}, got {endpoint_id}")))
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected node identity checks"))?;
    let values = record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("node identity checks must be a sequence"))?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected node identity check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("node identity evidence missing passing {name} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&record[0], label)
}
