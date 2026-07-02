
fn sign_and_verify_promotion(
    dir: &std::path::Path,
    keys: &Keys,
    receipt_path: &std::path::Path,
    receipt_ref: &str,
) -> CliResult<()> {
    let signed_path = dir.join("release-promotion-gate.signed.preserves");
    let sign_promotion = molten_cmd()
        .args(["receipts", "sign"])
        .arg(receipt_path)
        .args(["--out"])
        .arg(&signed_path)
        .args([
            "--signer",
            "release-signer",
            "--purpose",
            "release-promotion",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
        ])
        .output()?;
    assert_success(&sign_promotion, "sign release promotion receipt");
    let verify_signed_promotion = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(&signed_path)
        .args([
            "--purpose",
            "release-promotion",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--key-ref"])
        .arg(&keys.key_ref)
        .args(["--signer", "release-signer", "--subject-ref"])
        .arg(receipt_ref)
        .output()?;
    assert_success(&verify_signed_promotion, "verify signed release promotion receipt");
    std::fs::write(dir.join("release-promotion-gate-signed-verify.txt"), stdout(&verify_signed_promotion))?;
    Ok(())
}

fn write_promotion_summary(
    dir: &std::path::Path,
    keys: &Keys,
    name: &str,
    stdout_name: Option<&str>,
    expected_decision: &str,
) -> CliResult<PromotionRefs> {
    let summary_path = dir.join(name);
    let summary = molten_cmd()
        .args(["dogfood", "release-promotion-summary", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&summary_path)
        .args(["--signed-key-ledger"])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args([
            "--signed-signer",
            "release-signer",
            "--signed-trust-root",
            "release-root",
        ])
        .output()?;
    assert_success(&summary, "dogfood release-promotion-summary");
    if let Some(stdout_name) = stdout_name {
        std::fs::write(dir.join(stdout_name), stdout(&summary))?;
    }
    let parsed = molten::operator_dogfood::parse_release_promotion_summary(&read_preserves(&summary_path)?)?;
    assert_eq!(parsed.decision, expected_decision);
    Ok(PromotionRefs {
        promotion_ref: parsed.promotion_ref,
        summary_ref: parsed.summary_ref,
    })
}

fn missing_member_summary(dir: &std::path::Path, keys: &Keys) -> CliResult<()> {
    let signed_path = dir.join("release-promotion-gate.signed.preserves");
    let missing_path = dir.join("release-promotion-gate.signed.missing");
    std::fs::rename(&signed_path, &missing_path)?;
    let _refs = write_promotion_summary(dir, keys, "release-promotion-summary-missing-signed.preserves", None, "deny")?;
    std::fs::rename(&missing_path, &signed_path)?;
    Ok(())
}

fn archive_case(dir: &std::path::Path, promotion: &PromotionSummary) -> CliResult<ArchiveCase> {
    let archive_path = dir.join("release-evidence.tar.zst");
    let manifest = dir.join("release-export-manifest.preserves");
    let release_export = molten_cmd()
        .args(["dogfood", "release-export", "--output-path"])
        .arg(dir)
        .args(["--out"])
        .arg(&archive_path)
        .args(["--manifest-out"])
        .arg(&manifest)
        .output()?;
    assert_success(&release_export, "dogfood release-export");
    assert!(stdout(&release_export).contains("release-export manifest="));
    let parsed_export = molten::operator_dogfood::parse_release_export_manifest(&read_preserves(&manifest)?)?;
    assert_eq!(parsed_export.promotion_summary_ref, promotion.summary_ref);

    let verify_path = dir.join("release-export-verify.preserves");
    let verify = molten_cmd()
        .args(["dogfood", "release-export-verify", "--bundle"])
        .arg(&archive_path)
        .args(["--receipt-out"])
        .arg(&verify_path)
        .output()?;
    assert_success(&verify, "dogfood release-export-verify");
    assert!(stdout(&verify).contains("decision=pass"));
    let parsed_verify = molten::operator_dogfood::parse_release_export_verify_receipt(&read_preserves(&verify_path)?)?;
    assert_eq!(parsed_verify.decision, "pass");
    Ok(ArchiveCase {
        manifest,
        member_refs: parsed_export.member_refs,
    })
}

fn archive_denials(dir: &std::path::Path, archive: &ArchiveCase) -> CliResult<()> {
    let missing_manifest = dir.join("release-evidence-missing-manifest.tar.zst");
    write_release_export_test_archive(dir, &missing_manifest, None, &archive.member_refs)?;
    verify_archive_deny(dir, &missing_manifest, "release-export-verify-missing-manifest.preserves")?;

    let extra = dir.join("release-evidence-extra.tar.zst");
    write_release_export_test_archive_with_extra(
        dir,
        &extra,
        &archive.manifest,
        &archive.member_refs,
        ExtraArchiveMember {
            name: "unexpected.txt",
            bytes: b"extra evidence",
        },
    )?;
    verify_archive_deny(dir, &extra, "release-export-verify-extra.preserves")?;

    let tampered = dir.join("release-evidence-tampered.tar.zst");
    write_release_export_test_archive_with_tamper(dir, &tampered, &archive.manifest, &archive.member_refs)?;
    verify_archive_deny(dir, &tampered, "release-export-verify-tampered.preserves")?;

    let duplicate = dir.join("release-evidence-duplicate.tar.zst");
    write_release_export_test_archive_with_duplicate(dir, &duplicate, &archive.manifest, &archive.member_refs)?;
    verify_archive_deny(dir, &duplicate, "release-export-verify-duplicate.preserves")?;
    Ok(())
}

fn verify_archive_deny(dir: &std::path::Path, archive_path: &std::path::Path, receipt_name: &str) -> CliResult<()> {
    let receipt_path = dir.join(receipt_name);
    let verify = molten_cmd()
        .args(["dogfood", "release-export-verify", "--bundle"])
        .arg(archive_path)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .output()?;
    assert_success(&verify, "dogfood release-export-verify emits deny receipt");
    let parsed = molten::operator_dogfood::parse_release_export_verify_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(parsed.decision, "deny");
    Ok(())
}

fn wrong_signer_denial(dir: &std::path::Path, base: &BaseOutputs, members: &MemberFiles) -> CliResult<()> {
    let receipt_path = dir.join("release-evidence-bundle-verify-wrong-signer.preserves");
    let mut verify = molten_cmd();
    verify
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key",
            "release-key",
            "--signed-signer",
            "wrong-signer",
        ]);
    add_member_args(&mut verify, members);
    let output = verify.output()?;
    assert_success(&output, "dogfood release-bundle-verify wrong signer");
    let receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("signer")));
    Ok(())
}

fn revoked_key_denial(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<()> {
    let revoke_key = molten_cmd()
        .args(["receipts", "key", "revoke"])
        .arg(&keys.key_ref)
        .args(["--ledger"])
        .arg(&keys.ledger)
        .args(["--reason", "test-revoked"])
        .output()?;
    assert_success(&revoke_key, "receipts key revoke");
    let revoked_verify = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(dir.join("dogfood-report.signed.preserves"))
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--key-ref"])
        .arg(&keys.key_ref)
        .output()?;
    assert_failure(&revoked_verify, "revoked key denies signed receipt");
    assert!(stderr(&revoked_verify).contains("revoked"));
    revoked_promotion_denial(dir, keys, bundles)?;
    let rotate_key = molten_cmd()
        .args(["receipts", "key", "rotate"])
        .arg(&keys.key_ref)
        .args(["--ledger"])
        .arg(&keys.ledger)
        .args(["--new-key-id", "release-key-2", "--new-key", "release-key-2"])
        .output()?;
    assert_failure(&rotate_key, "cannot rotate already revoked key");
    assert!(stderr(&rotate_key).contains("already revoked"));
    Ok(())
}

fn revoked_promotion_denial(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<()> {
    let receipt_path = dir.join("release-promotion-gate-revoked.preserves");
    let revoked_promotion = molten_cmd()
        .args(["dogfood", "release-promote", "--output-path"])
        .arg(dir)
        .args(["--bundle-verify"])
        .arg(&bundles.keyring_verify)
        .args(["--receipt-out"])
        .arg(&receipt_path)
        .args(["--signed-key-ledger"])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args([
            "--signed-signer",
            "release-signer",
            "--signed-trust-root",
            "release-root",
            "--source-evidence",
            "source:cli-dogfood-fixture",
            "--octet-evidence",
            "octet:clean-fixture",
            "--cairn-evidence",
            "cairn:strict-fixture",
        ])
        .output()?;
    assert_success(&revoked_promotion, "dogfood release-promote revoked key emits deny receipt");
    let receipt = molten::operator_dogfood::parse_release_promotion_gate_receipt(&read_preserves(&receipt_path)?)?;
    assert_eq!(receipt.decision, "deny");
    Ok(())
}
