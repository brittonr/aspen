use std::fs;
use std::path::Path;

use molten::error::MoltenError;
use molten::error::Result;
use molten::operator_dogfood;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

use crate::cli_receipts;

#[path = "dogfood/archive.rs"]
mod archive;
#[path = "dogfood/command.rs"]
mod command;
#[path = "dogfood/signed.rs"]
mod signed;

pub(crate) type DogfoodCommand = command::Command;

pub(crate) fn run_dogfood_command(command: DogfoodCommand) -> Result<()> {
    match command {
        DogfoodCommand::LocalNode {
            state_root,
            out,
            release_gate_out,
            replay_verify_out,
            replay_index_out,
        } => {
            let run = operator_dogfood::run_local_node_dogfood(&operator_dogfood::LocalNodeDogfoodInput {
                state_root: &state_root,
            })?;
            write_file(&out, &to_text(&run.report_value)?)?;
            if let (Some(path), Some(value)) = (release_gate_out.as_ref(), run.release_gate_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let (Some(path), Some(value)) = (replay_verify_out.as_ref(), run.replay_verify_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            if let (Some(path), Some(value)) = (replay_index_out.as_ref(), run.replay_index_value.as_ref()) {
                write_file(path, &to_text(value)?)?;
            }
            println!(
                "dogfood local-node decision={} report={} release-gate={}",
                run.decision,
                run.report_ref,
                run.release_gate_ref.as_deref().unwrap_or("none")
            );
            Ok(())
        }
        DogfoodCommand::NixReleaseExport { output_path, out } => {
            let evidence =
                operator_dogfood::nix_dogfood_release_evidence_value(&operator_dogfood::NixDogfoodEvidenceInput {
                    output_path: &output_path,
                })?;
            let parsed = operator_dogfood::parse_nix_dogfood_evidence(&evidence)?;
            write_file(&out, &to_text(&evidence)?)?;
            println!(
                "dogfood nix-release-export evidence={} report={} release-gate={}",
                parsed.evidence_ref, parsed.report_ref, parsed.release_gate_ref
            );
            Ok(())
        }
        DogfoodCommand::NixReleaseVerify {
            output_path,
            evidence,
            receipt_out,
        } => {
            let evidence_value = read_preserves_file(&evidence)?;
            let receipt = operator_dogfood::verify_nix_dogfood_evidence(&operator_dogfood::NixDogfoodVerifyInput {
                output_path: &output_path,
                evidence_value: &evidence_value,
            })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood nix-release-verify decision={} receipt={} evidence={}",
                receipt.decision, receipt.receipt_ref, receipt.evidence_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleaseBundleExport { output_path, out } => {
            let bundle =
                operator_dogfood::release_evidence_bundle_value(&operator_dogfood::ReleaseEvidenceBundleInput {
                    output_path: &output_path,
                })?;
            let parsed = operator_dogfood::parse_release_evidence_bundle(&bundle)?;
            write_file(&out, &to_text(&bundle)?)?;
            println!(
                "dogfood release-bundle-export bundle={} report={} release-gate={} nix-verify={}",
                parsed.bundle_ref, parsed.report_ref, parsed.release_gate_ref, parsed.nix_verify_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleaseBundleVerify {
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
        } => {
            let bundle_value = read_preserves_file(&bundle)?;
            let signed_member_values = signed::read_preserves_files(&signed_members)?;
            cli_receipts::ensure_keyring_selector_has_ledger(
                signed_key_ledger.as_deref(),
                signed_key_ref.as_deref(),
                signed_key_id.as_deref(),
            )?;
            let keyring = match signed_key_ledger.as_ref() {
                Some(ledger) => cli_receipts::load_signed_receipt_keyring(ledger)?,
                None => cli_receipts::SignedReceiptKeyring {
                    keys: Vec::new(),
                    revocations: Vec::new(),
                },
            };
            let receipt = operator_dogfood::verify_release_evidence_bundle(
                &operator_dogfood::ReleaseEvidenceBundleVerifyInput {
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
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood release-bundle-verify decision={} receipt={} bundle={}",
                receipt.decision, receipt.receipt_ref, receipt.bundle_ref
            );
            Ok(())
        }
        DogfoodCommand::ReleasePromote {
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
        } => {
            let bundle_verify_value = read_preserves_file(&bundle_verify)?;
            let keyring = cli_receipts::load_signed_receipt_keyring(&signed_key_ledger)?;
            let receipt =
                operator_dogfood::release_promotion_gate_receipt_value(&operator_dogfood::ReleasePromotionGateInput {
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
                })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
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
        DogfoodCommand::ReleasePromotionSummary {
            output_path,
            out,
            signed_key_ledger,
            signed_trust_root,
            signed_key_ref,
            signed_key_id,
            signed_signer,
        } => {
            let key_ledger = signed_key_ledger.unwrap_or_else(|| output_path.join("signed-keyring"));
            let keyring = cli_receipts::load_signed_receipt_keyring(&key_ledger)?;
            let summary =
                operator_dogfood::release_promotion_summary_value(&operator_dogfood::ReleasePromotionSummaryInput {
                    output_path: &output_path,
                    signed_keys: &keyring.keys,
                    signed_key_revocations: &keyring.revocations,
                    signed_trust_root: &signed_trust_root,
                    signed_signer: signed_signer.as_deref(),
                    signed_key_ref: signed_key_ref.as_deref(),
                    signed_key_id: signed_key_id.as_deref(),
                })?;
            write_file(&out, &to_text(&summary.value)?)?;
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
        DogfoodCommand::ReleaseExport {
            output_path,
            out,
            manifest_out,
        } => {
            let manifest =
                operator_dogfood::release_export_manifest_value(&operator_dogfood::ReleaseExportManifestInput {
                    output_path: &output_path,
                })?;
            write_file(&manifest_out, &to_text(&manifest.value)?)?;
            archive::write(&output_path, &out, &manifest)?;
            println!(
                "dogfood release-export manifest={} promotion-summary={} members={} archive={}",
                manifest.manifest_ref,
                manifest.promotion_summary_ref,
                manifest.member_refs.len(),
                out.display()
            );
            Ok(())
        }
        DogfoodCommand::ReleaseExportVerify { bundle, receipt_out } => {
            let archive = archive::read(&bundle)?;
            let receipt = operator_dogfood::verify_release_export(&operator_dogfood::ReleaseExportVerifyInput {
                manifest_value: archive.manifest_value.as_ref(),
                member_refs: &archive.member_refs,
                archive_diagnostics: &archive.diagnostics,
            })?;
            write_file(&receipt_out, &to_text(&receipt.value)?)?;
            println!(
                "dogfood release-export-verify decision={} receipt={} manifest={} promotion-summary={}",
                receipt.decision, receipt.receipt_ref, receipt.manifest_ref, receipt.promotion_summary_ref
            );
            Ok(())
        }
        DogfoodCommand::Show { artifact } => {
            let value = read_preserves_file(&artifact)?;
            println!("{}", operator_dogfood::operator_dogfood_summary(&value)?);
            Ok(())
        }
    }
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
