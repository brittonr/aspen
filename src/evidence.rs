use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::EVIDENCE_SIGNED_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;

pub const SIGNATURE_ALGORITHM: &str = "blake3-local-fixture-v1";
pub const PASS_EVIDENCE_PURPOSE: &str = "pass-evidence";

const MAX_SIGNED_RECEIPT_PARENTS: usize = 256;
const _: () = assert!(MAX_SIGNED_RECEIPT_PARENTS > 0);

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
    let subject = value_to_iovalue(&signed[1]);
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
    if let Some(expected_subject_ref) = policy.expected_subject_ref
        && subject_ref != expected_subject_ref
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt subject ref {subject_ref} does not match required subject ref {expected_subject_ref}"
        )));
    }

    let signer_record_value = value_to_iovalue(&signed[2]);
    let signer_record = signer_record_value
        .collect_simple_record("signer", Some(3))
        .ok_or_else(|| MoltenError::invalid_harness("signed receipt missing signer record"))?;
    let signer = required_string(&signer_record[0], "signed receipt signer")?;
    let purpose = required_string(&signer_record[1], "signed receipt purpose")?;
    let actual_trust_root = required_string(&signer_record[2], "signed receipt trust root")?;
    if let Some(expected_signer) = policy.expected_signer
        && signer != expected_signer
    {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt signer {signer} does not match required signer {expected_signer}"
        )));
    }
    if purpose != policy.required_purpose {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt purpose {purpose} does not satisfy required purpose {}",
            policy.required_purpose
        )));
    }
    if actual_trust_root != policy.trust_root {
        return Err(MoltenError::invalid_harness(format!(
            "signed receipt trust root {actual_trust_root} does not match required trust root {}",
            policy.trust_root
        )));
    }

    let algorithm = required_record_string(&signed[3], "algorithm", "signed receipt algorithm")?;
    if algorithm != SIGNATURE_ALGORITHM {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt algorithm {algorithm}; expected {SIGNATURE_ALGORITHM}"
        )));
    }
    let signature = required_record_string(&signed[4], "signature", "signed receipt signature")?;
    let expected_signature = signature_for(&receipt_value, &signer, &purpose, &actual_trust_root, policy.key)?;
    if signature != expected_signature {
        return Err(MoltenError::invalid_harness("signed receipt signature verification failed"));
    }
    let parents_value = value_to_iovalue(&signed[5]);
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
    let checks = parse_signed_checks(&signed[6])?;
    require_signed_check(&checks, "subject-ref-binding")?;
    require_signed_check(&checks, "signature-covers-canonical-receipt")?;
    require_signed_check(&checks, "parent-receipt-refs")?;
    require_signed_check(&checks, "signed-receipt-is-evidence-only")?;
    Ok(SignedReceipt {
        envelope_ref: canonical_hash(value)?,
        subject_ref,
        signer,
        purpose,
        trust_root: actual_trust_root,
        algorithm,
        parents,
    })
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

fn signature_for(receipt: &IOValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<String> {
    let mut material = canonical_bytes(receipt)?;
    material.extend_from_slice(signer.as_bytes());
    material.push(0);
    material.extend_from_slice(purpose.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_root.as_bytes());
    material.push(0);
    material.extend_from_slice(key.as_bytes());
    Ok(format!("blake3:{}", blake3::hash(&material).to_hex()))
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
}
