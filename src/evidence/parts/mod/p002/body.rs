
fn parent_refs(value: &preserves::Value<IoValue>) -> Result<Vec<String>> {
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

fn signed_receipt_envelope(value: &IoValue) -> Result<SignedReceiptEnvelope> {
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

fn parse_signed_checks(value: &preserves::Value<IoValue>) -> Result<Vec<(String, String)>> {
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

fn optional_ref_value(reference: Option<&str>) -> IoValue {
    reference.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_ref(value: &preserves::Value<IoValue>, field: &str) -> Result<Option<String>> {
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

fn required_u64(value: &preserves::Value<IoValue>, field: &str) -> Result<u64> {
    let number = value.as_u64().ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?;
    number.map_err(|_| MoltenError::invalid_harness(format!("u64 out of range for {field}")))
}

fn key_ref_suffix(key_ref: Option<&str>) -> String {
    key_ref.map_or_else(String::new, |value| format!(" key-ref {value}"))
}

fn key_id_suffix(key_id: Option<&str>) -> String {
    key_id.map_or_else(String::new, |value| format!(" key-id {value}"))
}

fn signature_for(receipt: &IoValue, signer: &str, purpose: &str, trust_root: &str, key: &str) -> Result<String> {
    let mut material = crate::preserves_rail::canonical_bytes(receipt)?;
    material.extend_from_slice(signer.as_bytes());
    material.push(0);
    material.extend_from_slice(purpose.as_bytes());
    material.push(0);
    material.extend_from_slice(trust_root.as_bytes());
    material.push(0);
    material.extend_from_slice(key.as_bytes());
    Ok(crate::preserves_rail::content_ref_from_bytes(&material))
}

fn required_record_string(value: &Value<IoValue>, label: &str, field: &str) -> Result<String> {
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

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_value() -> IoValue {
        crate::preserves_rail::parse_text("<gate-receipt-placeholder \"ok\">").expect("parse receipt")
    }

    #[test]
    fn signed_receipt_verification_rejects_wrong_purpose_and_key() {
        let receipt = receipt_value();
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
        let receipt = receipt_value();
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
