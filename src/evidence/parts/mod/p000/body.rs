type IoValue = preserves::IOValue;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA: &str =
    crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA;
const EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA: &str = crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA;
const EVIDENCE_SIGNED_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value.as_ref())
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const SIGNATURE_ALGORITHM: &str = "blake3-local-fixture-v1";
pub const PASS_EVIDENCE_PURPOSE: &str = "pass-evidence";

const MAX_SIGNED_RECEIPT_PARENTS: usize = 256;
const MAX_SIGNED_KEY_RECORDS: usize = 4096;
pub const SIGNED_RECEIPT_KEY_STATUS_CURRENT: &str = "current";
const _: () = assert!(MAX_SIGNED_RECEIPT_PARENTS > 0);
const _: () = assert!(MAX_SIGNED_KEY_RECORDS > 0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedReceipt {
    pub envelope_ref: String,
    pub subject_ref: String,
    pub signer: String,
    pub purpose: String,
    pub trust_root: String,
    pub algorithm: String,
    pub parents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedReceiptKey {
    pub key_ref: String,
    pub key_id: String,
    pub signer: String,
    pub trust_root: String,
    pub key: String,
    pub status: String,
    pub generation: u64,
    pub predecessor_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedReceiptKeyRevocation {
    pub revocation_ref: String,
    pub key_ref: String,
    pub key_id: String,
    pub signer: String,
    pub trust_root: String,
    pub reason: String,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedReceiptWithKey {
    pub receipt: SignedReceipt,
    pub key_ref: String,
    pub key_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SignedReceiptEnvelope {
    subject_ref: String,
    signer: String,
    purpose: String,
    trust_root: String,
}

struct SubjectParts {
    subject_ref: String,
    receipt_value: IoValue,
}

struct SignerParts {
    signer: String,
    purpose: String,
    trust_root: String,
}

pub struct SignReceiptInput<'a> {
    pub receipt: &'a IoValue,
    pub signer: &'a str,
    pub purpose: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub parents: &'a [String],
}

pub struct VerifySignedReceiptPolicy<'a> {
    pub required_purpose: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub expected_signer: Option<&'a str>,
    pub expected_subject_ref: Option<&'a str>,
}

pub struct SignedReceiptKeyInput<'a> {
    pub key_id: &'a str,
    pub signer: &'a str,
    pub trust_root: &'a str,
    pub key: &'a str,
    pub generation: u64,
    pub predecessor_ref: Option<&'a str>,
}

pub struct SignedReceiptKeyRevocationInput<'a> {
    pub key: &'a SignedReceiptKey,
    pub reason: &'a str,
    pub superseded_by: Option<&'a str>,
}

pub struct VerifySignedReceiptKeyringPolicy<'a> {
    pub required_purpose: &'a str,
    pub trust_root: &'a str,
    pub expected_signer: Option<&'a str>,
    pub expected_subject_ref: Option<&'a str>,
    pub required_key_ref: Option<&'a str>,
    pub required_key_id: Option<&'a str>,
    pub keys: &'a [SignedReceiptKey],
    pub revocations: &'a [SignedReceiptKeyRevocation],
}

pub fn sign_receipt(input: &SignReceiptInput<'_>) -> Result<IoValue> {
    if input.signer.trim().is_empty() {
        return Err(MoltenError::invalid_harness("signer id must not be empty"));
    }
    if input.trust_root.trim().is_empty() {
        return Err(MoltenError::invalid_harness("signed receipt trust root must not be empty"));
    }
    let subject_ref = canonical_hash(input.receipt)?;
    let signature = signature_for(input.receipt, input.signer, input.purpose, input.trust_root, input.key)?;
    Ok(record("signed-receipt-v1", vec![
        string(EVIDENCE_SIGNED_RECEIPT_SCHEMA),
        record("subject", vec![string(&subject_ref), input.receipt.clone()]),
        record("signer", vec![string(input.signer), string(input.purpose), string(input.trust_root)]),
        record("algorithm", vec![string(SIGNATURE_ALGORITHM)]),
        record("signature", vec![string(&signature)]),
        record("parents", vec![sequence(input.parents.iter().map(string).collect())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("subject-ref-binding"), string("pass")]),
            record("check", vec![string("signature-covers-canonical-receipt"), string("pass")]),
            record("check", vec![string("parent-receipt-refs"), string("pass")]),
            record("check", vec![string("signed-receipt-is-evidence-only"), string("pass")]),
        ])]),
    ]))
}

pub fn signed_receipt_key_value(input: &SignedReceiptKeyInput<'_>) -> Result<IoValue> {
    require_non_empty(input.key_id, "signed receipt key id")?;
    require_non_empty(input.signer, "signed receipt key signer")?;
    require_non_empty(input.trust_root, "signed receipt key trust root")?;
    require_non_empty(input.key, "signed receipt verification key")?;
    Ok(record("signed-receipt-key-v1", vec![
        string(EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA),
        record("identity", vec![string(input.key_id), string(input.signer), string(input.trust_root)]),
        record("verification-key", vec![string(SIGNATURE_ALGORITHM), string(input.key)]),
        record("status", vec![string(SIGNED_RECEIPT_KEY_STATUS_CURRENT), u64_value(input.generation)]),
        record("predecessor", vec![optional_ref_value(input.predecessor_ref)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("key-id-bound"), string("pass")]),
            record("check", vec![string("key-material-bound"), string("pass")]),
            record("check", vec![string("key-record-is-evidence-only"), string("pass")]),
        ])]),
    ]))
}

pub fn signed_receipt_key_revocation_value(input: &SignedReceiptKeyRevocationInput<'_>) -> Result<IoValue> {
    require_non_empty(input.reason, "signed receipt key revocation reason")?;
    Ok(record("signed-receipt-key-revocation-v1", vec![
        string(EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA),
        record("key", vec![
            string(&input.key.key_ref),
            string(&input.key.key_id),
            string(&input.key.signer),
            string(&input.key.trust_root),
        ]),
        record("reason", vec![string(input.reason)]),
        record("superseded-by", vec![optional_ref_value(input.superseded_by)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("key-ref-bound"), string("pass")]),
            record("check", vec![string("key-revocation-currentness-bound"), string("pass")]),
            record("check", vec![string("key-revocation-is-evidence-only"), string("pass")]),
        ])]),
    ]))
}

pub fn parse_signed_receipt_key(value: &IoValue) -> Result<SignedReceiptKey> {
    let fields = value
        .collect_simple_record("signed-receipt-key-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <signed-receipt-key-v1 ...>"))?;
    let schema = required_string(&fields[0], "signed receipt key schema")?;
    if schema != EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt key schema {schema}; expected {EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA}"
        )));
    }
    let identity_value = value_to_iovalue(&fields[1]);
    let identity = identity_value
        .collect_simple_record("identity", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key missing identity record"))?;
    let key_value = value_to_iovalue(&fields[2]);
    let key_fields = key_value
        .collect_simple_record("verification-key", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key missing verification-key record"))?;
    let algorithm = required_string(&key_fields[0], "signed receipt key algorithm")?;
    if algorithm != SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt key algorithm {algorithm}; expected {SIGNATURE_ALGORITHM}"
        )));
    }
    let status_value = value_to_iovalue(&fields[3]);
    let status = status_value
        .collect_simple_record("status", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key missing status record"))?;
    let predecessor_value = value_to_iovalue(&fields[4]);
    let predecessor = predecessor_value
        .collect_simple_record("predecessor", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key missing predecessor record"))?;
    let checks = parse_signed_checks(&fields[5])?;
    require_signed_check(&checks, "key-id-bound")?;
    require_signed_check(&checks, "key-material-bound")?;
    require_signed_check(&checks, "key-record-is-evidence-only")?;
    Ok(SignedReceiptKey {
        key_ref: canonical_hash(value)?,
        key_id: required_string(&identity[0], "signed receipt key id")?,
        signer: required_string(&identity[1], "signed receipt key signer")?,
        trust_root: required_string(&identity[2], "signed receipt key trust root")?,
        key: required_string(&key_fields[1], "signed receipt key material")?,
        status: required_string(&status[0], "signed receipt key status")?,
        generation: required_u64(&status[1], "signed receipt key generation")?,
        predecessor_ref: optional_ref(&predecessor[0], "signed receipt key predecessor")?,
    })
}

pub fn parse_signed_receipt_key_revocation(value: &IoValue) -> Result<SignedReceiptKeyRevocation> {
    let fields = value
        .collect_simple_record("signed-receipt-key-revocation-v1", Some(5))
        .ok_or_else(|| MoltenError::invalid_harness("expected <signed-receipt-key-revocation-v1 ...>"))?;
    let schema = required_string(&fields[0], "signed receipt key revocation schema")?;
    if schema != EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt key revocation schema {schema}; expected {EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA}"
        )));
    }
    let key_value = value_to_iovalue(&fields[1]);
    let key_fields = key_value
        .collect_simple_record("key", Some(4))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key revocation missing key record"))?;
    let superseded_value = value_to_iovalue(&fields[3]);
    let superseded = superseded_value
        .collect_simple_record("superseded-by", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt key revocation missing superseded-by record"))?;
    let checks = parse_signed_checks(&fields[4])?;
    require_signed_check(&checks, "key-ref-bound")?;
    require_signed_check(&checks, "key-revocation-currentness-bound")?;
    require_signed_check(&checks, "key-revocation-is-evidence-only")?;
    Ok(SignedReceiptKeyRevocation {
        revocation_ref: canonical_hash(value)?,
        key_ref: required_string(&key_fields[0], "signed receipt revoked key ref")?,
        key_id: required_string(&key_fields[1], "signed receipt revoked key id")?,
        signer: required_string(&key_fields[2], "signed receipt revoked key signer")?,
        trust_root: required_string(&key_fields[3], "signed receipt revoked key trust root")?,
        reason: required_record_string(&fields[2], "reason", "signed receipt key revocation reason")?,
        superseded_by: optional_ref(&superseded[0], "signed receipt key superseded-by ref")?,
    })
}
