
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

fn fixture_retention(core: &FixtureCore) -> Result<crate::retention::Evaluation> {
    let retention_root = secrets_fixture_retention_root(&core.secret.secret_ref)?;
    crate::retention::evaluate(crate::retention::EvaluationInput {
        root: &retention_root,
        object_ref: &core.secret.secret_ref,
        object_kind: "secret-ref",
        retention_class: crate::retention::CLASS_PRIVATE_SECRET_REF,
        action: crate::retention::ACTION_REDACT,
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

fn fixture_cleanup(core: &FixtureCore, retention: &crate::retention::Evaluation) -> Result<SecretCleanupReceipt> {
    let tombstone = retention
        .tombstone
        .as_ref()
        .ok_or_else(|| MoltenError::invalid_harness("secrets cleanup retention missing tombstone"))?;
    let value = secret_cleanup_receipt_value(&SecretCleanupInput {
        secret_ref: core.secret.secret_ref.clone(),
        revocation_ref: fixture_ref("secret-revocation"),
        tombstone_ref: tombstone.tombstone_ref.clone(),
        retention_refs: vec![retention.receipt.receipt_ref.clone()],
        retention_receipts: vec![retention.receipt.value.clone()],
        retention_tombstones: vec![tombstone.value.clone()],
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
