use crate::retention;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateBundleProfileInput {
    pub profile_ref: String,
    pub encrypted_refs: Vec<String>,
    pub reveal_receipt_refs: Vec<String>,
    pub transform_receipt_ref: String,
    pub is_gate_preserving: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateBundleProfile {
    pub profile_ref: String,
    pub encrypted_refs: Vec<String>,
    pub reveal_receipt_refs: Vec<String>,
    pub transform_receipt_ref: String,
    pub is_gate_preserving: bool,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedValue {
    pub value: IoValue,
    pub marker: Option<RedactionMarker>,
    pub transform_receipt: Option<RedactionTransformReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretsFixtureRun {
    pub value: IoValue,
    pub report_ref: String,
    pub secret: SecretRef,
    pub encrypted: EncryptedRef,
    pub marker: RedactionMarker,
    pub transform: RedactionTransformReceipt,
    pub reveal_denied: RevealReceipt,
    pub reveal_pass: RevealReceipt,
    pub decrypt_denied: DecryptReceipt,
    pub decrypt_pass: DecryptReceipt,
    pub replay: CommitmentReplayReceipt,
    pub cleanup: SecretCleanupReceipt,
    pub private_bundle: PrivateBundleProfile,
    pub evidence_values: Vec<IoValue>,
}

pub fn confidential_label_value(input: &ConfidentialLabelInput) -> Result<IoValue> {
    validate_non_empty(&input.surface, "confidential label surface")?;
    validate_non_empty(&input.field_path, "confidential label field path")?;
    validate_classification(&input.classification)?;
    validate_ref(&input.schema_ref, "confidential label schema ref")?;
    validate_refs(&input.policy_refs, "confidential label policy ref")?;
    Ok(record("confidential-label-v1", vec![
        string(CONFIDENTIAL_LABEL_SCHEMA),
        record("surface", vec![string(&input.surface)]),
        record("field-path", vec![string(&input.field_path)]),
        record("classification", vec![string(&input.classification)]),
        record("schema", vec![string(&input.schema_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&[
            ("field-label-metadata", "pass"),
            ("no-plaintext-default", "pass"),
            ("policy-bound", "pass"),
        ]),
    ]))
}

pub fn parse_confidential_label(value: &IoValue) -> Result<ConfidentialLabel> {
    let fields = simple_record(value, "confidential-label-v1", 7)?;
    require_schema(&fields[0], CONFIDENTIAL_LABEL_SCHEMA, "confidential label")?;
    let surface = record_string(&fields[1], "surface", "confidential label surface")?;
    let field_path = record_string(&fields[2], "field-path", "confidential label field path")?;
    let classification = record_string(&fields[3], "classification", "confidential label classification")?;
    validate_classification(&classification)?;
    let schema_ref = record_ref(&fields[4], "schema", "confidential label schema ref")?;
    let policy_refs = record_refs(&fields[5], "policy", "confidential label policy refs")?;
    require_checks(&fields[6], &["field-label-metadata", "no-plaintext-default", "policy-bound"])?;
    Ok(ConfidentialLabel {
        label_ref: canonical_hash(value)?,
        surface,
        field_path,
        classification,
        schema_ref,
        policy_refs,
        value: value.clone(),
    })
}

pub fn secret_ref_value(input: &SecretRefInput) -> Result<IoValue> {
    validate_non_empty(&input.secret_id, "secret id")?;
    validate_ref(&input.scope_ref, "secret scope ref")?;
    validate_allowed_uses(&input.allowed_uses)?;
    validate_ref(&input.commitment_ref, "secret commitment ref")?;
    validate_ref(&input.encryption_ref, "secret encryption ref")?;
    validate_ref(&input.redaction_label_ref, "secret redaction label ref")?;
    validate_optional_ref(input.expiry_ref.as_deref(), "secret expiry ref")?;
    validate_refs(&input.revocation_refs, "secret revocation ref")?;
    validate_refs(&input.evidence_refs, "secret evidence ref")?;
    ensure_count_at_most(input.allowed_uses.len(), MAX_SECRET_USES, "secret allowed uses")?;
    Ok(record("secret-ref-v1", vec![
        string(SECRET_REF_SCHEMA),
        record("secret-id", vec![string(&input.secret_id)]),
        record("scope", vec![string(&input.scope_ref)]),
        record("allowed-use", vec![strings_sequence(&input.allowed_uses)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        record("encryption", vec![string(&input.encryption_ref)]),
        record("redaction-label", vec![string(&input.redaction_label_ref)]),
        record("expiry", vec![optional_ref_value(input.expiry_ref.as_deref())]),
        record("revocation", vec![refs_sequence(&input.revocation_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&[
            ("canonical-secret-ref", "pass"),
            ("no-plaintext-default", "pass"),
            ("possession-not-authority", "pass"),
        ]),
    ]))
}

pub fn parse_secret_ref(value: &IoValue) -> Result<SecretRef> {
    let fields = simple_record(value, "secret-ref-v1", 11)?;
    require_schema(&fields[0], SECRET_REF_SCHEMA, "secret ref")?;
    let secret_id = record_string(&fields[1], "secret-id", "secret id")?;
    let scope_ref = record_ref(&fields[2], "scope", "secret scope ref")?;
    let allowed_uses = record_strings(&fields[3], "allowed-use", "secret allowed uses")?;
    validate_allowed_uses(&allowed_uses)?;
    let commitment_ref = record_ref(&fields[4], "commitment", "secret commitment ref")?;
    let encryption_ref = record_ref(&fields[5], "encryption", "secret encryption ref")?;
    let redaction_label_ref = record_ref(&fields[6], "redaction-label", "secret redaction label ref")?;
    let expiry_ref = record_optional_ref(&fields[7], "expiry", "secret expiry ref")?;
    let revocation_refs = record_refs(&fields[8], "revocation", "secret revocation refs")?;
    let evidence_refs = record_refs(&fields[9], "evidence", "secret evidence refs")?;
    require_checks(&fields[10], &[
        "canonical-secret-ref",
        "no-plaintext-default",
        "possession-not-authority",
    ])?;
    Ok(SecretRef {
        secret_ref: canonical_hash(value)?,
        secret_id,
        scope_ref,
        allowed_uses,
        commitment_ref,
        encryption_ref,
        redaction_label_ref,
        expiry_ref,
        revocation_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn encrypted_ref_value(input: &EncryptedRefInput) -> Result<IoValue> {
    validate_ref(&input.ciphertext_ref, "encrypted ref ciphertext")?;
    validate_ref(&input.commitment_ref, "encrypted ref commitment")?;
    validate_ref(&input.encryption_ref, "encrypted ref encryption profile")?;
    validate_ref(&input.schema_ref, "encrypted ref schema")?;
    validate_refs(&input.policy_refs, "encrypted ref policy")?;
    validate_refs(&input.evidence_refs, "encrypted ref evidence")?;
    Ok(record("encrypted-ref-v1", vec![
        string(ENCRYPTED_REF_SCHEMA),
        record("ciphertext", vec![string(&input.ciphertext_ref)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        record("encryption", vec![string(&input.encryption_ref)]),
        record("schema", vec![string(&input.schema_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("evidence", vec![refs_sequence(&input.evidence_refs)]),
        checks_value(&[
            ("ciphertext-not-authority", "pass"),
            ("commitment-bound", "pass"),
            ("schema-bound", "pass"),
        ]),
    ]))
}

pub fn parse_encrypted_ref(value: &IoValue) -> Result<EncryptedRef> {
    let fields = simple_record(value, "encrypted-ref-v1", 8)?;
    require_schema(&fields[0], ENCRYPTED_REF_SCHEMA, "encrypted ref")?;
    let ciphertext_ref = record_ref(&fields[1], "ciphertext", "encrypted ref ciphertext")?;
    let commitment_ref = record_ref(&fields[2], "commitment", "encrypted ref commitment")?;
    let encryption_ref = record_ref(&fields[3], "encryption", "encrypted ref encryption")?;
    let schema_ref = record_ref(&fields[4], "schema", "encrypted ref schema")?;
    let policy_refs = record_refs(&fields[5], "policy", "encrypted ref policy")?;
    let evidence_refs = record_refs(&fields[6], "evidence", "encrypted ref evidence")?;
    require_checks(&fields[7], &["ciphertext-not-authority", "commitment-bound", "schema-bound"])?;
    Ok(EncryptedRef {
        encrypted_ref: canonical_hash(value)?,
        ciphertext_ref,
        commitment_ref,
        encryption_ref,
        schema_ref,
        policy_refs,
        evidence_refs,
        value: value.clone(),
    })
}

pub fn redaction_marker_value(input: &RedactionMarkerInput) -> Result<IoValue> {
    validate_redaction_reason(&input.reason)?;
    validate_ref(&input.commitment_ref, "redaction marker commitment")?;
    validate_ref(&input.schema_ref, "redaction marker schema")?;
    validate_ref(&input.path_ref, "redaction marker path")?;
    validate_refs(&input.policy_refs, "redaction marker policy")?;
    validate_ref(&input.receipt_ref, "redaction marker receipt")?;
    Ok(record("redaction-marker-v1", vec![
        string(SECRET_REDACTION_MARKER_SCHEMA),
        record("reason", vec![string(&input.reason)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        record("schema", vec![string(&input.schema_ref)]),
        record("path", vec![string(&input.path_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("receipt", vec![string(&input.receipt_ref)]),
        checks_value(&[
            ("safe-commitment-bound", "pass"),
            ("receipt-bound", "pass"),
            ("plaintext-omitted", "pass"),
        ]),
    ]))
}

pub fn parse_redaction_marker(value: &IoValue) -> Result<RedactionMarker> {
    let fields = simple_record(value, "redaction-marker-v1", 8)?;
    require_schema(&fields[0], SECRET_REDACTION_MARKER_SCHEMA, "redaction marker")?;
    let reason = record_string(&fields[1], "reason", "redaction reason")?;
    validate_redaction_reason(&reason)?;
    let commitment_ref = record_ref(&fields[2], "commitment", "redaction commitment")?;
    let schema_ref = record_ref(&fields[3], "schema", "redaction schema")?;
    let path_ref = record_ref(&fields[4], "path", "redaction path")?;
    let policy_refs = record_refs(&fields[5], "policy", "redaction policy")?;
    let receipt_ref = record_ref(&fields[6], "receipt", "redaction receipt")?;
    require_checks(&fields[7], &["safe-commitment-bound", "receipt-bound", "plaintext-omitted"])?;
    Ok(RedactionMarker {
        marker_ref: canonical_hash(value)?,
        reason,
        commitment_ref,
        schema_ref,
        path_ref,
        policy_refs,
        receipt_ref,
        value: value.clone(),
    })
}

pub fn reveal_receipt_value(input: &RevealReceiptInput) -> Result<IoValue> {
    validate_ref(&input.secret_ref, "reveal secret ref")?;
    validate_optional_ref(input.encrypted_ref.as_deref(), "reveal encrypted ref")?;
    validate_ref(&input.requester_ref, "reveal requester ref")?;
    validate_purpose(&input.purpose)?;
    validate_optional_ref(input.plaintext_ref.as_deref(), "reveal plaintext ref")?;
    validate_ref(&input.commitment_ref, "reveal commitment ref")?;
    validate_refs(&input.authority_refs, "reveal authority ref")?;
    validate_refs(&input.policy_refs, "reveal policy ref")?;
    validate_refs(&input.resource_refs, "reveal resource ref")?;
    validate_refs(&input.effect_handle_refs, "reveal effect handle ref")?;
    validate_refs(&input.revocation_refs, "reveal revocation ref")?;
    let mut diagnostics = Vec::new();
    collect_gate_diagnostics(
        AccessGateInput {
            authority_refs: &input.authority_refs,
            policy_refs: &input.policy_refs,
            resource_refs: &input.resource_refs,
            effect_handle_refs: &input.effect_handle_refs,
            revocation_refs: &input.revocation_refs,
            operation: "reveal",
        },
        &mut diagnostics,
    )?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let plaintext_ref = if decision == "pass" {
        input.plaintext_ref.as_deref()
    } else {
        None
    };
    Ok(record("reveal-receipt-v1", vec![
        string(SECRET_REVEAL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("secret", vec![string(&input.secret_ref)]),
        record("encrypted-ref", vec![optional_ref_value(input.encrypted_ref.as_deref())]),
        record("requester", vec![string(&input.requester_ref)]),
        record("purpose", vec![string(&input.purpose)]),
        record("plaintext-ref", vec![optional_ref_value(plaintext_ref)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        diagnostics_value(&diagnostics),
        checks_value(&reveal_checks(decision, input.encrypted_ref.is_some())),
    ]))
}

pub fn parse_reveal_receipt(value: &IoValue) -> Result<RevealReceipt> {
    let fields =
        simple_record(value, "reveal-receipt-v1", 10).or_else(|_| simple_record(value, "reveal-receipt-v1", 9))?;
    let arity = fields.fields_iter().count();
    require_schema(&fields[0], SECRET_REVEAL_RECEIPT_SCHEMA, "reveal receipt")?;
    let decision = record_decision(&fields[1])?;
    let secret_ref = record_ref(&fields[2], "secret", "reveal secret ref")?;
    let (encrypted_ref, requester_ref, purpose, plaintext_ref, commitment_ref, diagnostics, checks_index) =
        if arity == 10 {
            (
                record_optional_ref(&fields[3], "encrypted-ref", "reveal encrypted ref")?,
                record_ref(&fields[4], "requester", "reveal requester ref")?,
                record_string(&fields[5], "purpose", "reveal purpose")?,
                record_optional_ref(&fields[6], "plaintext-ref", "reveal plaintext ref")?,
                record_ref(&fields[7], "commitment", "reveal commitment")?,
                parse_diagnostics(&fields[8])?,
                9usize,
            )
        } else {
            (
                None,
                record_ref(&fields[3], "requester", "reveal requester ref")?,
                record_string(&fields[4], "purpose", "reveal purpose")?,
                record_optional_ref(&fields[5], "plaintext-ref", "reveal plaintext ref")?,
                record_ref(&fields[6], "commitment", "reveal commitment")?,
                parse_diagnostics(&fields[7])?,
                8usize,
            )
        };
    validate_purpose(&purpose)?;
    let required = if decision == "pass" {
        [
            "authorized-reveal",
            "policy-bound",
            "resource-bound",
            "effect-handle-bound",
        ]
    } else {
        [
            "deny-without-authority",
            "no-plaintext-on-deny",
            "ciphertext-not-authority",
            "audit-receipt",
        ]
    };
    require_checks(&fields[checks_index], &required)?;
    if encrypted_ref.is_some() {
        require_checks(&fields[checks_index], &["encrypted-ref-bound"])?;
    }
    Ok(RevealReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        secret_ref,
        encrypted_ref,
        requester_ref,
        purpose,
        plaintext_ref,
        commitment_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn decrypt_receipt_value(input: &DecryptReceiptInput) -> Result<IoValue> {
    validate_ref(&input.encrypted_ref, "decrypt encrypted ref")?;
    validate_ref(&input.requester_ref, "decrypt requester ref")?;
    validate_purpose(&input.purpose)?;
    validate_optional_ref(input.plaintext_ref.as_deref(), "decrypt plaintext ref")?;
    validate_ref(&input.commitment_ref, "decrypt commitment ref")?;
    validate_ref(&input.expected_commitment_ref, "decrypt expected commitment ref")?;
    validate_optional_ref(input.reveal_receipt_ref.as_deref(), "decrypt reveal receipt ref")?;
    validate_refs(&input.authority_refs, "decrypt authority ref")?;
    validate_refs(&input.policy_refs, "decrypt policy ref")?;
    validate_refs(&input.resource_refs, "decrypt resource ref")?;
    validate_refs(&input.effect_handle_refs, "decrypt effect handle ref")?;
    let mut diagnostics = Vec::new();
    collect_gate_diagnostics(
        AccessGateInput {
            authority_refs: &input.authority_refs,
            policy_refs: &input.policy_refs,
            resource_refs: &input.resource_refs,
            effect_handle_refs: &input.effect_handle_refs,
            revocation_refs: &[],
            operation: "decrypt",
        },
        &mut diagnostics,
    )?;
    if !input.has_reveal_authority || input.reveal_receipt_ref.is_none() {
        diagnostics.push_limited(
            "decrypt requires a passing reveal receipt; encrypted refs alone are not authority".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "decrypt diagnostics",
        )?;
    }
    if input.commitment_ref != input.expected_commitment_ref {
        diagnostics.push_limited(
            "decrypt commitment does not match encrypted ref commitment".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "decrypt diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let plaintext_ref = if decision == "pass" {
        input.plaintext_ref.as_deref()
    } else {
        None
    };
    Ok(record("decrypt-receipt-v1", vec![
        string(SECRET_DECRYPT_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("encrypted-ref", vec![string(&input.encrypted_ref)]),
        record("requester", vec![string(&input.requester_ref)]),
        record("purpose", vec![string(&input.purpose)]),
        record("plaintext-ref", vec![optional_ref_value(plaintext_ref)]),
        record("commitment", vec![string(&input.commitment_ref)]),
        record("reveal-receipt", vec![optional_ref_value(input.reveal_receipt_ref.as_deref())]),
        diagnostics_value(&diagnostics),
        checks_value(&decrypt_checks(decision)),
    ]))
}

pub fn parse_decrypt_receipt(value: &IoValue) -> Result<DecryptReceipt> {
    let fields = simple_record(value, "decrypt-receipt-v1", 10)?;
    require_schema(&fields[0], SECRET_DECRYPT_RECEIPT_SCHEMA, "decrypt receipt")?;
    let decision = record_decision(&fields[1])?;
    let encrypted_ref = record_ref(&fields[2], "encrypted-ref", "decrypt encrypted ref")?;
    let requester_ref = record_ref(&fields[3], "requester", "decrypt requester ref")?;
    let purpose = record_string(&fields[4], "purpose", "decrypt purpose")?;
    validate_purpose(&purpose)?;
    let plaintext_ref = record_optional_ref(&fields[5], "plaintext-ref", "decrypt plaintext ref")?;
    let commitment_ref = record_ref(&fields[6], "commitment", "decrypt commitment")?;
    let reveal_receipt_ref = record_optional_ref(&fields[7], "reveal-receipt", "decrypt reveal receipt ref")?;
    let diagnostics = parse_diagnostics(&fields[8])?;
    let required = if decision == "pass" {
        [
            "authorized-decrypt",
            "reveal-receipt-bound",
            "commitment-match",
            "effect-handle-bound",
        ]
    } else {
        [
            "deny-without-reveal",
            "no-plaintext-on-deny",
            "ciphertext-not-authority",
            "audit-receipt",
        ]
    };
    require_checks(&fields[9], &required)?;
    Ok(DecryptReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        encrypted_ref,
        requester_ref,
        purpose,
        plaintext_ref,
        commitment_ref,
        reveal_receipt_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn redaction_transform_receipt_value(input: &RedactionTransformInput) -> Result<IoValue> {
    validate_ref(&input.source_ref, "redaction source ref")?;
    validate_ref(&input.output_ref, "redaction output ref")?;
    validate_refs(&input.policy_refs, "redaction policy ref")?;
    validate_ref(&input.profile_ref, "redaction profile ref")?;
    validate_refs(&input.marker_refs, "redaction marker ref")?;
    validate_diagnostics(&input.diagnostics, "redaction diagnostics")?;
    let decision = if input.diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("redaction-transform-receipt-v1", vec![
        string(SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("source", vec![string(&input.source_ref)]),
        record("output", vec![string(&input.output_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("profile", vec![string(&input.profile_ref)]),
        record("markers", vec![refs_sequence(&input.marker_refs)]),
        record("gate-preserving", vec![bool_value(input.is_gate_preserving)]),
        diagnostics_value(&input.diagnostics),
        checks_value(&redaction_transform_checks(decision, input.is_gate_preserving)),
    ]))
}

pub fn parse_redaction_transform_receipt(value: &IoValue) -> Result<RedactionTransformReceipt> {
    let fields = simple_record(value, "redaction-transform-receipt-v1", 10)?;
    require_schema(&fields[0], SECRET_REDACTION_TRANSFORM_RECEIPT_SCHEMA, "redaction transform")?;
    let decision = record_decision(&fields[1])?;
    let source_ref = record_ref(&fields[2], "source", "redaction source")?;
    let output_ref = record_ref(&fields[3], "output", "redaction output")?;
    let _policy_refs = record_refs(&fields[4], "policy", "redaction policy")?;
    let _profile_ref = record_ref(&fields[5], "profile", "redaction profile")?;
    let marker_refs = record_refs(&fields[6], "markers", "redaction markers")?;
    let is_gate_preserving = record_bool(&fields[7], "gate-preserving", "redaction gate preserving")?;
    let diagnostics = parse_diagnostics(&fields[8])?;
    if is_gate_preserving {
        require_checks(&fields[9], &[
            "source-ref-bound",
            "output-ref-bound",
            "marker-ref-bound",
            "semantic-evidence-preserved",
        ])?;
    } else {
        require_checks(&fields[9], &[
            "source-ref-bound",
            "output-ref-bound",
            "marker-ref-bound",
            "diagnostic-only",
        ])?;
    }
    Ok(RedactionTransformReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        source_ref,
        output_ref,
        marker_refs,
        is_gate_preserving,
        diagnostics,
        value: value.clone(),
    })
}

pub fn commitment_replay_receipt_value(input: &CommitmentReplayInput) -> Result<IoValue> {
    validate_ref(&input.expected_commitment_ref, "expected commitment")?;
    validate_ref(&input.actual_commitment_ref, "actual commitment")?;
    validate_optional_ref(input.reveal_receipt_ref.as_deref(), "commitment replay reveal receipt")?;
    let mut diagnostics = Vec::new();
    if input.expected_commitment_ref != input.actual_commitment_ref {
        diagnostics.push_limited(
            "secret commitment mismatch during replay".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "commitment replay diagnostics",
        )?;
    }
    if input.is_plaintext_required && input.reveal_receipt_ref.is_none() {
        diagnostics.push_limited(
            "plaintext-required replay needs recorded effect response or reveal receipt".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "commitment replay diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("commitment-replay-receipt-v1", vec![
        string(SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("expected", vec![string(&input.expected_commitment_ref)]),
        record("actual", vec![string(&input.actual_commitment_ref)]),
        record("reveal-receipt", vec![optional_ref_value(input.reveal_receipt_ref.as_deref())]),
        record("plaintext-required", vec![bool_value(input.is_plaintext_required)]),
        diagnostics_value(&diagnostics),
        checks_value(&commitment_replay_checks(decision, input.is_plaintext_required)),
    ]))
}

pub fn parse_commitment_replay_receipt(value: &IoValue) -> Result<CommitmentReplayReceipt> {
    let fields = simple_record(value, "commitment-replay-receipt-v1", 8)?;
    require_schema(&fields[0], SECRET_COMMITMENT_REPLAY_RECEIPT_SCHEMA, "commitment replay")?;
    let decision = record_decision(&fields[1])?;
    let expected_commitment_ref = record_ref(&fields[2], "expected", "expected commitment")?;
    let actual_commitment_ref = record_ref(&fields[3], "actual", "actual commitment")?;
    let reveal_receipt_ref = record_optional_ref(&fields[4], "reveal-receipt", "commitment replay reveal")?;
    let is_plaintext_required = record_bool(&fields[5], "plaintext-required", "plaintext required")?;
    let diagnostics = parse_diagnostics(&fields[6])?;
    if decision == "pass" {
        require_checks(&fields[7], &["commitment-match", "plaintext-not-required", "replay-without-plaintext"])?;
    } else if is_plaintext_required {
        require_checks(&fields[7], &["commitment-comparison", "plaintext-required", "diagnostic-only"])?;
    } else {
        require_checks(&fields[7], &["commitment-mismatch", "fail-closed", "audit-receipt"])?;
    }
    Ok(CommitmentReplayReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        expected_commitment_ref,
        actual_commitment_ref,
        reveal_receipt_ref,
        diagnostics,
        value: value.clone(),
    })
}

pub fn secret_cleanup_receipt_value(input: &SecretCleanupInput) -> Result<IoValue> {
    validate_ref(&input.secret_ref, "cleanup secret ref")?;
    validate_ref(&input.revocation_ref, "cleanup revocation ref")?;
    validate_ref(&input.tombstone_ref, "cleanup tombstone ref")?;
    validate_refs(&input.retention_refs, "cleanup retention ref")?;
    validate_refs(&input.authority_refs, "cleanup authority ref")?;
    validate_refs(&input.policy_refs, "cleanup policy ref")?;
    ensure_count_at_most(input.retention_receipts.len(), MAX_SECRET_REFS, "cleanup retention receipts")?;
    let mut diagnostics = cleanup_retention_diagnostics(input)?;
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires authority evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires policy evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("secret-cleanup-receipt-v1", vec![
        string(SECRET_CLEANUP_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("secret", vec![string(&input.secret_ref)]),
        record("revocation", vec![string(&input.revocation_ref)]),
        record("tombstone", vec![string(&input.tombstone_ref)]),
        record("retention", vec![refs_sequence(&input.retention_refs)]),
        diagnostics_value(&diagnostics),
        checks_value(&secret_cleanup_checks(decision)),
    ]))
}

fn cleanup_retention_diagnostics(input: &SecretCleanupInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    let expected_refs = input.retention_refs.iter().cloned().collect::<BtreeSet<_>>();
    let mut actual_refs = BtreeSet::new();
    let mut has_matching_pass = false;
    let mut has_matching_tombstone = false;
    for receipt_value in &input.retention_receipts {
        match retention::parse_retention_receipt(receipt_value) {
            Ok(receipt) => {
                actual_refs.insert(receipt.receipt_ref.clone());
                let is_cleanup_action = matches!(
                    receipt.action.as_str(),
                    retention::ACTION_DELETE | retention::ACTION_TOMBSTONE | retention::ACTION_REDACT
                );
                if receipt.decision == "pass"
                    && receipt.object_ref == input.secret_ref
                    && receipt.retention_class == retention::CLASS_PRIVATE_SECRET_REF
                    && is_cleanup_action
                {
                    has_matching_pass = true;
                    if receipt.tombstone_ref.as_deref() == Some(input.tombstone_ref.as_str()) {
                        has_matching_tombstone = true;
                    }
                }
            }
            Err(_) => diagnostics.push_limited(
                "secret cleanup retention receipt invalid".to_string(),
                MAX_SECRET_DIAGNOSTICS,
                "secret cleanup diagnostics",
            )?,
        }
    }
    if input.retention_receipts.is_empty() {
        diagnostics.push_limited(
            "secret cleanup requires retention receipt evidence".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if expected_refs != actual_refs {
        diagnostics.push_limited(
            "secret cleanup retention receipt refs mismatch".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    if !has_matching_pass {
        diagnostics.push_limited(
            "secret cleanup requires passing private-secret retention receipt".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    } else if !has_matching_tombstone {
        diagnostics.push_limited(
            "secret cleanup retention tombstone mismatch".to_string(),
            MAX_SECRET_DIAGNOSTICS,
            "secret cleanup diagnostics",
        )?;
    }
    Ok(diagnostics)
}

pub fn parse_secret_cleanup_receipt(value: &IoValue) -> Result<SecretCleanupReceipt> {
    let fields = simple_record(value, "secret-cleanup-receipt-v1", 8)?;
    require_schema(&fields[0], SECRET_CLEANUP_RECEIPT_SCHEMA, "secret cleanup")?;
    let decision = record_decision(&fields[1])?;
    let secret_ref = record_ref(&fields[2], "secret", "cleanup secret")?;
    let revocation_ref = record_ref(&fields[3], "revocation", "cleanup revocation")?;
    let tombstone_ref = record_ref(&fields[4], "tombstone", "cleanup tombstone")?;
    let retention_refs = record_refs(&fields[5], "retention", "cleanup retention")?;
    let diagnostics = parse_diagnostics(&fields[6])?;
    if decision == "pass" {
        require_checks(&fields[7], &[
            "revocation-bound",
            "tombstone-bound",
            "retention-gc-bound",
            "idempotent-cleanup",
        ])?;
    } else {
        require_checks(&fields[7], &[
            "cleanup-denied",
            "no-plaintext-default",
            "audit-receipt",
            "retention-preserved",
        ])?;
    }
    Ok(SecretCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        secret_ref,
        revocation_ref,
        tombstone_ref,
        retention_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn private_bundle_profile_value(input: &PrivateBundleProfileInput) -> Result<IoValue> {
    validate_ref(&input.profile_ref, "private bundle profile ref")?;
    validate_refs(&input.encrypted_refs, "private bundle encrypted ref")?;
    validate_refs(&input.reveal_receipt_refs, "private bundle reveal receipt")?;
    validate_ref(&input.transform_receipt_ref, "private bundle transform receipt")?;
    let checks = if input.is_gate_preserving {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("gate-preserving-redaction", "pass"),
        ]
    } else {
        [
            ("encrypted-ref-validation", "pass"),
            ("reveal-receipts-bound", "pass"),
            ("redaction-transform-bound", "pass"),
            ("diagnostic-only", "pass"),
        ]
    };
    Ok(record("private-bundle-profile-v1", vec![
        string(PRIVATE_BUNDLE_PROFILE_SCHEMA),
        record("profile", vec![string(&input.profile_ref)]),
        record("encrypted-refs", vec![refs_sequence(&input.encrypted_refs)]),
        record("reveal-receipts", vec![refs_sequence(&input.reveal_receipt_refs)]),
        record("transform-receipt", vec![string(&input.transform_receipt_ref)]),
        record("gate-preserving", vec![bool_value(input.is_gate_preserving)]),
        checks_value(&checks),
    ]))
}

pub fn parse_private_bundle_profile(value: &IoValue) -> Result<PrivateBundleProfile> {
    let fields = simple_record(value, "private-bundle-profile-v1", 7)?;
    require_schema(&fields[0], PRIVATE_BUNDLE_PROFILE_SCHEMA, "private bundle profile")?;
    let profile_ref = record_ref(&fields[1], "profile", "private bundle profile ref")?;
    let encrypted_refs = record_refs(&fields[2], "encrypted-refs", "private bundle encrypted refs")?;
    let reveal_receipt_refs = record_refs(&fields[3], "reveal-receipts", "private bundle reveal receipts")?;
    let transform_receipt_ref = record_ref(&fields[4], "transform-receipt", "private bundle transform receipt")?;
    let is_gate_preserving = record_bool(&fields[5], "gate-preserving", "private bundle gate preserving")?;
    if is_gate_preserving {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "gate-preserving-redaction",
        ])?;
    } else {
        require_checks(&fields[6], &[
            "encrypted-ref-validation",
            "reveal-receipts-bound",
            "redaction-transform-bound",
            "diagnostic-only",
        ])?;
    }
    Ok(PrivateBundleProfile {
        profile_ref,
        encrypted_refs,
        reveal_receipt_refs,
        transform_receipt_ref,
        is_gate_preserving,
        value: value.clone(),
    })
}

pub fn contains_secret_marker(value: &IoValue) -> Result<bool> {
    let text = to_text(value)?;
    Ok(SENSITIVE_RECORD_LABELS.iter().any(|label| text.contains(&format!("<{label}"))))
}

pub fn redacted_value(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<IoValue> {
    Ok(redacted_view(value, redaction_profile_ref)?.value)
}

pub fn redacted_view(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<RedactedValue> {
    if !contains_secret_marker(value)? {
        return Ok(RedactedValue {
            value: value.clone(),
            marker: None,
            transform_receipt: None,
        });
    }
    let source_ref = canonical_hash(value)?;
    let policy_refs = vec![DEFAULT_REDACTION_POLICY.to_string()];
    let profile_ref = redaction_profile_ref.unwrap_or(DEFAULT_REDACTION_PROFILE).to_string();
    validate_ref(&profile_ref, "redaction profile ref")?;
    let reason = first_redaction_reason(value)?;
    let path_ref = fixture_ref(&format!("redaction-path:{source_ref}"));
    let marker_receipt_ref = redaction_seed_ref(&source_ref, &profile_ref, &policy_refs)?;
    let marker_value = redaction_marker_value(&RedactionMarkerInput {
        reason,
        commitment_ref: source_ref.clone(),
        schema_ref: fixture_ref("redacted-source-schema"),
        path_ref,
        policy_refs: policy_refs.clone(),
        receipt_ref: marker_receipt_ref,
    })?;
    let marker = parse_redaction_marker(&marker_value)?;
    let output_ref = canonical_hash(&marker.value)?;
    let transform_value = redaction_transform_receipt_value(&RedactionTransformInput {
        source_ref,
        output_ref,
        policy_refs,
        profile_ref,
        marker_refs: vec![marker.marker_ref.clone()],
        is_gate_preserving: true,
        diagnostics: Vec::new(),
    })?;
    let transform_receipt = parse_redaction_transform_receipt(&transform_value)?;
    Ok(RedactedValue {
        value: marker.value.clone(),
        marker: Some(marker),
        transform_receipt: Some(transform_receipt),
    })
}

pub fn redacted_text(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<String> {
    to_text(&redacted_value(value, redaction_profile_ref)?)
}

pub fn secrets_summary(value: &IoValue) -> Result<String> {
    let kind = crate::ledger::artifact_kind(value);
    if let Some(line) = summary_core(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_receipts(kind, value)? {
        return Ok(line);
    }
    if let Some(line) = summary_profiles(kind, value)? {
        return Ok(line);
    }
    Err(MoltenError::invalid_harness("not a secrets artifact"))
}

fn summary_core(kind: &str, value: &IoValue) -> Result<Option<String>> {
    match kind {
        "secret-ref" => {
            let secret = parse_secret_ref(value)?;
            Ok(Some(format!(
                "secret id={} scope={} commitment={} ref={} plaintext=redacted",
                secret.secret_id, secret.scope_ref, secret.commitment_ref, secret.secret_ref
            )))
        }
        "confidential-label" => {
            let label = parse_confidential_label(value)?;
            Ok(Some(format!(
                "confidential-label surface={} field={} classification={} ref={}",
                label.surface, label.field_path, label.classification, label.label_ref
            )))
        }
        "encrypted-ref" => {
            let encrypted = parse_encrypted_ref(value)?;
            Ok(Some(format!(
                "encrypted-ref ciphertext={} commitment={} ref={} authority=required",
                encrypted.ciphertext_ref, encrypted.commitment_ref, encrypted.encrypted_ref
            )))
        }
        "redaction-marker" => {
            let marker = parse_redaction_marker(value)?;
            Ok(Some(format!(
                "redaction-marker reason={} commitment={} ref={}",
                marker.reason, marker.commitment_ref, marker.marker_ref
            )))
        }
        _ => Ok(None),
    }
}

fn summary_receipts(kind: &str, value: &IoValue) -> Result<Option<String>> {
    match kind {
        "reveal-receipt" => {
            let receipt = parse_reveal_receipt(value)?;
            let encrypted_ref = receipt.encrypted_ref.as_deref().unwrap_or("none");
            Ok(Some(format!(
                "reveal-receipt decision={} purpose={} secret={} encrypted={} ref={}",
                receipt.decision, receipt.purpose, receipt.secret_ref, encrypted_ref, receipt.receipt_ref
            )))
        }
        "decrypt-receipt" => {
            let receipt = parse_decrypt_receipt(value)?;
            Ok(Some(format!(
                "decrypt-receipt decision={} purpose={} encrypted={} ref={}",
                receipt.decision, receipt.purpose, receipt.encrypted_ref, receipt.receipt_ref
            )))
        }
        "redaction-transform-receipt" => {
            let receipt = parse_redaction_transform_receipt(value)?;
            Ok(Some(format!(
                "redaction-transform decision={} source={} output={} ref={}",
                receipt.decision, receipt.source_ref, receipt.output_ref, receipt.receipt_ref
            )))
        }
        "secret-cleanup-receipt" => {
            let receipt = parse_secret_cleanup_receipt(value)?;
            Ok(Some(format!(
                "secret-cleanup decision={} secret={} tombstone={} ref={}",
                receipt.decision, receipt.secret_ref, receipt.tombstone_ref, receipt.receipt_ref
            )))
        }
        _ => Ok(None),
    }
}

fn summary_profiles(kind: &str, value: &IoValue) -> Result<Option<String>> {
    match kind {
        "private-bundle-profile" => {
            let profile = parse_private_bundle_profile(value)?;
            Ok(Some(format!(
                "private-bundle-profile profile={} encrypted={} gate-preserving={}",
                profile.profile_ref,
                profile.encrypted_refs.len(),
                profile.is_gate_preserving
            )))
        }
        "commitment-replay-receipt" => {
            let receipt = parse_commitment_replay_receipt(value)?;
            Ok(Some(format!(
                "commitment-replay decision={} expected={} actual={} ref={}",
                receipt.decision, receipt.expected_commitment_ref, receipt.actual_commitment_ref, receipt.receipt_ref
            )))
        }
        _ => Ok(None),
    }
}

pub fn fixture_field_labels() -> Result<Vec<ConfidentialLabel>> {
    let schema = fixture_ref("field-label-schema");
    let policy = vec![fixture_ref("field-label-policy")];
    let surfaces = [
        ("envelope", "/payload"),
        ("trace", "/events/*/payload"),
        ("receipt", "/diagnostics"),
        ("snapshot", "/state/secret"),
        ("storage", "/value"),
        ("transcript", "/stanzas/*/output"),
        ("catalog", "/payload"),
        ("report", "/observations/*/value"),
        ("bundle", "/artifacts/report"),
    ];
    let mut labels = Vec::new();
    for (surface, field_path) in surfaces {
        labels.push_limited(
            parse_confidential_label(&confidential_label_value(&ConfidentialLabelInput {
                surface: surface.to_string(),
                field_path: field_path.to_string(),
                classification: "secret".to_string(),
                schema_ref: schema.clone(),
                policy_refs: policy.clone(),
            })?)?,
            MAX_SECRET_MARKERS,
            "confidential labels",
        )?;
    }
    Ok(labels)
}

fn secrets_fixture_retention_root(secret_ref: &str) -> Result<PathBuf> {
    static RETENTION_ROOT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let process_id = std::process::id();
    let invocation_id = RETENTION_ROOT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root_ref = canonical_hash(&record("secrets-fixture-retention-root-v1", vec![
        string(secret_ref),
        string(process_id.to_string()),
        string(invocation_id.to_string()),
    ]))?;
    Ok(std::env::temp_dir().join(format!("molten-secrets-retention-{process_id}")).join(root_ref))
}

struct FixtureRefs {
    policy: Vec<String>,
    evidence: Vec<String>,
    authority: Vec<String>,
    resource: Vec<String>,
    effect: Vec<String>,
    commitment: String,
    encryption: String,
}

struct FixtureCore {
    labels: Vec<ConfidentialLabel>,
    refs: FixtureRefs,
    secret: SecretRef,
    encrypted: EncryptedRef,
    marker: RedactionMarker,
    transform: RedactionTransformReceipt,
}

struct FixtureReceipts {
    reveal_denied: RevealReceipt,
    reveal_pass: RevealReceipt,
    decrypt_denied: DecryptReceipt,
    decrypt_pass: DecryptReceipt,
    replay: CommitmentReplayReceipt,
}

struct FixtureTail {
    retention: retention::RetentionEvaluation,
    cleanup: SecretCleanupReceipt,
    private_bundle: PrivateBundleProfile,
}

fn fixture_refs() -> FixtureRefs {
    FixtureRefs {
        policy: vec![fixture_ref("secret-policy")],
        evidence: vec![fixture_ref("secret-evidence")],
        authority: vec![fixture_ref("secret-authority")],
        resource: vec![fixture_ref("secret-resource")],
        effect: vec![fixture_ref("secret-effect-handle")],
        commitment: fixture_ref("secret-commitment"),
        encryption: fixture_ref("secret-encryption-profile"),
    }
}

fn fixture_secret(labels: &[ConfidentialLabel], refs: &FixtureRefs) -> Result<SecretRef> {
    let primary_label =
        labels.first().ok_or_else(|| MoltenError::invalid_harness("secrets fixture missing field label"))?;
    let value = secret_ref_value(&SecretRefInput {
        secret_id: "secret:fixture".to_string(),
        scope_ref: fixture_ref("scope-service"),
        allowed_uses: vec![
            "debug".to_string(),
            "replay".to_string(),
            "export".to_string(),
            "adapter-use".to_string(),
        ],
        commitment_ref: refs.commitment.clone(),
        encryption_ref: refs.encryption.clone(),
        redaction_label_ref: primary_label.label_ref.clone(),
        expiry_ref: None,
        revocation_refs: Vec::new(),
        evidence_refs: refs.evidence.clone(),
    })?;
    parse_secret_ref(&value)
}

fn fixture_encrypted(refs: &FixtureRefs) -> Result<EncryptedRef> {
    let value = encrypted_ref_value(&EncryptedRefInput {
        ciphertext_ref: fixture_ref("ciphertext"),
        commitment_ref: refs.commitment.clone(),
        encryption_ref: refs.encryption.clone(),
        schema_ref: fixture_ref("secret-schema"),
        policy_refs: refs.policy.clone(),
        evidence_refs: refs.evidence.clone(),
    })?;
    parse_encrypted_ref(&value)
}

fn fixture_redaction() -> Result<(RedactionMarker, RedactionTransformReceipt)> {
    let sensitive_value = record("credential", vec![string("do-not-render")]);
    let redacted = redacted_view(&sensitive_value, None)?;
    let marker = redacted
        .marker
        .ok_or_else(|| MoltenError::invalid_harness("secrets fixture expected redaction marker"))?;
    let transform = redacted
        .transform_receipt
        .ok_or_else(|| MoltenError::invalid_harness("secrets fixture expected transform receipt"))?;
    Ok((marker, transform))
}

fn fixture_core() -> Result<FixtureCore> {
    let labels = fixture_field_labels()?;
    let refs = fixture_refs();
    let secret = fixture_secret(&labels, &refs)?;
    let encrypted = fixture_encrypted(&refs)?;
    let (marker, transform) = fixture_redaction()?;
    Ok(FixtureCore {
        labels,
        refs,
        secret,
        encrypted,
        marker,
        transform,
    })
}

fn fixture_reveals(core: &FixtureCore) -> Result<(RevealReceipt, RevealReceipt)> {
    let denied_value = reveal_receipt_value(&RevealReceiptInput {
        secret_ref: core.secret.secret_ref.clone(),
        encrypted_ref: Some(core.encrypted.encrypted_ref.clone()),
        requester_ref: fixture_ref("requester"),
        purpose: "debug".to_string(),
        plaintext_ref: Some(fixture_ref("plaintext")),
        commitment_ref: core.refs.commitment.clone(),
        authority_refs: Vec::new(),
        policy_refs: core.refs.policy.clone(),
        resource_refs: core.refs.resource.clone(),
        effect_handle_refs: core.refs.effect.clone(),
        revocation_refs: Vec::new(),
    })?;
    let pass_value = reveal_receipt_value(&RevealReceiptInput {
        secret_ref: core.secret.secret_ref.clone(),
        encrypted_ref: Some(core.encrypted.encrypted_ref.clone()),
        requester_ref: fixture_ref("requester"),
        purpose: "debug".to_string(),
        plaintext_ref: Some(fixture_ref("plaintext")),
        commitment_ref: core.refs.commitment.clone(),
        authority_refs: core.refs.authority.clone(),
        policy_refs: core.refs.policy.clone(),
        resource_refs: core.refs.resource.clone(),
        effect_handle_refs: core.refs.effect.clone(),
        revocation_refs: Vec::new(),
    })?;
    Ok((parse_reveal_receipt(&denied_value)?, parse_reveal_receipt(&pass_value)?))
}

fn fixture_decrypts(core: &FixtureCore, reveal_pass: &RevealReceipt) -> Result<(DecryptReceipt, DecryptReceipt)> {
    let denied_value = decrypt_receipt_value(&DecryptReceiptInput {
        encrypted_ref: core.encrypted.encrypted_ref.clone(),
        requester_ref: fixture_ref("requester"),
        purpose: "adapter-use".to_string(),
        plaintext_ref: Some(fixture_ref("plaintext")),
        commitment_ref: core.refs.commitment.clone(),
        expected_commitment_ref: core.refs.commitment.clone(),
        reveal_receipt_ref: None,
        has_reveal_authority: false,
        authority_refs: core.refs.authority.clone(),
        policy_refs: core.refs.policy.clone(),
        resource_refs: core.refs.resource.clone(),
        effect_handle_refs: core.refs.effect.clone(),
    })?;
    let pass_value = decrypt_receipt_value(&DecryptReceiptInput {
        encrypted_ref: core.encrypted.encrypted_ref.clone(),
        requester_ref: fixture_ref("requester"),
        purpose: "adapter-use".to_string(),
        plaintext_ref: reveal_pass.plaintext_ref.clone(),
        commitment_ref: core.refs.commitment.clone(),
        expected_commitment_ref: core.encrypted.commitment_ref.clone(),
        reveal_receipt_ref: Some(reveal_pass.receipt_ref.clone()),
        has_reveal_authority: reveal_pass.decision == "pass",
        authority_refs: core.refs.authority.clone(),
        policy_refs: core.refs.policy.clone(),
        resource_refs: core.refs.resource.clone(),
        effect_handle_refs: core.refs.effect.clone(),
    })?;
    Ok((parse_decrypt_receipt(&denied_value)?, parse_decrypt_receipt(&pass_value)?))
}

fn fixture_replay(core: &FixtureCore) -> Result<CommitmentReplayReceipt> {
    let value = commitment_replay_receipt_value(&CommitmentReplayInput {
        expected_commitment_ref: core.refs.commitment.clone(),
        actual_commitment_ref: core.encrypted.commitment_ref.clone(),
        reveal_receipt_ref: None,
        is_plaintext_required: false,
    })?;
    parse_commitment_replay_receipt(&value)
}

fn fixture_receipts(core: &FixtureCore) -> Result<FixtureReceipts> {
    let (reveal_denied, reveal_pass) = fixture_reveals(core)?;
    let (decrypt_denied, decrypt_pass) = fixture_decrypts(core, &reveal_pass)?;
    let replay = fixture_replay(core)?;
    Ok(FixtureReceipts {
        reveal_denied,
        reveal_pass,
        decrypt_denied,
        decrypt_pass,
        replay,
    })
}

fn fixture_retention(core: &FixtureCore) -> Result<retention::RetentionEvaluation> {
    let retention_root = secrets_fixture_retention_root(&core.secret.secret_ref)?;
    retention::evaluate_retention(retention::RetentionEvaluationInput {
        root: &retention_root,
        object_ref: &core.secret.secret_ref,
        object_kind: "secret-ref",
        retention_class: retention::CLASS_PRIVATE_SECRET_REF,
        action: retention::ACTION_REDACT,
        requester_ref: &fixture_ref("requester"),
        is_reference_index_complete: true,
        retained_refs: &[],
        remote_refs: &[],
        policy_refs: &core.refs.policy,
        evidence_refs: &core.refs.evidence,
        has_delete_authority: true,
        has_remote_gc_clearance: true,
    })
}

fn fixture_cleanup(core: &FixtureCore, retention: &retention::RetentionEvaluation) -> Result<SecretCleanupReceipt> {
    let tombstone_ref = retention
        .receipt
        .tombstone_ref
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("secrets cleanup retention receipt missing tombstone"))?;
    let value = secret_cleanup_receipt_value(&SecretCleanupInput {
        secret_ref: core.secret.secret_ref.clone(),
        revocation_ref: fixture_ref("secret-revocation"),
        tombstone_ref,
        retention_refs: vec![retention.receipt.receipt_ref.clone()],
        retention_receipts: vec![retention.receipt.value.clone()],
        authority_refs: core.refs.authority.clone(),
        policy_refs: core.refs.policy.clone(),
    })?;
    parse_secret_cleanup_receipt(&value)
}

fn fixture_private_bundle(core: &FixtureCore, receipts: &FixtureReceipts) -> Result<PrivateBundleProfile> {
    let value = private_bundle_profile_value(&PrivateBundleProfileInput {
        profile_ref: fixture_ref("private-bundle-profile"),
        encrypted_refs: vec![core.encrypted.encrypted_ref.clone()],
        reveal_receipt_refs: vec![receipts.reveal_pass.receipt_ref.clone()],
        transform_receipt_ref: core.transform.receipt_ref.clone(),
        is_gate_preserving: true,
    })?;
    parse_private_bundle_profile(&value)
}

fn fixture_tail(core: &FixtureCore, receipts: &FixtureReceipts) -> Result<FixtureTail> {
    let retention = fixture_retention(core)?;
    let cleanup = fixture_cleanup(core, &retention)?;
    let private_bundle = fixture_private_bundle(core, receipts)?;
    Ok(FixtureTail {
        retention,
        cleanup,
        private_bundle,
    })
}

fn fixture_evidence_values(core: &FixtureCore, receipts: &FixtureReceipts, tail: &FixtureTail) -> Result<Vec<IoValue>> {
    let mut values = Vec::new();
    for label in &core.labels {
        values.push_limited(label.value.clone(), MAX_SECRET_MARKERS, "secrets fixture evidence")?;
    }
    for value in [
        core.secret.value.clone(),
        core.encrypted.value.clone(),
        core.marker.value.clone(),
        core.transform.value.clone(),
        receipts.reveal_denied.value.clone(),
        receipts.reveal_pass.value.clone(),
        receipts.decrypt_denied.value.clone(),
        receipts.decrypt_pass.value.clone(),
        receipts.replay.value.clone(),
        tail.retention.receipt.value.clone(),
        tail.cleanup.value.clone(),
        tail.private_bundle.value.clone(),
    ] {
        values.push_limited(value, MAX_SECRET_MARKERS, "secrets fixture evidence")?;
    }
    if let Some(tombstone) = tail.retention.tombstone.as_ref() {
        values.push_limited(tombstone.value.clone(), MAX_SECRET_MARKERS, "secrets fixture evidence")?;
    }
    Ok(values)
}

fn fixture_report(core: &FixtureCore, receipts: &FixtureReceipts, tail: &FixtureTail) -> Result<(IoValue, String)> {
    let value = record("secrets-fixture-report-v1", vec![
        string("molten.secrets.fixture-report.v1"),
        record("decision", vec![string("pass")]),
        record("secret", vec![string(&core.secret.secret_ref)]),
        record("encrypted", vec![string(&core.encrypted.encrypted_ref)]),
        record("redaction", vec![string(&core.marker.marker_ref)]),
        record("reveal", vec![string(&receipts.reveal_pass.receipt_ref)]),
        record("decrypt", vec![string(&receipts.decrypt_pass.receipt_ref)]),
        record("replay", vec![string(&receipts.replay.receipt_ref)]),
        record("cleanup", vec![string(&tail.cleanup.receipt_ref)]),
        record("private-bundle", vec![string(&tail.private_bundle.profile_ref)]),
        checks_value(&[
            ("no-plaintext-default", "pass"),
            ("encrypted-ref-not-authority", "pass"),
            ("commitment-replay", "pass"),
            ("gate-preserving-redaction", "pass"),
        ]),
    ]);
    let value_ref = canonical_hash(&value)?;
    Ok((value, value_ref))
}

pub fn run_secrets_fixture() -> Result<SecretsFixtureRun> {
    let core = fixture_core()?;
    let receipts = fixture_receipts(&core)?;
    let tail = fixture_tail(&core, &receipts)?;
    let (report_value, report_ref) = fixture_report(&core, &receipts, &tail)?;
    let mut evidence_values = fixture_evidence_values(&core, &receipts, &tail)?;
    evidence_values.push_limited(report_value.clone(), MAX_SECRET_MARKERS, "secrets fixture evidence")?;
    Ok(SecretsFixtureRun {
        value: report_value,
        report_ref,
        secret: core.secret,
        encrypted: core.encrypted,
        marker: core.marker,
        transform: core.transform,
        reveal_denied: receipts.reveal_denied,
        reveal_pass: receipts.reveal_pass,
        decrypt_denied: receipts.decrypt_denied,
        decrypt_pass: receipts.decrypt_pass,
        replay: receipts.replay,
        cleanup: tail.cleanup,
        private_bundle: tail.private_bundle,
        evidence_values,
    })
}

pub fn fixture_report_summary(value: &IoValue) -> Result<String> {
    let fields = simple_record(value, "secrets-fixture-report-v1", 11)?;
    require_schema(&fields[0], "molten.secrets.fixture-report.v1", "secrets fixture report")?;
    let decision = record_string(&fields[1], "decision", "secrets fixture decision")?;
    let secret_ref = record_ref(&fields[2], "secret", "secrets fixture secret")?;
    let encrypted_ref = record_ref(&fields[3], "encrypted", "secrets fixture encrypted")?;
    Ok(format!(
        "secrets fixture decision={decision} secret={secret_ref} encrypted={encrypted_ref} plaintext=redacted ref={}",
        canonical_hash(value)?
    ))
}

struct AccessGateInput<'a> {
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    effect_handle_refs: &'a [String],
    revocation_refs: &'a [String],
    operation: &'a str,
}

fn collect_gate_diagnostics(input: AccessGateInput<'_>, diagnostics: &mut impl PushLimited<String>) -> Result<()> {
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            format!("{} requires authority evidence", input.operation),
            MAX_SECRET_DIAGNOSTICS,
            "secrets diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() {
        diagnostics.push_limited(
            format!("{} requires policy evidence", input.operation),
            MAX_SECRET_DIAGNOSTICS,
            "secrets diagnostics",
        )?;
    }
    if input.resource_refs.is_empty() {
        diagnostics.push_limited(
            format!("{} requires resource evidence", input.operation),
            MAX_SECRET_DIAGNOSTICS,
            "secrets diagnostics",
        )?;
    }
    if input.effect_handle_refs.is_empty() {
        diagnostics.push_limited(
            format!("{} requires effect-handle evidence", input.operation),
            MAX_SECRET_DIAGNOSTICS,
            "secrets diagnostics",
        )?;
    }
    if !input.revocation_refs.is_empty() {
        diagnostics.push_limited(
            format!("{} denied because secret has revocation refs", input.operation),
            MAX_SECRET_DIAGNOSTICS,
            "secrets diagnostics",
        )?;
    }
    Ok(())
}

fn reveal_checks(decision: &str, has_encrypted_ref: bool) -> Vec<(&'static str, &'static str)> {
    let mut checks = if decision == "pass" {
        vec![
            ("authorized-reveal", "pass"),
            ("policy-bound", "pass"),
            ("resource-bound", "pass"),
            ("effect-handle-bound", "pass"),
        ]
    } else {
        vec![
            ("deny-without-authority", "pass"),
            ("no-plaintext-on-deny", "pass"),
            ("ciphertext-not-authority", "pass"),
            ("audit-receipt", "pass"),
        ]
    };
    if has_encrypted_ref {
        checks.push(("encrypted-ref-bound", "pass"));
    }
    checks
}

fn decrypt_checks(decision: &str) -> [(&'static str, &'static str); 4] {
    if decision == "pass" {
        [
            ("authorized-decrypt", "pass"),
            ("reveal-receipt-bound", "pass"),
            ("commitment-match", "pass"),
            ("effect-handle-bound", "pass"),
        ]
    } else {
        [
            ("deny-without-reveal", "pass"),
            ("no-plaintext-on-deny", "pass"),
            ("ciphertext-not-authority", "pass"),
            ("audit-receipt", "pass"),
        ]
    }
}

fn redaction_transform_checks(decision: &str, is_gate_preserving: bool) -> [(&'static str, &'static str); 4] {
    if decision == "pass" && is_gate_preserving {
        [
            ("source-ref-bound", "pass"),
            ("output-ref-bound", "pass"),
            ("marker-ref-bound", "pass"),
            ("semantic-evidence-preserved", "pass"),
        ]
    } else {
        [
            ("source-ref-bound", "pass"),
            ("output-ref-bound", "pass"),
            ("marker-ref-bound", "pass"),
            ("diagnostic-only", "pass"),
        ]
    }
}

fn commitment_replay_checks(decision: &str, is_plaintext_required: bool) -> [(&'static str, &'static str); 3] {
    if decision == "pass" {
        [
            ("commitment-match", "pass"),
            ("plaintext-not-required", "pass"),
            ("replay-without-plaintext", "pass"),
        ]
    } else if is_plaintext_required {
        [
            ("commitment-comparison", "pass"),
            ("plaintext-required", "pass"),
            ("diagnostic-only", "pass"),
        ]
    } else {
        [
            ("commitment-mismatch", "pass"),
            ("fail-closed", "pass"),
            ("audit-receipt", "pass"),
        ]
    }
}

fn secret_cleanup_checks(decision: &str) -> [(&'static str, &'static str); 4] {
    if decision == "pass" {
        [
            ("revocation-bound", "pass"),
            ("tombstone-bound", "pass"),
            ("retention-gc-bound", "pass"),
            ("idempotent-cleanup", "pass"),
        ]
    } else {
        [
            ("cleanup-denied", "pass"),
            ("no-plaintext-default", "pass"),
            ("audit-receipt", "pass"),
            ("retention-preserved", "pass"),
        ]
    }
}

fn first_redaction_reason(value: &IoValue) -> Result<String> {
    let text = to_text(value)?;
    if text.contains("<credential") {
        Ok("credential".to_string())
    } else if text.contains("<private") {
        Ok("private".to_string())
    } else if text.contains("<encrypted-ref") {
        Ok("encrypted-ref".to_string())
    } else {
        Ok("secret".to_string())
    }
}

fn redaction_seed_ref(source_ref: &str, profile_ref: &str, policy_refs: &[String]) -> Result<String> {
    canonical_hash(&record("redaction-receipt-seed-v1", vec![
        record("source", vec![string(source_ref)]),
        record("profile", vec![string(profile_ref)]),
        record("policy", vec![refs_sequence(policy_refs)]),
    ]))
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn diagnostics_value(diagnostics: &[String]) -> IoValue {
    record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())])
}

fn parse_diagnostics(value: &Value<IoValue>) -> Result<Vec<String>> {
    record_strings(value, "diagnostics", "diagnostics")
}

fn record_decision(value: &Value<IoValue>) -> Result<String> {
    let decision = record_string(value, "decision", "decision")?;
    if decision == "pass" || decision == "deny" {
        Ok(decision)
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported secrets decision {decision}")))
    }
}

fn record_string(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<String> {
    let value = record_string(value, record_name, label)?;
    validate_ref(&value, label)?;
    Ok(value)
}

fn record_optional_ref(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    parse_optional_ref(&record[0], label)
}

fn record_bool(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<bool> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    record[0]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected bool for {label}")))
}

fn record_strings(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, record_name, 1)?;
    let values = required_sequence(&record[0], label)?;
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn record_refs(value: &Value<IoValue>, record_name: &str, label: &str) -> Result<Vec<String>> {
    let refs = record_strings(value, record_name, label)?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn parse_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        let item = required_string(&some[0], label)?;
        validate_ref(&item, label)?;
        return Ok(Some(item));
    }
    let item = required_string(value, label)?;
    validate_ref(&item, label)?;
    Ok(Some(item))
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {label} schema {actual}; expected {expected}")))
    }
}

fn require_checks(value: &Value<IoValue>, expected: &[&str]) -> Result<()> {
    let value = value_to_iovalue(value);
    let check_record = simple_record(&value, "checks", 1)?;
    let values = required_sequence(&check_record[0], "checks")?;
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, "checks")?;
    let mut seen = BtreeSet::new();
    for value in values.iter() {
        let item = value_to_iovalue(value);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("unsupported check status {status}")));
        }
        seen.insert(name);
    }
    for expected in expected {
        if !seen.contains(*expected) {
            return Err(MoltenError::invalid_harness(format!("missing secrets check {expected}")));
        }
    }
    Ok(())
}

fn simple_record<'a>(value: &'a IoValue, label: &str, arity: usize) -> Result<Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn required_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {label}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, label: &str) -> Result<Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))
}

fn validate_classification(value: &str) -> Result<()> {
    match value {
        "secret" | "credential" | "private" | "policy" | "encrypted-ref" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported confidential classification {value}"))),
    }
}

fn validate_redaction_reason(value: &str) -> Result<()> {
    match value {
        "secret" | "credential" | "private" | "policy" | "encrypted-ref" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported redaction reason {value}"))),
    }
}

fn validate_purpose(value: &str) -> Result<()> {
    match value {
        "debug" | "replay" | "export" | "adapter-use" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported secret purpose {value}"))),
    }
}

fn validate_allowed_uses(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_USES, "secret allowed uses")?;
    for value in values {
        validate_purpose(value)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, label: &str) -> Result<()> {
    validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{label} must be a canonical content ref: {error}")))
}

fn validate_optional_ref(value: Option<&str>, label: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, label)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_REFS, label)?;
    for value in values {
        validate_ref(value, label)?;
    }
    Ok(())
}

fn validate_diagnostics(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_SECRET_DIAGNOSTICS, label)
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    }
}

fn fixture_ref(label: &str) -> String {
    content_ref_from_bytes(label.as_bytes())
}

trait PushLimited<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: crate::bounded::VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.item_count().saturating_add(1), maximum, label)?;
        self.push_item(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove stale temp root");
        }
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn canonical_confidentiality_records_roundtrip() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.reveal_denied.decision, "deny");
        assert_eq!(run.reveal_pass.decision, "pass");
        assert_eq!(run.decrypt_denied.decision, "deny");
        assert_eq!(run.decrypt_pass.decision, "pass");
        assert_eq!(run.replay.decision, "pass");
        assert_eq!(run.cleanup.decision, "pass");
        assert_eq!(run.cleanup.retention_refs.len(), 1);
        assert!(run.reveal_denied.plaintext_ref.is_none());
        assert!(fixture_report_summary(&run.value).expect("fixture summary").contains("plaintext=redacted"));
        assert_eq!(parse_secret_ref(&run.secret.value).expect("secret").secret_ref, run.secret.secret_ref);
        assert_eq!(
            parse_encrypted_ref(&run.encrypted.value).expect("encrypted").encrypted_ref,
            run.encrypted.encrypted_ref
        );
        assert_eq!(parse_redaction_marker(&run.marker.value).expect("marker").marker_ref, run.marker.marker_ref);
    }

    #[test]
    fn secret_cleanup_requires_actual_retention_receipt_evidence() {
        let denied_value = secret_cleanup_receipt_value(&SecretCleanupInput {
            secret_ref: fixture_ref("secret"),
            revocation_ref: fixture_ref("revocation"),
            tombstone_ref: fixture_ref("tombstone"),
            retention_refs: Vec::new(),
            retention_receipts: Vec::new(),
            authority_refs: vec![fixture_ref("authority")],
            policy_refs: vec![fixture_ref("policy")],
        })
        .expect("cleanup receipt");
        let denied = parse_secret_cleanup_receipt(&denied_value).expect("parse cleanup deny");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("retention receipt")));
    }

    #[test]
    fn catalog_and_mcp_render_redaction_markers_without_plaintext() {
        let root = temp_dir("secrets-catalog");
        let registry = root.join("registry");
        let artifact = crate::artifacts::install_artifact(&registry, &crate::artifacts::ArtifactInstallInput {
            kind: "doc".to_string(),
            payload: parse_text("<doc <credential \"do-not-render\">>").expect("secret doc"),
            schema_refs: Vec::new(),
            dependency_refs: Vec::new(),
            effect_manifest_ref: None,
            policy_refs: Vec::new(),
            evidence_refs: Vec::new(),
            installer_ref: fixture_ref("catalog-installer"),
            capability_refs: vec![fixture_ref("catalog-capability")],
        })
        .expect("install");
        let viewed = crate::catalog::view(&registry, None, &crate::catalog::CatalogViewInput {
            reference: artifact.artifact_ref.clone(),
            include_payload: true,
            redacted: true,
            visibility: crate::catalog::CatalogVisibilityInput::default(),
        })
        .expect("view");
        let text = to_text(&viewed.value).expect("view text");
        assert!(text.contains("redaction-marker-v1"));
        assert!(!text.contains("do-not-render"));
        let request = crate::catalog_mcp::mcp_request_value("catalog.view", vec![
            record("reference", vec![string(&artifact.artifact_ref)]),
            record("payload", vec![bool_value(true)]),
        ])
        .expect("mcp request");
        let response = crate::catalog_mcp::call(&registry, None, &request).expect("mcp call");
        let response_text = to_text(&response.response_value).expect("response");
        assert!(response_text.contains("redaction-marker-v1"));
        assert!(!response_text.contains("do-not-render"));
    }

    #[test]
    fn reveal_and_decrypt_require_authority_not_ciphertext_possession() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.reveal_denied.decision, "deny");
        assert!(run.reveal_denied.diagnostics.join(";").contains("authority"));
        assert_eq!(run.decrypt_denied.decision, "deny");
        assert!(run.decrypt_denied.diagnostics.join(";").contains("encrypted refs alone are not authority"));
        assert_eq!(run.decrypt_pass.decision, "pass");
        assert_eq!(run.decrypt_pass.commitment_ref, run.encrypted.commitment_ref);
    }

    #[test]
    fn commitment_replay_and_private_bundle_profiles_are_receipted() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(run.private_bundle.transform_receipt_ref, run.transform.receipt_ref);
        assert!(run.private_bundle.is_gate_preserving);
        let plaintext_required = commitment_replay_receipt_value(&CommitmentReplayInput {
            expected_commitment_ref: run.secret.commitment_ref.clone(),
            actual_commitment_ref: run.encrypted.commitment_ref.clone(),
            reveal_receipt_ref: None,
            is_plaintext_required: true,
        })
        .expect("plaintext required replay");
        let denied = parse_commitment_replay_receipt(&plaintext_required).expect("parse denied replay");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.join(";").contains("plaintext-required"));
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_confidentiality_artifacts() {
        let run = run_secrets_fixture().expect("fixture");
        assert_eq!(crate::ledger::artifact_kind(&run.secret.value), "secret-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.encrypted.value), "encrypted-ref");
        assert_eq!(crate::ledger::artifact_kind(&run.marker.value), "redaction-marker");
        assert_eq!(crate::ledger::artifact_kind(&run.transform.value), "redaction-transform-receipt");
        let root = temp_dir("secrets-ledger");
        let registry = root.join("registry");
        let ledger_root = root.join("ledger");
        std::fs::create_dir_all(&registry).expect("registry");
        crate::ledger::import_artifact(&ledger_root, &run.secret.value).expect("import");
        let list = crate::catalog::list(&registry, Some(&ledger_root), &crate::catalog::CatalogListInput {
            kind: Some("secret-ref".to_string()),
            visibility: crate::catalog::CatalogVisibilityInput::default(),
        })
        .expect("list");
        assert_eq!(list.items.len(), 1);
        let request = crate::catalog_mcp::mcp_request_value("catalog.view", vec![record("reference", vec![string(
            &run.secret.secret_ref,
        )])])
        .expect("mcp request");
        let response = crate::catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("mcp call");
        assert_eq!(response.decision, "pass");
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_redaction_stability_no_plaintext_and_authority_monotonicity(tc: TestCase) {
        let salt = tc.draw(generators::integers::<u64>().min_value(1).max_value(1_000_000));
        let payload = record("secret", vec![string(format!("payload-{salt}"))]);
        let first = redacted_view(&payload, None).expect("first redaction");
        let second = redacted_view(&payload, None).expect("second redaction");
        assert_eq!(first.value, second.value);
        let redacted = to_text(&first.value).expect("redacted text");
        assert!(!redacted.contains(&format!("payload-{salt}")));
        let secret_ref = fixture_ref(&format!("secret-{salt}"));
        let commitment_ref = fixture_ref(&format!("commitment-{salt}"));
        let denied = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref: secret_ref.clone(),
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref: commitment_ref.clone(),
                authority_refs: Vec::new(),
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("deny reveal value"),
        )
        .expect("deny reveal");
        let admitted = parse_reveal_receipt(
            &reveal_receipt_value(&RevealReceiptInput {
                secret_ref,
                encrypted_ref: None,
                requester_ref: fixture_ref("requester"),
                purpose: "debug".to_string(),
                plaintext_ref: Some(fixture_ref("plain")),
                commitment_ref,
                authority_refs: vec![fixture_ref("authority")],
                policy_refs: vec![fixture_ref("policy")],
                resource_refs: vec![fixture_ref("resource")],
                effect_handle_refs: vec![fixture_ref("effect")],
                revocation_refs: Vec::new(),
            })
            .expect("admit reveal value"),
        )
        .expect("admit reveal");
        assert_eq!(denied.decision, "deny");
        assert_eq!(admitted.decision, "pass");
    }
}
