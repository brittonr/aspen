const KEY_SOURCE_EXPLICIT: &str = "explicit-key";
const KEY_SOURCE_GENERATE: &str = "generate-and-persist";
const KEY_SOURCE_MANAGED_BACKEND: &str = "managed-secret-backend";
const KEY_SOURCE_PERSISTED_FILE: &str = "persisted-file";
const KEY_SOURCE_UNAVAILABLE: &str = "unavailable";
const IROH_ENDPOINT_PREFIX: &str = "iroh:";
const OWNER_ONLY_SECRET_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const GROUP_OR_OTHER_SECRET_PERMISSION_BITS: u32 = 0o077;
const IDENTITY_NAMESPACE_LABEL: &str = "node-state/identity";

type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

#[cfg(test)]
mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn metadata(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::Metadata> {
        std::fs::metadata(path)
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
    pub secret_backend_key: Option<String>,
    pub secret_backend_ref: Option<String>,
    pub require_secret_backend: bool,
    pub allow_generate: bool,
    pub allow_rotation: bool,
    pub rotation_receipt_ref: Option<String>,
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
            secret_backend_key: None,
            secret_backend_ref: None,
            require_secret_backend: false,
            allow_generate: true,
            allow_rotation: false,
            rotation_receipt_ref: None,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrohSecretPermissionStatus {
    NotPresent,
    Restricted,
    Unsupported,
    Unsafe,
}

impl IrohSecretPermissionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotPresent => "not-present",
            Self::Restricted => "restricted-owner-only",
            Self::Unsupported => "unsupported-diagnostic-only",
            Self::Unsafe => "unsafe-shared",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrohSecretSourceDecisionKind {
    LoadExplicit,
    LoadBackend,
    LoadFile,
    GenerateAndPersist,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrohSecretSourceFacts {
    pub explicit_key_present: bool,
    pub managed_secret_present: bool,
    pub managed_secret_required: bool,
    pub persisted_file_present: bool,
    pub persisted_file_permission: IrohSecretPermissionStatus,
    pub generation_allowed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrohSecretSourceDecision {
    pub kind: IrohSecretSourceDecisionKind,
    pub key_source_class: &'static str,
    pub permission_status: IrohSecretPermissionStatus,
    pub diagnostic: &'static str,
}

pub fn resolve_iroh_secret_source(facts: &IrohSecretSourceFacts) -> IrohSecretSourceDecision {
    if facts.explicit_key_present {
        return IrohSecretSourceDecision {
            kind: IrohSecretSourceDecisionKind::LoadExplicit,
            key_source_class: KEY_SOURCE_EXPLICIT,
            permission_status: IrohSecretPermissionStatus::NotPresent,
            diagnostic: "explicit endpoint key metadata selected before shell secret effects",
        };
    }
    if facts.managed_secret_present {
        return IrohSecretSourceDecision {
            kind: IrohSecretSourceDecisionKind::LoadBackend,
            key_source_class: KEY_SOURCE_MANAGED_BACKEND,
            permission_status: IrohSecretPermissionStatus::NotPresent,
            diagnostic: "managed secret backend metadata selected before file fallback",
        };
    }
    if facts.managed_secret_required {
        return IrohSecretSourceDecision {
            kind: IrohSecretSourceDecisionKind::Deny,
            key_source_class: KEY_SOURCE_MANAGED_BACKEND,
            permission_status: IrohSecretPermissionStatus::NotPresent,
            diagnostic: "managed secret backend is required but unavailable",
        };
    }
    if facts.persisted_file_present {
        return match facts.persisted_file_permission {
            IrohSecretPermissionStatus::Unsafe => IrohSecretSourceDecision {
                kind: IrohSecretSourceDecisionKind::Deny,
                key_source_class: KEY_SOURCE_PERSISTED_FILE,
                permission_status: IrohSecretPermissionStatus::Unsafe,
                diagnostic: "persisted endpoint secret permissions are not owner-only",
            },
            permission_status => IrohSecretSourceDecision {
                kind: IrohSecretSourceDecisionKind::LoadFile,
                key_source_class: KEY_SOURCE_PERSISTED_FILE,
                permission_status,
                diagnostic: "persisted endpoint key metadata selected with redacted source diagnostics",
            },
        };
    }
    if facts.generation_allowed {
        return IrohSecretSourceDecision {
            kind: IrohSecretSourceDecisionKind::GenerateAndPersist,
            key_source_class: KEY_SOURCE_GENERATE,
            permission_status: IrohSecretPermissionStatus::NotPresent,
            diagnostic: "first boot generation admitted before persistence side effects",
        };
    }
    IrohSecretSourceDecision {
        kind: IrohSecretSourceDecisionKind::Deny,
        key_source_class: KEY_SOURCE_UNAVAILABLE,
        permission_status: IrohSecretPermissionStatus::NotPresent,
        diagnostic: "persistent endpoint key unavailable and generation is disabled",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrohEndpointObservationDecisionKind {
    Accept,
    Rotate,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohEndpointObservationFacts {
    pub prior_endpoint_id: Option<String>,
    pub observed_endpoint_id: String,
    pub rotation_allowed: bool,
    pub supplied_rotation_receipt_ref: Option<String>,
    pub expected_rotation_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohEndpointObservationDecision {
    pub kind: IrohEndpointObservationDecisionKind,
    pub previous_endpoint_id: Option<String>,
    pub rotation_receipt_ref: Option<String>,
    pub diagnostic: &'static str,
}

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
    secret: &'a str,
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
    match source_decision.kind {
        IrohSecretSourceDecisionKind::LoadExplicit => {
            let explicit_key = config
                .explicit_key
                .as_deref()
                .ok_or_else(|| MoltenError::invalid_harness("explicit endpoint key metadata was selected but missing"))?;
            let material = derive_endpoint_material(explicit_key)?;
            finish_resolution(ResolutionInput {
                config,
                root,
                operation: source_decision.key_source_class,
                secret: explicit_key,
                material: &material,
                backend_ref: &backend_ref,
                source_metadata_ref: &source_metadata_ref,
                permission_status: source_decision.permission_status,
                endpoint_path: &endpoint_path,
                is_first_boot: false,
            })
        }
        IrohSecretSourceDecisionKind::LoadBackend => {
            let backend_key = config
                .secret_backend_key
                .as_deref()
                .ok_or_else(|| MoltenError::invalid_harness("managed endpoint secret backend was selected but missing"))?;
            let material = derive_endpoint_material(backend_key)?;
            finish_resolution(ResolutionInput {
                config,
                root,
                operation: source_decision.key_source_class,
                secret: backend_key,
                material: &material,
                backend_ref: &backend_ref,
                source_metadata_ref: &source_metadata_ref,
                permission_status: source_decision.permission_status,
                endpoint_path: &endpoint_path,
                is_first_boot: false,
            })
        }
        IrohSecretSourceDecisionKind::LoadFile => {
            let secret = read_observed_secret(secret_observation)?;
            let material = derive_endpoint_material(&secret)?;
            finish_resolution(ResolutionInput {
                config,
                root,
                operation: source_decision.key_source_class,
                secret: &secret,
                material: &material,
                backend_ref: &backend_ref,
                source_metadata_ref: &source_metadata_ref,
                permission_status: source_decision.permission_status,
                endpoint_path: &endpoint_path,
                is_first_boot: false,
            })
        }
        IrohSecretSourceDecisionKind::GenerateAndPersist => {
            let secret = generate_secret(&config.node_id)?;
            write_secret_restricted(root, &secret_path, &secret)?;
            let material = derive_endpoint_material(&secret)?;
            finish_resolution(ResolutionInput {
                config,
                root,
                operation: source_decision.key_source_class,
                secret: &secret,
                material: &material,
                backend_ref: &backend_ref,
                source_metadata_ref: &source_metadata_ref,
                permission_status: IrohSecretPermissionStatus::Restricted,
                endpoint_path: &endpoint_path,
                is_first_boot: true,
            })
        }
        IrohSecretSourceDecisionKind::Deny => source_denial(config, &backend_ref, &source_metadata_ref, &source_decision),
    }
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
