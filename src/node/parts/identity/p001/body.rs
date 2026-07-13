
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

fn source_denial(
    config: &Config,
    backend_ref: &str,
    source_metadata_ref: &str,
    source_decision: &IrohSecretSourceDecision,
) -> Result<Resolution> {
    let checks = source_denial_checks(source_decision.key_source_class);
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: source_denial_operation(source_decision.key_source_class),
        decision: "fail",
        node_id: &config.node_id,
        identity_ref: None,
        endpoint_id: None,
        previous_endpoint_id: None,
        rotation_receipt_ref: None,
        key_source_class: source_decision.key_source_class,
        backend_ref,
        source_metadata_ref: Some(source_metadata_ref),
        permission_status: source_decision.permission_status,
        policy_refs: &config.policy_refs,
        diagnostic: source_decision.diagnostic,
        checks: &checks,
    });
    Ok(Resolution {
        identity: None,
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn source_denial_operation(key_source_class: &str) -> &'static str {
    match key_source_class {
        KEY_SOURCE_MANAGED_BACKEND => "managed-backend-required",
        KEY_SOURCE_PERSISTED_FILE => "unsafe-persisted-permissions",
        _ => "deny-if-unavailable",
    }
}

fn source_denial_checks(key_source_class: &str) -> Vec<&'static str> {
    match key_source_class {
        KEY_SOURCE_MANAGED_BACKEND => vec!["resolution-order", "managed-backend-required", "no-secret-material"],
        KEY_SOURCE_PERSISTED_FILE => vec!["restricted-secret-file", "unsafe-permission-denied", "no-secret-material"],
        _ => vec!["resolution-order", "deny-if-unavailable", "no-secret-material"],
    }
}

fn finish_resolution(input: ResolutionInput<'_>) -> Result<Resolution> {
    if input.secret_record.is_empty() {
        return Err(MoltenError::invalid_harness("node endpoint key record must not be empty"));
    }
    let existing_endpoint = match input.root.observe_file(input.endpoint_path)? {
        crate::node_state::NodeStateFileObservation::Missing => None,
        crate::node_state::NodeStateFileObservation::NonRegular(kind) => {
            return Err(MoltenError::invalid_harness(format!(
                "node endpoint identity leaf must be a regular file, got {kind:?}"
            )));
        }
        crate::node_state::NodeStateFileObservation::Regular(file) => {
            let bytes = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
            Some(
                String::from_utf8(bytes)
                    .map_err(|error| MoltenError::invalid_harness(format!("node endpoint id is not UTF-8: {error}")))?
                    .trim()
                    .to_string(),
            )
        }
    };
    let expected_rotation_receipt_ref = existing_endpoint
        .as_deref()
        .filter(|prior_endpoint_id| *prior_endpoint_id != input.material.endpoint_id.as_str())
        .map(|prior_endpoint_id| {
            admitted_rotation_receipt_ref(prior_endpoint_id, &input.material.endpoint_id, &input.config.policy_refs)
        })
        .transpose()?;
    let observation = admit_iroh_endpoint_observation(&IrohEndpointObservationFacts {
        prior_endpoint_id: existing_endpoint,
        observed_endpoint_id: input.material.endpoint_id.clone(),
        rotation_allowed: input.config.allow_rotation,
        supplied_rotation_receipt_ref: input.config.rotation_receipt_ref.clone(),
        expected_rotation_receipt_ref,
    });
    if observation.kind == IrohEndpointObservationDecisionKind::Deny {
        return endpoint_observation_denial(&input, &observation);
    }

    input.root.write(input.endpoint_path, input.material.endpoint_id.as_bytes())?;
    let receipt_operation = resolution_operation(&input, observation.kind);
    let pre_receipt_value = pass_receipt(&input, &observation, receipt_operation, None);
    let pre_receipt_ref = crate::preserves_rail::canonical_hash(&pre_receipt_value)?;
    let identity_value = identity_value(
        input.config,
        input.material,
        input.operation,
        input.backend_ref,
        std::slice::from_ref(&pre_receipt_ref),
    );
    let identity = parse_identity(&identity_value)?;
    let receipt_value = pass_receipt(&input, &observation, receipt_operation, Some(&identity.identity_ref));
    Ok(Resolution {
        identity: Some(identity),
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn endpoint_observation_denial(
    input: &ResolutionInput<'_>,
    observation: &IrohEndpointObservationDecision,
) -> Result<Resolution> {
    let checks = endpoint_denial_checks(observation);
    let receipt_value = receipt_value(&ReceiptValueInput {
        operation: "drift-detected",
        decision: "fail",
        node_id: &input.config.node_id,
        identity_ref: None,
        endpoint_id: Some(&input.material.endpoint_id),
        previous_endpoint_id: observation.previous_endpoint_id.as_deref(),
        rotation_receipt_ref: observation.rotation_receipt_ref.as_deref(),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        source_metadata_ref: Some(input.source_metadata_ref),
        permission_status: input.permission_status,
        policy_refs: &input.config.policy_refs,
        diagnostic: observation.diagnostic,
        checks: &checks,
    });
    Ok(Resolution {
        identity: None,
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn endpoint_denial_checks(observation: &IrohEndpointObservationDecision) -> Vec<&'static str> {
    if observation.rotation_receipt_ref.is_some() {
        vec!["drift-detection", "stale-rotation-denied", "no-secret-material"]
    } else {
        vec!["drift-detection", "rotation-denied", "no-secret-material"]
    }
}

fn resolution_operation<'a>(input: &'a ResolutionInput<'a>, observation_kind: IrohEndpointObservationDecisionKind) -> &'a str {
    if observation_kind == IrohEndpointObservationDecisionKind::Rotate {
        "rotation"
    } else if input.is_first_boot {
        "first-boot-generate"
    } else {
        input.operation
    }
}

fn pass_receipt(
    input: &ResolutionInput<'_>,
    observation: &IrohEndpointObservationDecision,
    operation: &str,
    identity_ref: Option<&str>,
) -> IoValue {
    let checks = pass_checks(observation.kind);
    receipt_value(&ReceiptValueInput {
        operation,
        decision: "pass",
        node_id: &input.config.node_id,
        identity_ref,
        endpoint_id: Some(&input.material.endpoint_id),
        previous_endpoint_id: observation.previous_endpoint_id.as_deref(),
        rotation_receipt_ref: observation.rotation_receipt_ref.as_deref(),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        source_metadata_ref: Some(input.source_metadata_ref),
        permission_status: input.permission_status,
        policy_refs: &input.config.policy_refs,
        diagnostic: observation.diagnostic,
        checks: &checks,
    })
}

fn pass_checks(observation_kind: IrohEndpointObservationDecisionKind) -> Vec<&'static str> {
    let mut checks = vec![
        "resolution-order",
        "stable-endpoint-id",
        "restricted-secret-file",
        "no-secret-material",
        "identity-grants-no-authority",
        "config-contract",
    ];
    if observation_kind == IrohEndpointObservationDecisionKind::Rotate {
        checks.push("rotation-receipt-admitted");
    }
    checks
}

fn receipt_value(input: &ReceiptValueInput<'_>) -> IoValue {
    record("node-identity-receipt-v1", vec![
        string(crate::preserves_rail::NODE_IDENTITY_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node_id)]),
        record("identity", vec![optional_ref_value(input.identity_ref)]),
        record("endpoint-id", vec![optional_string_value(input.endpoint_id)]),
        record("previous-endpoint-id", vec![optional_string_value(input.previous_endpoint_id)]),
        record("rotation-receipt", vec![optional_ref_value(input.rotation_receipt_ref)]),
        record("key-source", vec![
            record("class", vec![string(input.key_source_class)]),
            record("backend-ref", vec![string(input.backend_ref)]),
            record("source-metadata-ref", vec![optional_ref_value(input.source_metadata_ref)]),
            record("permission", vec![string(input.permission_status.as_str())]),
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

fn derive_endpoint_material(secret_record: &[u8], backend_ref: &str) -> Result<EndpointMaterial> {
    let material = crate::fabric_crypto_identity::transport_endpoint_material(secret_record, backend_ref)?;
    Ok(EndpointMaterial {
        public_key: format!("ed25519:{}", material.public_key),
        endpoint_id: material.endpoint_id,
        secret_ref: material.handle_ref,
    })
}

fn selected_backend_ref(config: &Config, source_class: &str) -> Result<String> {
    if source_class == KEY_SOURCE_MANAGED_BACKEND
        && let Some(backend_ref) = config.secret_backend_ref.as_deref()
    {
        require_ref(backend_ref, "managed secret backend ref")?;
        return Ok(backend_ref.to_string());
    }
    backend_ref(source_class)
}

fn backend_ref(source_class: &str) -> Result<String> {
    crate::preserves_rail::canonical_hash(&record("node-identity-backend", vec![
        record("class", vec![string(source_class)]),
        record("namespace", vec![string(IDENTITY_NAMESPACE_LABEL)]),
    ]))
}

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
