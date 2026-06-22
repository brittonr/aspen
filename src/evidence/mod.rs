use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_KEY_REVOCATION_SCHEMA;
use crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_KEY_SCHEMA;
use crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;
use crate::preserves_rail::value_to_iovalue;

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
    receipt_value: IOValue,
}

struct SignerParts {
    signer: String,
    purpose: String,
    trust_root: String,
}

pub struct SignReceiptInput<'a> {
    pub receipt: &'a IOValue,
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

pub fn sign_receipt(input: &SignReceiptInput<'_>) -> Result<IOValue> {
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

pub fn signed_receipt_key_value(input: &SignedReceiptKeyInput<'_>) -> Result<IOValue> {
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

pub fn signed_receipt_key_revocation_value(input: &SignedReceiptKeyRevocationInput<'_>) -> Result<IOValue> {
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

pub fn parse_signed_receipt_key(value: &IOValue) -> Result<SignedReceiptKey> {
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

pub fn parse_signed_receipt_key_revocation(value: &IOValue) -> Result<SignedReceiptKeyRevocation> {
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

pub fn verify_signed_receipt(
    value: &IOValue,
    required_purpose: &str,
    trust_root: &str,
    key: &str,
) -> Result<SignedReceipt> {
    verify_signed_receipt_with_policy(value, &VerifySignedReceiptPolicy {
        required_purpose,
        trust_root,
        key,
        expected_signer: None,
        expected_subject_ref: None,
    })
}

pub fn verify_signed_receipt_with_policy(
    value: &IOValue,
    policy: &VerifySignedReceiptPolicy<'_>,
) -> Result<SignedReceipt> {
    let signed = value
        .collect_simple_record("signed-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <signed-receipt-v1 ...>"))?;
    let schema = required_string(&signed[0], "signed receipt schema")?;
    if schema != EVIDENCE_SIGNED_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt schema {schema}; expected {EVIDENCE_SIGNED_RECEIPT_SCHEMA}"
        )));
    }
    let subject = subject_parts(&signed[1])?;
    if let Some(expected_subject_ref) = policy.expected_subject_ref
        && subject.subject_ref != expected_subject_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt subject ref {} does not match required subject ref {expected_subject_ref}",
            subject.subject_ref
        )));
    }

    let signer = signer_parts(&signed[2])?;
    if let Some(expected_signer) = policy.expected_signer
        && signer.signer != expected_signer
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt signer {} does not match required signer {expected_signer}",
            signer.signer
        )));
    }
    if signer.purpose != policy.required_purpose {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt purpose {} does not satisfy required purpose {}",
            signer.purpose, policy.required_purpose
        )));
    }
    if signer.trust_root != policy.trust_root {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt trust root {} does not match required trust root {}",
            signer.trust_root, policy.trust_root
        )));
    }

    let algorithm = required_record_string(&signed[3], "algorithm", "signed receipt algorithm")?;
    if algorithm != SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt algorithm {algorithm}; expected {SIGNATURE_ALGORITHM}"
        )));
    }
    let signature = required_record_string(&signed[4], "signature", "signed receipt signature")?;
    let expected_signature =
        signature_for(&subject.receipt_value, &signer.signer, &signer.purpose, &signer.trust_root, policy.key)?;
    if signature != expected_signature {
        return Err(MoltenError::invalid_harness("signed receipt signature verification failed"));
    }
    let parents = parent_refs(&signed[5])?;
    let checks = parse_signed_checks(&signed[6])?;
    require_signed_check(&checks, "subject-ref-binding")?;
    require_signed_check(&checks, "signature-covers-canonical-receipt")?;
    require_signed_check(&checks, "parent-receipt-refs")?;
    require_signed_check(&checks, "signed-receipt-is-evidence-only")?;
    Ok(SignedReceipt {
        envelope_ref: canonical_hash(value)?,
        subject_ref: subject.subject_ref,
        signer: signer.signer,
        purpose: signer.purpose,
        trust_root: signer.trust_root,
        algorithm,
        parents,
    })
}

pub fn verify_signed_receipt_with_keyring_policy(
    value: &IOValue,
    policy: &VerifySignedReceiptKeyringPolicy<'_>,
) -> Result<SignedReceiptWithKey> {
    let envelope = signed_receipt_envelope(value)?;
    require_envelope(&envelope, policy)?;
    let key = select_key(&envelope, policy)?;
    let receipt = verify_signed_receipt_with_policy(value, &VerifySignedReceiptPolicy {
        required_purpose: policy.required_purpose,
        trust_root: policy.trust_root,
        key: &key.key,
        expected_signer: Some(&key.signer),
        expected_subject_ref: policy.expected_subject_ref,
    })?;
    Ok(SignedReceiptWithKey {
        receipt,
        key_ref: key.key_ref.clone(),
        key_id: key.key_id.clone(),
    })
}

fn require_envelope(envelope: &SignedReceiptEnvelope, policy: &VerifySignedReceiptKeyringPolicy<'_>) -> Result<()> {
    if envelope.purpose != policy.required_purpose {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt purpose {} does not satisfy required purpose {}",
            envelope.purpose, policy.required_purpose
        )));
    }
    if envelope.trust_root != policy.trust_root {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt trust root {} does not match required trust root {}",
            envelope.trust_root, policy.trust_root
        )));
    }
    if let Some(expected_signer) = policy.expected_signer
        && envelope.signer != expected_signer
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt signer {} does not match required signer {expected_signer}",
            envelope.signer
        )));
    }
    if let Some(expected_subject_ref) = policy.expected_subject_ref
        && envelope.subject_ref != expected_subject_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt subject ref {} does not match required subject ref {expected_subject_ref}",
            envelope.subject_ref
        )));
    }
    Ok(())
}

fn select_key<'a>(
    envelope: &SignedReceiptEnvelope,
    policy: &'a VerifySignedReceiptKeyringPolicy<'a>,
) -> Result<&'a SignedReceiptKey> {
    let matches = matching_keys(envelope, policy)?;
    if matches.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt keyring has no key for signer {} trust-root {}{}{}",
            envelope.signer,
            envelope.trust_root,
            key_ref_suffix(policy.required_key_ref),
            key_id_suffix(policy.required_key_id)
        )));
    }
    eligible_key(envelope, policy, matches)
}

fn matching_keys<'a>(
    envelope: &SignedReceiptEnvelope,
    policy: &'a VerifySignedReceiptKeyringPolicy<'a>,
) -> Result<Vec<&'a SignedReceiptKey>> {
    let mut matches = Vec::new();
    for key in policy.keys {
        if key.signer != envelope.signer || key.trust_root != envelope.trust_root {
            continue;
        }
        if let Some(required_key_ref) = policy.required_key_ref
            && key.key_ref != required_key_ref
        {
            continue;
        }
        if let Some(required_key_id) = policy.required_key_id
            && key.key_id != required_key_id
        {
            continue;
        }
        push_bounded(&mut matches, key, MAX_SIGNED_KEY_RECORDS, "signed receipt keyring matches")?;
    }
    Ok(matches)
}

fn eligible_key<'a>(
    envelope: &SignedReceiptEnvelope,
    policy: &VerifySignedReceiptKeyringPolicy<'_>,
    matches: Vec<&'a SignedReceiptKey>,
) -> Result<&'a SignedReceiptKey> {
    let mut eligible = Vec::new();
    let mut blocked_reasons = Vec::new();
    for key in matches {
        if key.status != SIGNED_RECEIPT_KEY_STATUS_CURRENT {
            push_bounded(
                &mut blocked_reasons,
                format!("key {} status is {}", key.key_ref, key.status),
                MAX_SIGNED_KEY_RECORDS,
                "signed receipt keyring blocked key diagnostics",
            )?;
            continue;
        }
        if let Some(revocation) = policy.revocations.iter().find(|revocation| revocation.key_ref == key.key_ref) {
            push_bounded(
                &mut blocked_reasons,
                format!("key {} is revoked by {}", key.key_ref, revocation.revocation_ref),
                MAX_SIGNED_KEY_RECORDS,
                "signed receipt keyring blocked key diagnostics",
            )?;
            continue;
        }
        push_bounded(&mut eligible, key, MAX_SIGNED_KEY_RECORDS, "signed receipt keyring eligible keys")?;
    }
    if eligible.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt keyring has no current unrevoked key for signer {} trust-root {}: {}",
            envelope.signer,
            envelope.trust_root,
            blocked_reasons.join("; ")
        )));
    }
    if eligible.len() > 1 {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt keyring matched {} current keys; specify --key-ref or --key-id",
            eligible.len()
        )));
    }
    Ok(eligible[0])
}

pub fn signed_receipt_summary(value: &IOValue) -> Result<String> {
    let signed = value
        .collect_simple_record("signed-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <signed-receipt-v1 ...>"))?;
    let subject = value_to_iovalue(&signed[1]);
    let subject_record = subject
        .collect_simple_record("subject", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing subject record"))?;
    let subject_ref = required_string(&subject_record[0], "signed receipt subject ref")?;
    let signer_record_value = value_to_iovalue(&signed[2]);
    let signer_record = signer_record_value
        .collect_simple_record("signer", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing signer record"))?;
    let signer = required_string(&signer_record[0], "signed receipt signer")?;
    let purpose = required_string(&signer_record[1], "signed receipt purpose")?;
    Ok(format!(
        "signed receipt {}\nsubject={}\nsigner={}\npurpose={}",
        canonical_hash(value)?,
        subject_ref,
        signer,
        purpose
    ))
}

fn subject_parts(value: &preserves::Value<IOValue>) -> Result<SubjectParts> {
    let subject = value_to_iovalue(value);
    let subject_record = subject
        .collect_simple_record("subject", Some(2))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing subject record"))?;
    let subject_ref = required_string(&subject_record[0], "signed receipt subject ref")?;
    let receipt_value = value_to_iovalue(&subject_record[1]);
    let actual_ref = canonical_hash(&receipt_value)?;
    if actual_ref != subject_ref {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt subject ref mismatch: got {subject_ref}, expected {actual_ref}"
        )));
    }
    Ok(SubjectParts {
        subject_ref,
        receipt_value,
    })
}

fn signer_parts(value: &preserves::Value<IOValue>) -> Result<SignerParts> {
    let signer_record_value = value_to_iovalue(value);
    let signer_record = signer_record_value
        .collect_simple_record("signer", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing signer record"))?;
    Ok(SignerParts {
        signer: required_string(&signer_record[0], "signed receipt signer")?,
        purpose: required_string(&signer_record[1], "signed receipt purpose")?,
        trust_root: required_string(&signer_record[2], "signed receipt trust root")?,
    })
}

fn parent_refs(value: &preserves::Value<IOValue>) -> Result<Vec<String>> {
    let parents_value = value_to_iovalue(value);
    let parents_record = parents_value
        .collect_simple_record("parents", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing parents record"))?;
    let parent_values = parents_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt parents must be a sequence"))?;
    if parent_values.len() > MAX_SIGNED_RECEIPT_PARENTS {
        return Err(MoltenError::invalid_harness("signed receipt parent ref count exceeds bound"));
    }
    let mut parents = Vec::with_capacity(parent_values.len());
    for parent in parent_values.iter() {
        push_bounded(
            &mut parents,
            required_string(parent, "signed receipt parent ref")?,
            MAX_SIGNED_RECEIPT_PARENTS,
            "signed receipt parent refs",
        )?;
    }
    Ok(parents)
}

fn signed_receipt_envelope(value: &IOValue) -> Result<SignedReceiptEnvelope> {
    let signed = value
        .collect_simple_record("signed-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <signed-receipt-v1 ...>"))?;
    let schema = required_string(&signed[0], "signed receipt schema")?;
    if schema != EVIDENCE_SIGNED_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt schema {schema}; expected {EVIDENCE_SIGNED_RECEIPT_SCHEMA}"
        )));
    }
    let subject = subject_parts(&signed[1])?;
    let signer = signer_parts(&signed[2])?;
    Ok(SignedReceiptEnvelope {
        subject_ref: subject.subject_ref,
        signer: signer.signer,
        purpose: signer.purpose,
        trust_root: signer.trust_root,
    })
}

fn parse_signed_checks(value: &preserves::Value<IOValue>) -> Result<Vec<(String, String)>> {
    let checks_value = value_to_iovalue(value);
    let checks_record = checks_value
        .collect_simple_record("checks", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing checks record"))?;
    let check_values = checks_record[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt checks must be a sequence"))?;
    let mut checks = Vec::with_capacity(check_values.len());
    for check in check_values.iter() {
        let check_value = value_to_iovalue(check);
        let fields = check_value
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected signed receipt check record"))?;
        push_bounded(
            &mut checks,
            (
                required_string(&fields[0], "signed receipt check name")?,
                required_string(&fields[1], "signed receipt check status")?,
            ),
            MAX_SIGNED_RECEIPT_PARENTS,
            "signed receipt checks",
        )?;
    }
    Ok(checks)
}

fn require_signed_check(checks: &[(String, String)], name: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("signed receipt missing pass check {name}")))
    }
}

fn optional_ref_value(reference: Option<&str>) -> IOValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_ref(value: &preserves::Value<IOValue>, field: &str) -> Result<Option<String>> {
    let inner = value_to_iovalue(value);
    if inner.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = inner.collect_simple_record("some", Some(1)) {
        return Ok(Some(required_string(&fields[0], field)?));
    }
    Err(MoltenError::invalid_harness(format!("expected <some ref> or <none> for {field}")))
}

fn require_non_empty(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn required_u64(value: &preserves::Value<IOValue>, field: &str) -> Result<u64> {
    let number = value.as_u64().ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {field}")))
}

fn key_ref_suffix(key_ref: Option<&str>) -> String {
    key_ref.map_or_else(String::new, |value| format!(" key-ref {value}"))
}

fn key_id_suffix(key_id: Option<&str>) -> String {
    key_id.map_or_else(String::new, |value| format!(" key-id {value}"))
}

fn signature_for(receipt: &IOValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<String> {
    let mut material = canonical_bytes(receipt)?;
    material.extend_from_slice(signer.as_bytes());
    material.push(0);
    material.extend_from_slice(purpose.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_root.as_bytes());
    material.push(0);
    material.extend_from_slice(key.as_bytes());
    Ok(content_ref_from_bytes(&material))
}

fn required_record_string(value: &preserves::Value<IOValue>, label: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&record[0], field)
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn required_string(value: &preserves::Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preserves_rail::parse_text;

    #[test]
    fn signed_receipt_verification_rejects_wrong_purpose_and_key() {
        let receipt = parse_text("<gate-receipt-placeholder \"ok\">").expect("parse receipt");
        let signed = sign_receipt(&SignReceiptInput {
            receipt: &receipt,
            signer: "local",
            purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "root",
            key: "key",
            parents: &[],
        })
        .expect("sign receipt");
        let verified = verify_signed_receipt(&signed, PASS_EVIDENCE_PURPOSE, "root", "key").expect("verify receipt");
        assert_eq!(verified.subject_ref, canonical_hash(&receipt).expect("subject ref"));
        let wrong_purpose = verify_signed_receipt(&signed, "diagnostic", "root", "key").expect_err("wrong purpose");
        assert!(wrong_purpose.to_string().contains("purpose"));
        let wrong_key = verify_signed_receipt(&signed, PASS_EVIDENCE_PURPOSE, "root", "wrong").expect_err("wrong key");
        assert!(wrong_key.to_string().contains("signature verification failed"));
        let wrong_signer = verify_signed_receipt_with_policy(&signed, &VerifySignedReceiptPolicy {
            required_purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "root",
            key: "key",
            expected_signer: Some("other-signer"),
            expected_subject_ref: None,
        })
        .expect_err("wrong signer");
        assert!(wrong_signer.to_string().contains("signer"));
        let wrong_subject = verify_signed_receipt_with_policy(&signed, &VerifySignedReceiptPolicy {
            required_purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "root",
            key: "key",
            expected_signer: Some("local"),
            expected_subject_ref: Some("blake3:wrong-subject"),
        })
        .expect_err("wrong subject");
        assert!(wrong_subject.to_string().contains("subject ref"));
    }

    #[test]
    fn signed_receipt_keyring_enforces_current_unrevoked_keys() {
        let receipt = parse_text("<gate-receipt-placeholder \"ok\">").expect("parse receipt");
        let signed = sign_receipt(&SignReceiptInput {
            receipt: &receipt,
            signer: "release-signer",
            purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "release-root",
            key: "release-key",
            parents: &[],
        })
        .expect("sign receipt");
        let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
            key_id: "release-key-1",
            signer: "release-signer",
            trust_root: "release-root",
            key: "release-key",
            generation: 1,
            predecessor_ref: None,
        })
        .expect("key value");
        let key = parse_signed_receipt_key(&key_value).expect("parse key");
        let verified = verify_signed_receipt_with_keyring_policy(&signed, &VerifySignedReceiptKeyringPolicy {
            required_purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "release-root",
            expected_signer: Some("release-signer"),
            expected_subject_ref: Some(&canonical_hash(&receipt).expect("receipt ref")),
            required_key_ref: Some(&key.key_ref),
            required_key_id: Some("release-key-1"),
            keys: std::slice::from_ref(&key),
            revocations: &[],
        })
        .expect("verify with keyring");
        assert_eq!(verified.key_ref, key.key_ref);
        let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
            key: &key,
            reason: "compromised",
            superseded_by: None,
        })
        .expect("revocation value");
        let revocation = parse_signed_receipt_key_revocation(&revocation_value).expect("parse revocation");
        let revoked = verify_signed_receipt_with_keyring_policy(&signed, &VerifySignedReceiptKeyringPolicy {
            required_purpose: PASS_EVIDENCE_PURPOSE,
            trust_root: "release-root",
            expected_signer: Some("release-signer"),
            expected_subject_ref: None,
            required_key_ref: Some(&key.key_ref),
            required_key_id: Some("release-key-1"),
            keys: std::slice::from_ref(&key),
            revocations: std::slice::from_ref(&revocation),
        })
        .expect_err("revoked key denies");
        assert!(revoked.to_string().contains("revoked"));
    }
}
