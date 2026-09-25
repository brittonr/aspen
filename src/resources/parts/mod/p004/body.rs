// Declarative resource records — canonical resource identity, metadata, status conditions,
// owner refs, finalizers, and deletion gates.
//
// This part adds the canonical resource model DTOs and pure validation functions.
// Type aliases and common helper functions are inherited from p000.

const MAX_LABEL_COUNT: usize = 256;
const MAX_ANNOTATION_COUNT: usize = 1024;
const MAX_LABEL_KEY_LENGTH: usize = 256;
const MAX_LABEL_VALUE_LENGTH: usize = 1024;
const MAX_ANNOTATION_KEY_LENGTH: usize = 512;
const MAX_ANNOTATION_VALUE_LENGTH: usize = 4096;
const MAX_SCOPED_NAME_LENGTH: usize = 512;
const MAX_CONDITIONS: usize = 64;
const MAX_OWNER_REFS: usize = 64;
const MAX_FINALIZERS: usize = 64;
const MAX_EVIDENCE_REFS: usize = 256;
const _: () = assert!(MAX_LABEL_COUNT > 0);
const _: () = assert!(MAX_ANNOTATION_COUNT > 0);
const _: () = assert!(MAX_CONDITIONS > 0);
const _: () = assert!(MAX_OWNER_REFS > 0);
const _: () = assert!(MAX_FINALIZERS > 0);

fn refs_sequence(refs: &[String]) -> IoValue {
    crate::preserves_rail::refs_sequence(refs)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn symbol(name: &'static str) -> IoValue {
    crate::preserves_rail::symbol(name)
}

// ---------------------------------------------------------------------------
// Resource identity DTO
// ---------------------------------------------------------------------------

/// Canonical resource identity components used to compute the stable resource ref.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceIdentity {
    pub resource_type: String,
    pub scope_ref: String,
    pub scoped_name: String,
}

impl ResourceIdentity {
    /// Compute the canonical resource ref from identity bytes.
    pub fn canonical_ref(&self) -> Result<String> {
        let identity_value = record("resource-identity-v1", vec![
            string(&self.resource_type),
            string(&self.scope_ref),
            string(&self.scoped_name),
        ]);
        canonical_hash(&identity_value)
    }

    /// Validate identity fields.
    pub fn validate(&self) -> Result<()> {
        validate_non_empty(&self.resource_type, "resource type")?;
        require_ref(&self.scope_ref, "scope ref")?;
        validate_scoped_name(&self.scoped_name)
    }

    /// Preserves encoding of the identity.
    pub fn to_value(&self) -> IoValue {
        record("resource-identity-v1", vec![
            string(&self.resource_type),
            string(&self.scope_ref),
            string(&self.scoped_name),
        ])
    }
}

// ---------------------------------------------------------------------------
// Resource metadata DTOs
// ---------------------------------------------------------------------------

/// Canonical resource metadata with labels, annotations, owner refs, finalizers, and evidence refs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceMetadata {
    pub labels: std::collections::BTreeMap<String, String>,
    pub annotations: std::collections::BTreeMap<String, String>,
    pub owner_refs: Vec<OwnerRef>,
    pub finalizers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

/// Owner reference for GC and deletion cascading.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwnerRef {
    pub resource_ref: String,
    pub resource_type: String,
    pub block_delete_on_deletion: bool,
}

// ---------------------------------------------------------------------------
// Status condition DTOs
// ---------------------------------------------------------------------------

/// Status condition with observed generation, type, status, reason, message, severity, and evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCondition {
    pub observed_generation: u64,
    pub condition_type: String,
    pub status: ConditionStatus,
    pub reason: String,
    pub severity: ConditionSeverity,
    pub message: String,
    pub evidence_refs: Vec<String>,
    pub observed_state_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionStatus {
    True,
    False,
    Unknown,
}

impl ConditionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionStatus::True => "true",
            ConditionStatus::False => "false",
            ConditionStatus::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl ConditionSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionSeverity::Info => "info",
            ConditionSeverity::Warning => "warning",
            ConditionSeverity::Error => "error",
            ConditionSeverity::Critical => "critical",
        }
    }
}

// ---------------------------------------------------------------------------
// Resource record DTO
// ---------------------------------------------------------------------------

/// Canonical declarative resource record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRecord {
    pub resource_type: String,
    pub resource_ref: String,
    pub scope_ref: String,
    pub name: String,
    pub generation: u64,
    pub desired_ref: String,
    pub observed_ref: Option<String>,
    pub metadata: ResourceMetadata,
    pub evidence_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Deletion gate DTOs
// ---------------------------------------------------------------------------

/// Deletion gate input for owner refs, finalizers, pins, retention, and authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionGateInput {
    pub resource_ref: String,
    pub owner_refs: Vec<OwnerRef>,
    pub finalizers: Vec<String>,
    pub finalizer_cleanup_receipts: Vec<String>,
    pub live_owner_refs: Vec<String>,
    pub pin_refs: Vec<String>,
    pub retention_policy_refs: Vec<String>,
    pub deletion_authority_refs: Vec<String>,
}

/// Deletion gate decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub cleared_blockers: Vec<String>,
    pub unresolved_blockers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_scoped_name(name: &str) -> Result<()> {
    validate_non_empty(name, "scoped name")?;
    if name.len() > MAX_SCOPED_NAME_LENGTH {
        return Err(MoltenError::invalid_harness(format!(
            "scoped name exceeds maximum length {MAX_SCOPED_NAME_LENGTH}: {name}"
        )));
    }
    let is_valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.');
    if !is_valid {
        return Err(MoltenError::invalid_harness(format!(
            "scoped name contains invalid characters: {name}"
        )));
    }
    Ok(())
}

fn validate_label_key(key: &str) -> Result<()> {
    validate_non_empty(key, "label key")?;
    if key.len() > MAX_LABEL_KEY_LENGTH {
        return Err(MoltenError::invalid_harness(format!(
            "label key exceeds maximum length {MAX_LABEL_KEY_LENGTH}: {key}"
        )));
    }
    let is_valid = key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '/');
    if !is_valid {
        return Err(MoltenError::invalid_harness(format!(
            "label key contains invalid characters: {key}"
        )));
    }
    Ok(())
}

fn validate_label_value(value: &str) -> Result<()> {
    if value.len() > MAX_LABEL_VALUE_LENGTH {
        return Err(MoltenError::invalid_harness(format!(
            "label value exceeds maximum length {MAX_LABEL_VALUE_LENGTH}"
        )));
    }
    let is_valid = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
    if !is_valid {
        return Err(MoltenError::invalid_harness(format!(
            "label value contains invalid characters: {value}"
        )));
    }
    Ok(())
}

fn validate_annotation_key(key: &str) -> Result<()> {
    validate_non_empty(key, "annotation key")?;
    if key.len() > MAX_ANNOTATION_KEY_LENGTH {
        return Err(MoltenError::invalid_harness(format!(
            "annotation key exceeds maximum length {MAX_ANNOTATION_KEY_LENGTH}: {key}"
        )));
    }
    Ok(())
}

fn validate_annotation_value(value: &str) -> Result<()> {
    if value.len() > MAX_ANNOTATION_VALUE_LENGTH {
        return Err(MoltenError::invalid_harness(format!(
            "annotation value exceeds maximum length {MAX_ANNOTATION_VALUE_LENGTH}"
        )));
    }
    Ok(())
}
