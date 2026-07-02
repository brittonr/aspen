
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
    retention: crate::retention::Evaluation,
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
