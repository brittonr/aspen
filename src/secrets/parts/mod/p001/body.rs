
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
