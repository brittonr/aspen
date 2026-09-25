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
const KEY_ALGORITHM: &str = "ed25519-iroh-v1";

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
