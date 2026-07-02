
fn key_set(dir: &std::path::Path) -> CliResult<Keys> {
    let ledger = dir.join("signed-keyring");
    let key_import = molten_cmd()
        .args(["receipts", "key", "import", "--ledger"])
        .arg(&ledger)
        .args([
            "--key-id",
            "release-key-1",
            "--signer",
            "release-signer",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
        ])
        .output()?;
    assert_success(&key_import, "receipts key import");
    let key_import_stdout = stdout(&key_import);
    std::fs::write(dir.join("signed-keyring-import.txt"), &key_import_stdout)?;
    let key_ref = key_import_stdout
        .split_whitespace()
        .find_map(|field| field.strip_prefix("key="))
        .ok_or_else(|| test_error("key import output did not include key ref"))?
        .to_string();
    let key_list = molten_cmd().args(["receipts", "key", "list", "--ledger"]).arg(&ledger).output()?;
    assert_success(&key_list, "receipts key list");
    assert!(stdout(&key_list).contains("release-key-1"));
    let key_show = molten_cmd()
        .args(["receipts", "key", "show"])
        .arg(&key_ref)
        .args(["--ledger"])
        .arg(&ledger)
        .output()?;
    assert_success(&key_show, "receipts key show");
    assert!(stdout(&key_show).contains("evidence-only=pass"));
    rotate_seed_key(&ledger)?;
    Ok(Keys { ledger, key_ref })
}

fn rotate_seed_key(ledger: &std::path::Path) -> CliResult<()> {
    let rotate_seed = molten_cmd()
        .args(["receipts", "key", "import", "--ledger"])
        .arg(ledger)
        .args([
            "--key-id",
            "rotate-key-1",
            "--signer",
            "rotate-signer",
            "--trust-root",
            "rotate-root",
            "--key",
            "rotate-key",
        ])
        .output()?;
    assert_success(&rotate_seed, "receipts key import rotate seed");
    let rotate_key_ref = stdout(&rotate_seed)
        .split_whitespace()
        .find_map(|field| field.strip_prefix("key="))
        .ok_or_else(|| test_error("rotate seed output did not include key ref"))?
        .to_string();
    let rotate_success = molten_cmd()
        .args(["receipts", "key", "rotate"])
        .arg(&rotate_key_ref)
        .args(["--ledger"])
        .arg(ledger)
        .args(["--new-key-id", "rotate-key-2", "--new-key", "rotate-key-2"])
        .output()?;
    assert_success(&rotate_success, "receipts key rotate");
    assert!(stdout(&rotate_success).contains("new-key-id=rotate-key-2"));
    Ok(())
}

fn member_files(dir: &std::path::Path, base: &BaseOutputs, keys: &Keys) -> CliResult<MemberFiles> {
    let members = MemberFiles {
        report: dir.join("dogfood-report.signed.preserves"),
        gate: dir.join("release-gate.signed.preserves"),
        replay_verify: dir.join("replay-verify.signed.preserves"),
        replay_index: dir.join("replay-evidence-index.signed.preserves"),
        nix_evidence: dir.join("nix-dogfood-evidence.signed.preserves"),
        nix_verify: dir.join("nix-dogfood-verify.signed.preserves"),
    };
    for (receipt_path, signed_path) in [
        (&base.report, &members.report),
        (&base.gate, &members.gate),
        (&base.replay_verify, &members.replay_verify),
        (&base.replay_index, &members.replay_index),
        (&base.nix_evidence, &members.nix_evidence),
        (&base.nix_verify, &members.nix_verify),
    ] {
        let signed = molten_cmd()
            .args(["receipts", "sign"])
            .arg(receipt_path)
            .args(["--out"])
            .arg(signed_path)
            .args([
                "--signer",
                "release-signer",
                "--purpose",
                "release-evidence",
                "--trust-root",
                "release-root",
                "--key",
                "release-key",
            ])
            .output()?;
        assert_success(&signed, "receipts sign release member");
    }
    verify_member_file(base, keys, &members.report)?;
    Ok(members)
}

fn verify_member_file(base: &BaseOutputs, keys: &Keys, signed_report: &std::path::Path) -> CliResult<()> {
    let verify_signed = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(signed_report)
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key",
            "release-key",
            "--signer",
            "release-signer",
            "--subject-ref",
        ])
        .arg(&base.report_ref)
        .output()?;
    assert_success(&verify_signed, "receipts verify-signed release member");
    assert!(stdout(&verify_signed).contains("evidence-only=pass"));
    let verify_signed_keyring = molten_cmd()
        .args(["receipts", "verify-signed"])
        .arg(signed_report)
        .args([
            "--purpose",
            "release-evidence",
            "--trust-root",
            "release-root",
            "--key-ledger",
        ])
        .arg(&keys.ledger)
        .args([
            "--key-ref",
            &keys.key_ref,
            "--signer",
            "release-signer",
            "--subject-ref",
        ])
        .arg(&base.report_ref)
        .output()?;
    assert_success(&verify_signed_keyring, "receipts verify-signed release member with keyring");
    assert!(stdout(&verify_signed_keyring).contains("keyring=current"));
    Ok(())
}

fn bundle_checks(
    dir: &std::path::Path,
    base: &BaseOutputs,
    keys: &Keys,
    members: &MemberFiles,
) -> CliResult<BundleChecks> {
    let direct = dir.join("release-evidence-bundle-verify-signed.preserves");
    let mut verify_direct = molten_cmd();
    verify_direct
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&direct)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key",
            "release-key",
            "--signed-signer",
            "release-signer",
        ]);
    add_member_args(&mut verify_direct, members);
    let direct_output = verify_direct.output()?;
    assert_success(&direct_output, "dogfood release-bundle-verify signed members");
    let direct_receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&direct)?)?;
    assert_eq!(direct_receipt.decision, "pass");

    let keyring_verify = dir.join("release-evidence-bundle-verify-keyring.preserves");
    let mut verify_keyring = molten_cmd();
    verify_keyring
        .args(["dogfood", "release-bundle-verify", "--output-path"])
        .arg(dir)
        .args(["--bundle"])
        .arg(&base.bundle)
        .args(["--receipt-out"])
        .arg(&keyring_verify)
        .args([
            "--require-signed-members",
            "--signed-purpose",
            "release-evidence",
            "--signed-trust-root",
            "release-root",
            "--signed-key-ledger",
        ])
        .arg(&keys.ledger)
        .args(["--signed-key-ref"])
        .arg(&keys.key_ref)
        .args(["--signed-signer", "release-signer"]);
    add_member_args(&mut verify_keyring, members);
    let keyring_output = verify_keyring.output()?;
    assert_success(&keyring_output, "dogfood release-bundle-verify signed keyring members");
    let keyring_receipt =
        molten::operator_dogfood::parse_release_evidence_bundle_verify_receipt(&read_preserves(&keyring_verify)?)?;
    assert_eq!(keyring_receipt.decision, "pass");
    Ok(BundleChecks {
        path: base.bundle.clone(),
        keyring_verify,
    })
}

fn add_member_args(command: &mut std::process::Command, members: &MemberFiles) {
    for signed_path in [
        &members.report,
        &members.gate,
        &members.replay_verify,
        &members.replay_index,
        &members.nix_evidence,
        &members.nix_verify,
    ] {
        command.args(["--signed-member"]).arg(signed_path);
    }
}

fn promotion_summary(dir: &std::path::Path, keys: &Keys, bundles: &BundleChecks) -> CliResult<PromotionSummary> {
    let promotion_receipt_path = dir.join("release-promotion-gate.preserves");
    let promotion = molten_cmd()
        .args(["dogfood", "release-promote", "--output-path"])
        .arg(dir)
        .args(["--bundle-verify"])
        .arg(&bundles.keyring_verify)
        .args(["--receipt-out"])
        .arg(&promotion_receipt_path)
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
    assert_success(&promotion, "dogfood release-promote");
    assert!(stdout(&promotion).contains("decision=pass"));
    std::fs::write(dir.join("release-promotion-gate.txt"), stdout(&promotion))?;
    let promotion_receipt =
        molten::operator_dogfood::parse_release_promotion_gate_receipt(&read_preserves(&promotion_receipt_path)?)?;
    assert_eq!(promotion_receipt.decision, "pass");
    sign_and_verify_promotion(dir, keys, &promotion_receipt_path, &promotion_receipt.receipt_ref)?;
    let refs = write_promotion_summary(
        dir,
        keys,
        "release-promotion-summary.preserves",
        Some("release-promotion-summary.txt"),
        "pass",
    )?;
    assert_eq!(refs.promotion_ref, promotion_receipt.receipt_ref);
    Ok(PromotionSummary {
        summary_ref: refs.summary_ref,
    })
}
