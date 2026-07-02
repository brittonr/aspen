
pub fn verify_signed_receipt(
    value: &IoValue,
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
    value: &IoValue,
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
    value: &IoValue,
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

pub fn signed_receipt_summary(value: &IoValue) -> Result<String> {
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

fn subject_parts(value: &preserves::Value<IoValue>) -> Result<SubjectParts> {
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

fn signer_parts(value: &preserves::Value<IoValue>) -> Result<SignerParts> {
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
