
pub fn admit_iroh_endpoint_observation(facts: &IrohEndpointObservationFacts) -> IrohEndpointObservationDecision {
    let Some(prior_endpoint_id) = facts.prior_endpoint_id.clone() else {
        return IrohEndpointObservationDecision {
            kind: IrohEndpointObservationDecisionKind::Accept,
            previous_endpoint_id: None,
            rotation_receipt_ref: None,
            diagnostic: "first admitted endpoint identity for node scope",
        };
    };
    if prior_endpoint_id == facts.observed_endpoint_id {
        return IrohEndpointObservationDecision {
            kind: IrohEndpointObservationDecisionKind::Accept,
            previous_endpoint_id: Some(prior_endpoint_id),
            rotation_receipt_ref: None,
            diagnostic: "observed endpoint identity matches prior node scope",
        };
    }
    if !facts.rotation_allowed {
        return IrohEndpointObservationDecision {
            kind: IrohEndpointObservationDecisionKind::Deny,
            previous_endpoint_id: Some(prior_endpoint_id),
            rotation_receipt_ref: None,
            diagnostic: "endpoint id drift detected; rotation policy is required",
        };
    }
    let Some(supplied_rotation_receipt_ref) = facts.supplied_rotation_receipt_ref.clone() else {
        return IrohEndpointObservationDecision {
            kind: IrohEndpointObservationDecisionKind::Deny,
            previous_endpoint_id: Some(prior_endpoint_id),
            rotation_receipt_ref: None,
            diagnostic: "endpoint id drift detected; rotation receipt is required",
        };
    };
    if facts.expected_rotation_receipt_ref.as_deref() == Some(supplied_rotation_receipt_ref.as_str()) {
        return IrohEndpointObservationDecision {
            kind: IrohEndpointObservationDecisionKind::Rotate,
            previous_endpoint_id: Some(prior_endpoint_id),
            rotation_receipt_ref: Some(supplied_rotation_receipt_ref),
            diagnostic: "endpoint rotation admitted by matching recovery receipt",
        };
    }
    IrohEndpointObservationDecision {
        kind: IrohEndpointObservationDecisionKind::Deny,
        previous_endpoint_id: Some(prior_endpoint_id),
        rotation_receipt_ref: Some(supplied_rotation_receipt_ref),
        diagnostic: "endpoint id drift detected; supplied rotation receipt is stale or mismatched",
    }
}

struct ResolutionInput<'a> {
    config: &'a Config,
    root: &'a crate::node_state::NodeStateNamespace,
    operation: &'a str,
    secret_record: &'a [u8],
    material: &'a EndpointMaterial,
    backend_ref: &'a str,
    source_metadata_ref: &'a str,
    permission_status: IrohSecretPermissionStatus,
    endpoint_path: &'a crate::node_state::NodeStatePath,
    is_first_boot: bool,
}

struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    node_id: &'a str,
    identity_ref: Option<&'a str>,
    endpoint_id: Option<&'a str>,
    previous_endpoint_id: Option<&'a str>,
    rotation_receipt_ref: Option<&'a str>,
    key_source_class: &'a str,
    backend_ref: &'a str,
    source_metadata_ref: Option<&'a str>,
    permission_status: IrohSecretPermissionStatus,
    policy_refs: &'a [String],
    diagnostic: &'a str,
    checks: &'a [&'a str],
}

pub fn resolve(config: &Config) -> Result<Resolution> {
    let root = crate::node_state::NodeStateNamespace::open(
        crate::node_state::NodeStateNamespaceKind::Identity,
        &config.data_dir,
    )?;
    resolve_with_root(config, &root)
}

pub fn resolve_with_root(config: &Config, root: &crate::node_state::NodeStateNamespace) -> Result<Resolution> {
    validate_config(config)?;
    validate_identity_namespace(root)?;
    let secret_path = crate::node_state::NodeStatePath::parse(SECRET_FILE)?;
    let endpoint_path = crate::node_state::NodeStatePath::parse(ENDPOINT_FILE)?;
    let secret_observation = root.observe_file(&secret_path)?;
    let permission_status = secret_file_permission_status(&secret_observation);
    let source_decision = resolve_iroh_secret_source(&IrohSecretSourceFacts {
        explicit_key_present: config.explicit_key.is_some(),
        managed_secret_present: config.secret_backend_key.is_some(),
        managed_secret_required: config.require_secret_backend,
        persisted_file_present: !matches!(
            &secret_observation,
            crate::node_state::NodeStateFileObservation::Missing
        ),
        persisted_file_permission: permission_status,
        generation_allowed: config.allow_generate,
    });
    let backend_ref = selected_backend_ref(config, source_decision.key_source_class)?;
    let source_metadata_ref = source_metadata_ref(source_decision.key_source_class, &backend_ref)?;
    let (secret_record, permission_status, is_first_boot) = match source_decision.kind {
        IrohSecretSourceDecisionKind::LoadExplicit => {
            let explicit_key = config
                .explicit_key
                .as_deref()
                .ok_or_else(|| MoltenError::invalid_harness("explicit endpoint key metadata was selected but missing"))?;
            let secret_record = crate::fabric_crypto_identity::transport_key_record_from_secret_hex(explicit_key)?;
            (secret_record, source_decision.permission_status, false)
        }
        IrohSecretSourceDecisionKind::LoadBackend => {
            let backend_key = config
                .secret_backend_key
                .as_deref()
                .ok_or_else(|| MoltenError::invalid_harness("managed endpoint secret backend was selected but missing"))?;
            let secret_record = crate::fabric_crypto_identity::transport_key_record_from_secret_hex(backend_key)?;
            (secret_record, source_decision.permission_status, false)
        }
        IrohSecretSourceDecisionKind::LoadFile => {
            (read_observed_secret(secret_observation)?, source_decision.permission_status, false)
        }
        IrohSecretSourceDecisionKind::GenerateAndPersist => {
            let secret_record = crate::fabric_crypto_identity::generate_transport_key_record();
            write_secret_restricted(root, &secret_path, &secret_record)?;
            (secret_record, IrohSecretPermissionStatus::Restricted, true)
        }
        IrohSecretSourceDecisionKind::Deny => {
            return source_denial(config, &backend_ref, &source_metadata_ref, &source_decision);
        }
    };
    let material = derive_endpoint_material(&secret_record, &backend_ref)?;
    finish_resolution(ResolutionInput {
        config,
        root,
        operation: source_decision.key_source_class,
        secret_record: &secret_record,
        material: &material,
        backend_ref: &backend_ref,
        source_metadata_ref: &source_metadata_ref,
        permission_status,
        endpoint_path: &endpoint_path,
        is_first_boot,
    })
}

pub fn identity_value(
    config: &Config,
    material: &EndpointMaterial,
    key_source_class: &str,
    backend_ref: &str,
    receipt_refs: &[String],
) -> IoValue {
    record("node-identity-v1", vec![
        string(crate::preserves_rail::NODE_IDENTITY_SCHEMA),
        record("node", vec![
            record("id", vec![string(&config.node_id)]),
            record("display-name", vec![string(&config.display_name)]),
        ]),
        record("endpoint", vec![
            record("public-key", vec![string(&material.public_key)]),
            record("endpoint-id", vec![string(&material.endpoint_id)]),
            record("algorithm", vec![string(KEY_ALGORITHM)]),
        ]),
        record("key-source", vec![
            record("class", vec![string(key_source_class)]),
            record("backend-ref", vec![string(backend_ref)]),
            record("secret-ref", vec![string(&material.secret_ref)]),
        ]),
        record("policy", vec![crate::preserves_rail::sequence(
            config.policy_refs.iter().map(string).collect(),
        )]),
        record("receipts", vec![crate::preserves_rail::sequence(
            receipt_refs.iter().map(string).collect(),
        )]),
        record("checks", vec![crate::preserves_rail::sequence(vec![
            record("check", vec![string("stable-endpoint-id"), string("pass")]),
            record("check", vec![string("no-ambient-authority"), string("pass")]),
            record("check", vec![string("secret-material-redacted"), string("pass")]),
            record("check", vec![string("config-contract"), string("pass")]),
        ])]),
    ])
}

pub fn parse_identity(value: &IoValue) -> Result<Identity> {
    let fields = value
        .collect_simple_record("node-identity-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-identity-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_IDENTITY_SCHEMA, "node identity schema")?;
    let node = value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("node identity missing node field"))?;
    let endpoint = value_to_iovalue(&fields[2]);
    let endpoint_fields = endpoint
        .collect_simple_record("endpoint", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node identity missing endpoint field"))?;
    let key_source = value_to_iovalue(&fields[3]);
    let key_source_fields = key_source
        .collect_simple_record("key-source", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node identity missing key-source field"))?;
    let policy_refs = parse_ref_sequence(&fields[4], "policy")?;
    let receipt_refs = parse_ref_sequence(&fields[5], "receipts")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "no-ambient-authority")?;
    require_check(&checks, "secret-material-redacted")?;
    Ok(Identity {
        identity_ref: crate::preserves_rail::canonical_hash(value)?,
        node_id: record_string(&node_fields[0], "id")?,
        display_name: record_string(&node_fields[1], "display-name")?,
        endpoint_public_key: record_string(&endpoint_fields[0], "public-key")?,
        endpoint_id: record_string(&endpoint_fields[1], "endpoint-id")?,
        key_source_class: record_string(&key_source_fields[0], "class")?,
        backend_ref: record_string(&key_source_fields[1], "backend-ref")?,
        secret_ref: record_string(&key_source_fields[2], "secret-ref")?,
        policy_refs,
        receipt_refs,
        value: value.clone(),
    })
}

pub fn bootstrap_handshake_value(identity: &Identity, peer: &str, policy_refs: &[String]) -> Result<IoValue> {
    if peer.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node bootstrap peer must not be empty"));
    }
    validate_refs(policy_refs, "node bootstrap policy ref")?;
    Ok(record("node-identity-bootstrap-v1", vec![
        string(crate::preserves_rail::NODE_IDENTITY_BOOTSTRAP_SCHEMA),
        record("node", vec![
            record("identity", vec![string(&identity.identity_ref)]),
            record("node-id", vec![string(&identity.node_id)]),
            record("endpoint-id", vec![string(&identity.endpoint_id)]),
        ]),
        record("peer", vec![string(peer)]),
        record("policy", vec![crate::preserves_rail::sequence(
            policy_refs.iter().map(string).collect(),
        )]),
        record("checks", vec![crate::preserves_rail::sequence(vec![
            record("check", vec![string("node-identity-ref-binding"), string("pass")]),
            record("check", vec![string("join-admission-still-required"), string("pass")]),
            record("check", vec![string("identity-grants-no-capabilities"), string("pass")]),
        ])]),
    ]))
}
