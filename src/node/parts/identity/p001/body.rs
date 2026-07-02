
pub fn parse_bootstrap_handshake(value: &IoValue) -> Result<BootstrapHandshake> {
    let fields = value
        .collect_simple_record("node-identity-bootstrap-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-identity-bootstrap-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_IDENTITY_BOOTSTRAP_SCHEMA,
        "node identity bootstrap schema",
    )?;
    let node = value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node bootstrap missing node field"))?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "join-admission-still-required")?;
    Ok(BootstrapHandshake {
        handshake_ref: crate::preserves_rail::canonical_hash(value)?,
        identity_ref: record_string(&node_fields[0], "identity")?,
        endpoint_id: record_string(&node_fields[2], "endpoint-id")?,
        peer: record_string(&fields[2], "peer")?,
        value: value.clone(),
    })
}

pub fn startup_evidence_value(identity_ref: &str, receipt_ref: &str) -> Result<IoValue> {
    require_ref(identity_ref, "node identity startup identity ref")?;
    require_ref(receipt_ref, "node identity startup receipt ref")?;
    Ok(record("node-identity-startup-v1", vec![
        string(crate::preserves_rail::NODE_IDENTITY_STARTUP_SCHEMA),
        record("identity", vec![string(identity_ref)]),
        record("receipt", vec![string(receipt_ref)]),
        record("checks", vec![crate::preserves_rail::sequence(vec![
            record("check", vec![string("replay-ref-only"), string("pass")]),
            record("check", vec![string("private-key-not-required"), string("pass")]),
        ])]),
    ]))
}

fn finish_resolution(input: ResolutionInput<'_>) -> Result<Resolution> {
    if input.secret.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node endpoint secret must not be empty"));
    }
    let existing_endpoint = fs::read_to_string(input.endpoint_path).ok().map(|value| value.trim().to_string());
    let is_drift = existing_endpoint.as_deref().is_some_and(|existing| existing != input.material.endpoint_id.as_str());
    if is_drift && !input.config.allow_rotation {
        return drift_denial(&input);
    }

    fs::create_dir_all(&input.config.data_dir).map_err(MoltenError::from)?;
    fs::write(input.endpoint_path, &input.material.endpoint_id).map_err(MoltenError::from)?;
    let receipt_operation = resolution_operation(&input, is_drift);
    let pre_receipt_value = pass_receipt(&input, receipt_operation, None);
    let pre_receipt_ref = crate::preserves_rail::canonical_hash(&pre_receipt_value)?;
    let identity_value = identity_value(
        input.config,
        input.material,
        input.operation,
        input.backend_ref,
        std::slice::from_ref(&pre_receipt_ref),
    );
    let identity = parse_identity(&identity_value)?;
    let receipt_value = pass_receipt(&input, receipt_operation, Some(&identity.identity_ref));
    Ok(Resolution {
        identity: Some(identity),
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn drift_denial(input: &ResolutionInput<'_>) -> Result<Resolution> {
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "drift-detected",
        decision: "fail",
        node_id: &input.config.node_id,
        identity_ref: None,
        endpoint_id: Some(&input.material.endpoint_id),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        policy_refs: &input.config.policy_refs,
        diagnostic: "endpoint id drift detected; rotation policy is required",
        checks: &["drift-detection", "rotation-denied", "no-secret-material"],
    });
    Ok(Resolution {
        identity: None,
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn resolution_operation<'a>(input: &ResolutionInput<'a>, is_drift: bool) -> &'a str {
    if is_drift {
        "rotation"
    } else if input.is_first_boot {
        "first-boot-generate"
    } else {
        input.operation
    }
}

fn pass_receipt(input: &ResolutionInput<'_>, operation: &str, identity_ref: Option<&str>) -> IoValue {
    const CHECKS: [&str; 6] = [
        "resolution-order",
        "stable-endpoint-id",
        "restricted-secret-file",
        "no-secret-material",
        "identity-grants-no-authority",
        "config-contract",
    ];
    receipt_value(&ReceiptValueInput {
        operation,
        decision: "pass",
        node_id: &input.config.node_id,
        identity_ref,
        endpoint_id: Some(&input.material.endpoint_id),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        policy_refs: &input.config.policy_refs,
        diagnostic: "node identity resolved without exposing secret material",
        checks: &CHECKS,
    })
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    record("node-identity-receipt-v1", vec![
        string(crate::preserves_rail::NODE_IDENTITY_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node_id)]),
        record("identity", vec![optional_ref_value(input.identity_ref)]),
        record("endpoint-id", vec![optional_string_value(input.endpoint_id)]),
        record("key-source", vec![
            record("class", vec![string(input.key_source_class)]),
            record("backend-ref", vec![string(input.backend_ref)]),
        ]),
        record("policy", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(string).collect(),
        )]),
        record("diagnostic", vec![string(input.diagnostic)]),
        record("checks", vec![crate::preserves_rail::sequence(
            input.checks.iter().map(|check| record("check", vec![string(check), string("pass")])).collect(),
        )]),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointMaterial {
    pub public_key: String,
    pub endpoint_id: String,
    pub secret_ref: String,
}

fn derive_endpoint_material(secret: &str) -> Result<EndpointMaterial> {
    if secret.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node endpoint secret must not be empty"));
    }
    let secret_ref = crate::preserves_rail::content_ref_from_bytes(secret.as_bytes());
    let mut public_material = b"molten-node-public\0".to_vec();
    public_material.extend_from_slice(secret.as_bytes());
    let public_key = crate::preserves_rail::content_ref_from_bytes(&public_material);
    let mut endpoint_material = b"molten-node-endpoint\0".to_vec();
    endpoint_material.extend_from_slice(public_key.as_bytes());
    let endpoint_id = format!("iroh:{}", blake3::hash(&endpoint_material).to_hex());
    Ok(EndpointMaterial {
        public_key,
        endpoint_id,
        secret_ref,
    })
}

fn generate_secret(node_id: &str, data_dir: &std::path::Path) -> Result<String> {
    let seed_ref = crate::preserves_rail::canonical_hash(&record("node-identity-generated-secret-seed", vec![
        record("node-id", vec![string(node_id)]),
        record("data-dir", vec![string(data_dir.display().to_string())]),
    ]))?;
    Ok(format!("molten-local-generated:{node_id}:{seed_ref}"))
}

fn backend_ref(data_dir: &std::path::Path) -> Result<String> {
    crate::preserves_rail::canonical_hash(&record("node-identity-backend", vec![
        record("class", vec![string("filesystem")]),
        record("data-dir", vec![string(data_dir.display().to_string())]),
    ]))
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
    validate_refs(&config.policy_refs, "node identity policy ref")
}

fn write_secret_restricted(path: &std::path::Path, secret: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(MoltenError::from)?;
        file.write_all(secret.as_bytes()).map_err(MoltenError::from)?;
        file.write_all(b"\n").map_err(MoltenError::from)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, format!("{secret}\n")).map_err(MoltenError::from)
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
