type BtreeSet<T> = std::collections::BTreeSet<T>;
type Cow<'a, B> = std::borrow::Cow<'a, B>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type PathBuf = std::path::PathBuf;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const CONFIDENTIAL_LABEL_SCHEMA: &str = crate::preserves_rail::CONFIDENTIAL_LABEL_SCHEMA;
const ENCRYPTED_REF_SCHEMA: &str = crate::preserves_rail::ENCRYPTED_REF_SCHEMA;
const PRIVATE_BUNDLE_PROFILE_SCHEMA: &str = crate::preserves_rail::PRIVATE_BUNDLE_PROFILE_SCHEMA;
const SECRET_CLEANUP_RECEIPT_SCHEMA: &str = crate::preserves_rail::SECRET_CLEANUP_RECEIPT_SCHEMA;
const SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA: &str = crate::preserves_rail::SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA;
const SECRET_DECRYPT_RECEIPT_SCHEMA: &str = crate::preserves_rail::SECRET_DECRYPT_RECEIPT_SCHEMA;
const SECRET_REDACTION_MARKER_SCHEMA: &str = crate::preserves_rail::SECRET_REDACTION_MARKER_SCHEMA;
const SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA: &str =
    crate::preserves_rail::SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA;
const SECRET_REF_SCHEMA: &str = crate::preserves_rail::SECRET_REF_SCHEMA;
const SECRET_REVEAL_RECEIPT_SCHEMA: &str = crate::preserves_rail::SECRET_REVEAL_RECEIPT_SCHEMA;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

#[cfg(test)]
fn parse_text(source: &str) -> Result<IoValue> {
    crate::preserves_rail::parse_text(source)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const MAX_SECRET_REFS: usize = 32;
const MAX_SECRET_USES: usize = 16;
const MAX_SECRET_DIAGNOSTICS: usize = 16;
const MAX_SECRET_MARKERS: usize = 32;
const DEFAULT_REDACTION_PROFILE: &str = "blake3:09d0a7256e7f74894f4f36bd105b6945ba299095f92af91b35826100bb68ca7d";
const DEFAULT_REDACTION_POLICY: &str = "blake3:6d9a5a7e7b7f33c443f8edbe8c9f74af78e90ce8bf93517ff349be62a06f335a";

const SENSITIVE_RECORD_LABELS: &[&str] = &[
    "secret",
    "confidential",
    "credential",
    "private",
    "encrypted-ref",
    "secret-ref-v1",
    "encrypted-ref-v1",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialLabelInput {
    pub surface: String,
    pub field_path: String,
    pub classification: String,
    pub schema_ref: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialLabel {
    pub label_ref: String,
    pub surface: String,
    pub field_path: String,
    pub classification: String,
    pub schema_ref: String,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRefInput {
    pub secret_id: String,
    pub scope_ref: String,
    pub allowed_uses: Vec<String>,
    pub commitment_ref: String,
    pub encryption_ref: String,
    pub redaction_label_ref: String,
    pub expiry_ref: Option<String>,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRef {
    pub secret_ref: String,
    pub secret_id: String,
    pub scope_ref: String,
    pub allowed_uses: Vec<String>,
    pub commitment_ref: String,
    pub encryption_ref: String,
    pub redaction_label_ref: String,
    pub expiry_ref: Option<String>,
    pub revocation_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedRefInput {
    pub ciphertext_ref: String,
    pub commitment_ref: String,
    pub encryption_ref: String,
    pub schema_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedRef {
    pub encrypted_ref: String,
    pub ciphertext_ref: String,
    pub commitment_ref: String,
    pub encryption_ref: String,
    pub schema_ref: String,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionMarkerInput {
    pub reason: String,
    pub commitment_ref: String,
    pub schema_ref: String,
    pub path_ref: String,
    pub policy_refs: Vec<String>,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionMarker {
    pub marker_ref: String,
    pub reason: String,
    pub commitment_ref: String,
    pub schema_ref: String,
    pub path_ref: String,
    pub policy_refs: Vec<String>,
    pub receipt_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealReceiptInput {
    pub secret_ref: String,
    pub encrypted_ref: Option<String>,
    pub requester_ref: String,
    pub purpose: String,
    pub plaintext_ref: Option<String>,
    pub commitment_ref: String,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_handle_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub secret_ref: String,
    pub encrypted_ref: Option<String>,
    pub requester_ref: String,
    pub purpose: String,
    pub plaintext_ref: Option<String>,
    pub commitment_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptReceiptInput {
    pub encrypted_ref: String,
    pub requester_ref: String,
    pub purpose: String,
    pub plaintext_ref: Option<String>,
    pub commitment_ref: String,
    pub expected_commitment_ref: String,
    pub reveal_receipt_ref: Option<String>,
    pub has_reveal_authority: bool,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_handle_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub encrypted_ref: String,
    pub requester_ref: String,
    pub purpose: String,
    pub plaintext_ref: Option<String>,
    pub commitment_ref: String,
    pub reveal_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionTransformInput {
    pub source_ref: String,
    pub output_ref: String,
    pub policy_refs: Vec<String>,
    pub profile_ref: String,
    pub marker_refs: Vec<String>,
    pub is_gate_preserving: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionTransformReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub source_ref: String,
    pub output_ref: String,
    pub marker_refs: Vec<String>,
    pub is_gate_preserving: bool,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentReplayInput {
    pub expected_commitment_ref: String,
    pub actual_commitment_ref: String,
    pub reveal_receipt_ref: Option<String>,
    pub is_plaintext_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentReplayReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub expected_commitment_ref: String,
    pub actual_commitment_ref: String,
    pub reveal_receipt_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCleanupInput {
    pub secret_ref: String,
    pub revocation_ref: String,
    pub tombstone_ref: String,
    pub retention_refs: Vec<String>,
    pub retention_receipts: Vec<IoValue>,
    pub authority_refs: Vec<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCleanupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub secret_ref: String,
    pub revocation_ref: String,
    pub tombstone_ref: String,
    pub retention_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}
