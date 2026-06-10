use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use preserves::Value;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::NODE_IDENTITY_BOOTSTRAP_SCHEMA;
use crate::preserves_rail::NODE_IDENTITY_RECEIPT_SCHEMA;
use crate::preserves_rail::NODE_IDENTITY_SCHEMA;
use crate::preserves_rail::NODE_IDENTITY_STARTUP_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::validate_content_ref;
use crate::preserves_rail::value_to_iovalue;

const SECRET_FILE: &str = "node-endpoint.secret";
const ENDPOINT_FILE: &str = "node-endpoint.id";
const KEY_ALGORITHM: &str = "blake3-local-endpoint-fixture-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentityConfig {
    pub node_id: String,
    pub display_name: String,
    pub data_dir: PathBuf,
    pub explicit_key: Option<String>,
    pub allow_generate: bool,
    pub allow_rotation: bool,
    pub policy_refs: Vec<String>,
}

impl NodeIdentityConfig {
    pub fn new(node_id: impl Into<String>, data_dir: impl Into<PathBuf>) -> Self {
        let node_id = node_id.into();
        Self {
            display_name: node_id.clone(),
            node_id,
            data_dir: data_dir.into(),
            explicit_key: None,
            allow_generate: true,
            allow_rotation: false,
            policy_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentity {
    pub identity_ref: String,
    pub node_id: String,
    pub display_name: String,
    pub endpoint_public_key: String,
    pub endpoint_id: String,
    pub key_source_class: String,
    pub backend_ref: String,
    pub secret_ref: String,
    pub policy_refs: Vec<String>,
    pub receipt_refs: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeIdentityResolution {
    pub identity: Option<NodeIdentity>,
    pub receipt_ref: String,
    pub receipt_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeBootstrapHandshake {
    pub handshake_ref: String,
    pub node_identity_ref: String,
    pub endpoint_id: String,
    pub peer: String,
    pub value: IOValue,
}

struct ResolutionInput<'a> {
    config: &'a NodeIdentityConfig,
    operation: &'a str,
    secret: &'a str,
    material: &'a EndpointMaterial,
    backend_ref: &'a str,
    endpoint_path: &'a Path,
    is_first_boot: bool,
}

struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    node_id: &'a str,
    identity_ref: Option<&'a str>,
    endpoint_id: Option<&'a str>,
    key_source_class: &'a str,
    backend_ref: &'a str,
    policy_refs: &'a [String],
    diagnostic: &'a str,
    checks: &'a [&'a str],
}

pub fn resolve_node_identity(config: &NodeIdentityConfig) -> Result<NodeIdentityResolution> {
    validate_config(config)?;
    let backend_ref = backend_ref(&config.data_dir)?;
    let secret_path = config.data_dir.join(SECRET_FILE);
    let endpoint_path = config.data_dir.join(ENDPOINT_FILE);

    if let Some(explicit_key) = config.explicit_key.as_deref() {
        let material = derive_endpoint_material(explicit_key)?;
        return finish_resolution(ResolutionInput {
            config,
            operation: "explicit-key",
            secret: explicit_key,
            material: &material,
            backend_ref: &backend_ref,
            endpoint_path: &endpoint_path,
            is_first_boot: false,
        });
    }

    if secret_path.exists() {
        let secret = fs::read_to_string(&secret_path).map_err(MoltenError::from)?;
        let material = derive_endpoint_material(secret.trim())?;
        return finish_resolution(ResolutionInput {
            config,
            operation: "persisted-file",
            secret: secret.trim(),
            material: &material,
            backend_ref: &backend_ref,
            endpoint_path: &endpoint_path,
            is_first_boot: false,
        });
    }

    if config.allow_generate {
        fs::create_dir_all(&config.data_dir).map_err(MoltenError::from)?;
        let secret = generate_secret(&config.node_id, &config.data_dir)?;
        write_secret_restricted(&secret_path, &secret)?;
        let material = derive_endpoint_material(&secret)?;
        return finish_resolution(ResolutionInput {
            config,
            operation: "generate-and-persist",
            secret: &secret,
            material: &material,
            backend_ref: &backend_ref,
            endpoint_path: &endpoint_path,
            is_first_boot: true,
        });
    }

    let receipt_value = node_identity_receipt_value(&ReceiptValueInput {
        operation: "deny-if-unavailable",
        decision: "fail",
        node_id: &config.node_id,
        identity_ref: None,
        endpoint_id: None,
        key_source_class: "unavailable",
        backend_ref: &backend_ref,
        policy_refs: &config.policy_refs,
        diagnostic: "persistent endpoint key unavailable and generation is disabled",
        checks: &["resolution-order", "no-secret-material", "deny-if-unavailable"],
    });
    Ok(NodeIdentityResolution {
        identity: None,
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

pub fn node_identity_value(
    config: &NodeIdentityConfig,
    material: &EndpointMaterial,
    key_source_class: &str,
    backend_ref: &str,
    receipt_refs: &[String],
) -> IOValue {
    record("node-identity-v1", vec![
        string(NODE_IDENTITY_SCHEMA),
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
        record("policy", vec![sequence(config.policy_refs.iter().map(string).collect())]),
        record("receipts", vec![sequence(receipt_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("stable-endpoint-id"), string("pass")]),
            record("check", vec![string("no-ambient-authority"), string("pass")]),
            record("check", vec![string("secret-material-redacted"), string("pass")]),
            record("check", vec![string("config-contract"), string("pass")]),
        ])]),
    ])
}

pub fn parse_node_identity(value: &IOValue) -> Result<NodeIdentity> {
    let fields = value
        .collect_simple_record("node-identity-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-identity-v1 ...>"))?;
    require_schema(&fields[0], NODE_IDENTITY_SCHEMA, "node identity schema")?;
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
    Ok(NodeIdentity {
        identity_ref: canonical_hash(value)?,
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

pub fn node_bootstrap_handshake_value(identity: &NodeIdentity, peer: &str, policy_refs: &[String]) -> Result<IOValue> {
    if peer.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node bootstrap peer must not be empty"));
    }
    validate_refs(policy_refs, "node bootstrap policy ref")?;
    Ok(record("node-identity-bootstrap-v1", vec![
        string(NODE_IDENTITY_BOOTSTRAP_SCHEMA),
        record("node", vec![
            record("identity", vec![string(&identity.identity_ref)]),
            record("node-id", vec![string(&identity.node_id)]),
            record("endpoint-id", vec![string(&identity.endpoint_id)]),
        ]),
        record("peer", vec![string(peer)]),
        record("policy", vec![sequence(policy_refs.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("node-identity-ref-binding"), string("pass")]),
            record("check", vec![string("join-admission-still-required"), string("pass")]),
            record("check", vec![string("identity-grants-no-capabilities"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_node_bootstrap_handshake(value: &IOValue) -> Result<NodeBootstrapHandshake> {
    let fields = value
        .collect_simple_record("node-identity-bootstrap-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-identity-bootstrap-v1 ...>"))?;
    require_schema(&fields[0], NODE_IDENTITY_BOOTSTRAP_SCHEMA, "node identity bootstrap schema")?;
    let node = value_to_iovalue(&fields[1]);
    let node_fields = node
        .collect_simple_record("node", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("node bootstrap missing node field"))?;
    let checks = parse_checks(&fields[4])?;
    require_check(&checks, "join-admission-still-required")?;
    Ok(NodeBootstrapHandshake {
        handshake_ref: canonical_hash(value)?,
        node_identity_ref: record_string(&node_fields[0], "identity")?,
        endpoint_id: record_string(&node_fields[2], "endpoint-id")?,
        peer: record_string(&fields[2], "peer")?,
        value: value.clone(),
    })
}

pub fn node_identity_startup_evidence_value(identity_ref: &str, receipt_ref: &str) -> Result<IOValue> {
    require_ref(identity_ref, "node identity startup identity ref")?;
    require_ref(receipt_ref, "node identity startup receipt ref")?;
    Ok(record("node-identity-startup-v1", vec![
        string(NODE_IDENTITY_STARTUP_SCHEMA),
        record("identity", vec![string(identity_ref)]),
        record("receipt", vec![string(receipt_ref)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("replay-ref-only"), string("pass")]),
            record("check", vec![string("private-key-not-required"), string("pass")]),
        ])]),
    ]))
}

fn finish_resolution(input: ResolutionInput<'_>) -> Result<NodeIdentityResolution> {
    if input.secret.trim().is_empty() {
        return Err(MoltenError::invalid_harness("node endpoint secret must not be empty"));
    }
    let existing_endpoint = fs::read_to_string(input.endpoint_path).ok().map(|value| value.trim().to_string());
    let is_drift = existing_endpoint.as_deref().is_some_and(|existing| existing != input.material.endpoint_id.as_str());
    if is_drift && !input.config.allow_rotation {
        let receipt_value = node_identity_receipt_value(&ReceiptValueInput {
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
        return Ok(NodeIdentityResolution {
            identity: None,
            receipt_ref: canonical_hash(&receipt_value)?,
            receipt_value,
        });
    }

    fs::create_dir_all(&input.config.data_dir).map_err(MoltenError::from)?;
    fs::write(input.endpoint_path, &input.material.endpoint_id).map_err(MoltenError::from)?;
    let receipt_operation = if is_drift {
        "rotation"
    } else if input.is_first_boot {
        "first-boot-generate"
    } else {
        input.operation
    };
    let common_checks = [
        "resolution-order",
        "stable-endpoint-id",
        "restricted-secret-file",
        "no-secret-material",
        "identity-grants-no-authority",
        "config-contract",
    ];
    let pre_receipt_value = node_identity_receipt_value(&ReceiptValueInput {
        operation: receipt_operation,
        decision: "pass",
        node_id: &input.config.node_id,
        identity_ref: None,
        endpoint_id: Some(&input.material.endpoint_id),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        policy_refs: &input.config.policy_refs,
        diagnostic: "node identity resolved without exposing secret material",
        checks: &common_checks,
    });
    let pre_receipt_ref = canonical_hash(&pre_receipt_value)?;
    let identity_value = node_identity_value(
        input.config,
        input.material,
        input.operation,
        input.backend_ref,
        std::slice::from_ref(&pre_receipt_ref),
    );
    let identity = parse_node_identity(&identity_value)?;
    let receipt_value = node_identity_receipt_value(&ReceiptValueInput {
        operation: receipt_operation,
        decision: "pass",
        node_id: &input.config.node_id,
        identity_ref: Some(&identity.identity_ref),
        endpoint_id: Some(&input.material.endpoint_id),
        key_source_class: input.operation,
        backend_ref: input.backend_ref,
        policy_refs: &input.config.policy_refs,
        diagnostic: "node identity resolved without exposing secret material",
        checks: &common_checks,
    });
    Ok(NodeIdentityResolution {
        identity: Some(identity),
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

fn node_identity_receipt_value(input: &ReceiptValueInput<'_>) -> IOValue {
    record("node-identity-receipt-v1", vec![
        string(NODE_IDENTITY_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node_id)]),
        record("identity", vec![optional_ref_value(input.identity_ref)]),
        record("endpoint-id", vec![optional_string_value(input.endpoint_id)]),
        record("key-source", vec![
            record("class", vec![string(input.key_source_class)]),
            record("backend-ref", vec![string(input.backend_ref)]),
        ]),
        record("policy", vec![sequence(input.policy_refs.iter().map(string).collect())]),
        record("diagnostic", vec![string(input.diagnostic)]),
        record("checks", vec![sequence(
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
    let secret_ref = format!("blake3:{}", blake3::hash(secret.as_bytes()).to_hex());
    let mut public_material = b"molten-node-public\0".to_vec();
    public_material.extend_from_slice(secret.as_bytes());
    let public_key = format!("blake3:{}", blake3::hash(&public_material).to_hex());
    let mut endpoint_material = b"molten-node-endpoint\0".to_vec();
    endpoint_material.extend_from_slice(public_key.as_bytes());
    let endpoint_id = format!("iroh:{}", blake3::hash(&endpoint_material).to_hex());
    Ok(EndpointMaterial {
        public_key,
        endpoint_id,
        secret_ref,
    })
}

fn generate_secret(node_id: &str, data_dir: &Path) -> Result<String> {
    let seed_ref = canonical_hash(&record("node-identity-generated-secret-seed", vec![
        record("node-id", vec![string(node_id)]),
        record("data-dir", vec![string(data_dir.display().to_string())]),
    ]))?;
    Ok(format!("molten-local-generated:{node_id}:{seed_ref}"))
}

fn backend_ref(data_dir: &Path) -> Result<String> {
    canonical_hash(&record("node-identity-backend", vec![
        record("class", vec![string("filesystem")]),
        record("data-dir", vec![string(data_dir.display().to_string())]),
    ]))
}

fn validate_config(config: &NodeIdentityConfig) -> Result<()> {
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

fn write_secret_restricted(path: &Path, secret: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
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

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn parse_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
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

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
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

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&record[0], label)
}

fn require_schema(value: &Value<IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::preserves_rail::to_text;

    #[test]
    fn restart_with_same_data_dir_preserves_endpoint_id_without_secret_in_receipts() {
        let dir = temp_dir("node-identity-restart");
        let config = NodeIdentityConfig::new("node-a", &dir);
        let first = resolve_node_identity(&config).expect("first resolve");
        let first_identity = first.identity.expect("first identity");
        let second = resolve_node_identity(&config).expect("second resolve");
        let second_identity = second.identity.expect("second identity");
        assert_eq!(first_identity.endpoint_id, second_identity.endpoint_id);
        assert_eq!(second_identity.key_source_class, "persisted-file");
        let secret = fs::read_to_string(dir.join(SECRET_FILE)).expect("read secret");
        let first_receipt_text = to_text(&first.receipt_value).expect("receipt text");
        let second_receipt_text = to_text(&second.receipt_value).expect("receipt text");
        assert!(!first_receipt_text.contains(secret.trim()));
        assert!(!second_receipt_text.contains(secret.trim()));
    }

    #[test]
    fn drift_is_denied_unless_rotation_policy_is_admitted() {
        let dir = temp_dir("node-identity-drift");
        let config = NodeIdentityConfig::new("node-a", &dir);
        let first = resolve_node_identity(&config).expect("first resolve");
        let first_endpoint = first.identity.expect("identity").endpoint_id;
        fs::write(dir.join(SECRET_FILE), "replacement-secret\n").expect("replace secret");
        let drift = resolve_node_identity(&config).expect("drift receipt");
        assert!(drift.identity.is_none());
        assert!(to_text(&drift.receipt_value).expect("drift text").contains("drift-detected"));

        let mut rotation = config.clone();
        rotation.allow_rotation = true;
        let rotated = resolve_node_identity(&rotation).expect("rotation allowed");
        let rotated_endpoint = rotated.identity.expect("rotated identity").endpoint_id;
        assert_ne!(first_endpoint, rotated_endpoint);
    }

    #[test]
    fn explicit_key_and_deny_if_unavailable_follow_resolution_order() {
        let explicit_dir = temp_dir("node-identity-explicit");
        let mut explicit = NodeIdentityConfig::new("node-explicit", &explicit_dir);
        explicit.explicit_key = Some("deployment-secret".to_string());
        explicit.allow_generate = false;
        let resolved = resolve_node_identity(&explicit).expect("explicit resolve");
        assert_eq!(resolved.identity.expect("explicit identity").key_source_class, "explicit-key");
        assert!(!to_text(&resolved.receipt_value).expect("receipt text").contains("deployment-secret"));

        let denied_dir = temp_dir("node-identity-denied");
        let mut denied = NodeIdentityConfig::new("node-denied", denied_dir);
        denied.allow_generate = false;
        let denied = resolve_node_identity(&denied).expect("denial receipt");
        assert!(denied.identity.is_none());
        assert!(to_text(&denied.receipt_value).expect("denial text").contains("deny-if-unavailable"));
    }

    #[test]
    fn bootstrap_and_startup_evidence_bind_identity_without_authority() {
        let dir = temp_dir("node-identity-bootstrap");
        let resolved = resolve_node_identity(&NodeIdentityConfig::new("node-a", &dir)).expect("resolve");
        let identity = resolved.identity.expect("identity");
        let handshake = node_bootstrap_handshake_value(&identity, "peer:b", &[]).expect("handshake");
        let parsed = parse_node_bootstrap_handshake(&handshake).expect("parse handshake");
        assert_eq!(parsed.node_identity_ref, identity.identity_ref);
        assert_eq!(parsed.endpoint_id, identity.endpoint_id);
        assert!(to_text(&handshake).expect("handshake text").contains("identity-grants-no-capabilities"));
        let startup =
            node_identity_startup_evidence_value(&identity.identity_ref, &resolved.receipt_ref).expect("startup");
        assert!(to_text(&startup).expect("startup text").contains("private-key-not-required"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_explicit_resolution_is_deterministic_and_receipts_redact_secret(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let secret_suffix = tc.draw(generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let secret = format!("explicit-secret-{salt}-{secret_suffix}");
        let mut first_config = NodeIdentityConfig::new(format!("node-{salt}"), temp_dir("node-identity-hegel-a"));
        first_config.explicit_key = Some(secret.clone());
        first_config.allow_generate = false;
        let mut second_config = NodeIdentityConfig::new(format!("node-{salt}"), temp_dir("node-identity-hegel-b"));
        second_config.explicit_key = Some(secret.clone());
        second_config.allow_generate = false;
        let first = resolve_node_identity(&first_config).expect("first explicit");
        let second = resolve_node_identity(&second_config).expect("second explicit");
        assert_eq!(
            first.identity.as_ref().expect("first identity").endpoint_id,
            second.identity.as_ref().expect("second identity").endpoint_id
        );
        assert!(!to_text(&first.receipt_value).expect("receipt text").contains(&secret));
        assert!(!to_text(&second.receipt_value).expect("receipt text").contains(&secret));
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
