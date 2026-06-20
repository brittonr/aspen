pub(super) fn bundle_export(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleaseBundleExport { output_path, out } = command else {
        return Err(super::wrong_handler("release-bundle-export"));
    };
    let bundle = molten::operator_dogfood::release_evidence_bundle_value(
        &molten::operator_dogfood::ReleaseEvidenceBundleInput {
            output_path: &output_path,
        },
    )?;
    let parsed = molten::operator_dogfood::parse_release_evidence_bundle(&bundle)?;
    super::super::io::write_file(&out, &molten::preserves_rail::to_text(&bundle)?)?;
    println!(
        "dogfood release-bundle-export bundle={} report={} release-gate={} nix-verify={}",
        parsed.bundle_ref, parsed.report_ref, parsed.release_gate_ref, parsed.nix_verify_ref
    );
    Ok(())
}

pub(super) fn bundle_verify(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleaseBundleVerify {
        output_path,
        bundle,
        receipt_out,
        signed_members,
        require_signed_members,
        signed_purpose,
        signed_trust_root,
        signed_key,
        signed_key_ledger,
        signed_key_ref,
        signed_key_id,
        signed_signer,
    } = command
    else {
        return Err(super::wrong_handler("release-bundle-verify"));
    };
    let bundle_value = super::super::io::read_preserves_file(&bundle)?;
    let signed_member_values = super::super::signed::read_preserves_files(&signed_members)?;
    crate::cli_receipts::ensure_keyring_selector_has_ledger(
        signed_key_ledger.as_deref(),
        signed_key_ref.as_deref(),
        signed_key_id.as_deref(),
    )?;
    let keyring = match signed_key_ledger.as_ref() {
        Some(ledger) => crate::cli_receipts::load_signed_receipt_keyring(ledger)?,
        None => crate::cli_receipts::SignedReceiptKeyring {
            keys: Vec::new(),
            revocations: Vec::new(),
        },
    };
    let receipt = molten::operator_dogfood::verify_release_evidence_bundle(
        &molten::operator_dogfood::ReleaseEvidenceBundleVerifyInput {
            output_path: &output_path,
            bundle_value: &bundle_value,
            signed_member_values: &signed_member_values,
            signed_purpose: &signed_purpose,
            signed_trust_root: &signed_trust_root,
            signed_key: &signed_key,
            signed_keys: &keyring.keys,
            signed_key_revocations: &keyring.revocations,
            signed_key_ref: signed_key_ref.as_deref(),
            signed_key_id: signed_key_id.as_deref(),
            signed_signer: signed_signer.as_deref(),
            is_signed_members_required: require_signed_members,
        },
    )?;
    super::super::io::write_file(&receipt_out, &molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "dogfood release-bundle-verify decision={} receipt={} bundle={}",
        receipt.decision, receipt.receipt_ref, receipt.bundle_ref
    );
    Ok(())
}

pub(super) fn promote(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleasePromote {
        output_path,
        bundle_verify,
        receipt_out,
        signed_key_ledger,
        signed_trust_root,
        signed_key_ref,
        signed_key_id,
        signed_signer,
        source_evidence,
        octet_evidence,
        cairn_evidence,
    } = command
    else {
        return Err(super::wrong_handler("release-promote"));
    };
    let bundle_verify_value = super::super::io::read_preserves_file(&bundle_verify)?;
    let keyring = crate::cli_receipts::load_signed_receipt_keyring(&signed_key_ledger)?;
    let receipt = molten::operator_dogfood::release_promotion_gate_receipt_value(
        &molten::operator_dogfood::ReleasePromotionGateInput {
            output_path: &output_path,
            bundle_verify_value: &bundle_verify_value,
            source_evidence: &source_evidence,
            octet_evidence: &octet_evidence,
            cairn_evidence: &cairn_evidence,
            signed_keys: &keyring.keys,
            signed_key_revocations: &keyring.revocations,
            signed_trust_root: &signed_trust_root,
            signed_signer: signed_signer.as_deref(),
            signed_key_ref: signed_key_ref.as_deref(),
            signed_key_id: signed_key_id.as_deref(),
        },
    )?;
    super::super::io::write_file(&receipt_out, &molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "dogfood release-promote decision={} receipt={} bundle-verify={} key={} source={} octet={} cairn={}",
        receipt.decision,
        receipt.receipt_ref,
        receipt.bundle_verify_ref,
        receipt.selected_key_ref,
        receipt.source_ref,
        receipt.octet_ref,
        receipt.cairn_ref
    );
    Ok(())
}

pub(super) fn promotion_summary(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleasePromotionSummary {
        output_path,
        out,
        signed_key_ledger,
        signed_trust_root,
        signed_key_ref,
        signed_key_id,
        signed_signer,
    } = command
    else {
        return Err(super::wrong_handler("release-promotion-summary"));
    };
    let key_ledger = signed_key_ledger.unwrap_or_else(|| output_path.join("signed-keyring"));
    let keyring = crate::cli_receipts::load_signed_receipt_keyring(&key_ledger)?;
    let summary = molten::operator_dogfood::release_promotion_summary_value(
        &molten::operator_dogfood::ReleasePromotionSummaryInput {
            output_path: &output_path,
            signed_keys: &keyring.keys,
            signed_key_revocations: &keyring.revocations,
            signed_trust_root: &signed_trust_root,
            signed_signer: signed_signer.as_deref(),
            signed_key_ref: signed_key_ref.as_deref(),
            signed_key_id: signed_key_id.as_deref(),
        },
    )?;
    super::super::io::write_file(&out, &molten::preserves_rail::to_text(&summary.value)?)?;
    println!(
        "dogfood release-promotion-summary decision={} summary={} promotion={} signed={} key={} source={} octet={} cairn={}",
        summary.decision,
        summary.summary_ref,
        summary.promotion_ref,
        summary.signed_envelope_ref,
        summary.signed_key_ref,
        summary.source_ref,
        summary.octet_ref,
        summary.cairn_ref
    );
    Ok(())
}

pub(super) fn export(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleaseExport {
        output_path,
        out,
        manifest_out,
    } = command
    else {
        return Err(super::wrong_handler("release-export"));
    };
    let manifest = molten::operator_dogfood::release_export_manifest_value(
        &molten::operator_dogfood::ReleaseExportManifestInput {
            output_path: &output_path,
        },
    )?;
    super::super::io::write_file(&manifest_out, &molten::preserves_rail::to_text(&manifest.value)?)?;
    super::super::archive::write(&output_path, &out, &manifest)?;
    println!(
        "dogfood release-export manifest={} promotion-summary={} members={} archive={}",
        manifest.manifest_ref,
        manifest.promotion_summary_ref,
        manifest.member_refs.len(),
        out.display()
    );
    Ok(())
}

pub(super) fn export_verify(command: super::Command) -> super::Outcome<()> {
    let super::Command::ReleaseExportVerify { bundle, receipt_out } = command else {
        return Err(super::wrong_handler("release-export-verify"));
    };
    let archive = super::super::archive::read(&bundle)?;
    let receipt =
        molten::operator_dogfood::verify_release_export(&molten::operator_dogfood::ReleaseExportVerifyInput {
            manifest_value: archive.manifest_value.as_ref(),
            member_refs: &archive.member_refs,
            archive_diagnostics: &archive.diagnostics,
        })?;
    super::super::io::write_file(&receipt_out, &molten::preserves_rail::to_text(&receipt.value)?)?;
    println!(
        "dogfood release-export-verify decision={} receipt={} manifest={} promotion-summary={}",
        receipt.decision, receipt.receipt_ref, receipt.manifest_ref, receipt.promotion_summary_ref
    );
    Ok(())
}
