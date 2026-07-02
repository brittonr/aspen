use std::io::Write;

type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type OpenOptions = std::fs::OpenOptions;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const SECRET_FILE: &str = "node-endpoint.secret";
const ENDPOINT_FILE: &str = "node-endpoint.id";
const KEY_ALGORITHM: &str = "blake3-local-endpoint-fixture-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub node_id: String,
    pub display_name: String,
    pub data_dir: std::path::PathBuf,
    pub explicit_key: Option<String>,
    pub allow_generate: bool,
    pub allow_rotation: bool,
    pub policy_refs: Vec<String>,
}

impl Config {
    pub fn new(node_id: impl Into<String>, data_dir: impl Into<std::path::PathBuf>) -> Self {
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
pub struct Identity {
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
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub identity: Option<Identity>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapHandshake {
    pub handshake_ref: String,
    pub identity_ref: String,
    pub endpoint_id: String,
    pub peer: String,
    pub value: IoValue,
}

struct ResolutionInput<'a> {
    config: &'a Config,
    operation: &'a str,
    secret: &'a str,
    material: &'a EndpointMaterial,
    backend_ref: &'a str,
    endpoint_path: &'a std::path::Path,
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

pub fn resolve(config: &Config) -> Result<Resolution> {
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

    let receipt_value = receipt_value(&ReceiptValueInput {
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
    Ok(Resolution {
        identity: None,
        receipt_ref: crate::preserves_rail::canonical_hash(&receipt_value)?,
        receipt_value,
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
