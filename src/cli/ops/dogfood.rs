use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::operator_dogfood;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

use crate::cli_receipts;

const DOGFOOD_SIGNED_MEMBER_LIMIT: usize = 64;
const _: () = assert!(DOGFOOD_SIGNED_MEMBER_LIMIT <= 100_000);

#[derive(Debug, Subcommand)]
pub(crate) enum DogfoodCommand {
    LocalNode {
        #[arg(long)]
        state_root: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        release_gate_out: Option<PathBuf>,
        #[arg(long)]
        replay_verify_out: Option<PathBuf>,
        #[arg(long)]
        replay_index_out: Option<PathBuf>,
    },
    NixReleaseExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    NixReleaseVerify {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        evidence: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    ReleaseBundleExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    ReleaseBundleVerify {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
        #[arg(long = "signed-member")]
        signed_members: Vec<PathBuf>,
        #[arg(long)]
        require_signed_members: bool,
        #[arg(long, default_value = "release-evidence")]
        signed_purpose: String,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long, default_value = "local-release-key")]
        signed_key: String,
        #[arg(long)]
        signed_key_ledger: Option<PathBuf>,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleasePromote {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        bundle_verify: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
        #[arg(long)]
        signed_key_ledger: PathBuf,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
        #[arg(long)]
        source_evidence: String,
        #[arg(long)]
        octet_evidence: String,
        #[arg(long)]
        cairn_evidence: String,
    },
    ReleasePromotionSummary {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        signed_key_ledger: Option<PathBuf>,
        #[arg(long, default_value = "local-release-trust-root")]
        signed_trust_root: String,
        #[arg(long)]
        signed_key_ref: Option<String>,
        #[arg(long)]
        signed_key_id: Option<String>,
        #[arg(long)]
        signed_signer: Option<String>,
    },
    ReleaseExport {
        #[arg(long)]
        output_path: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        manifest_out: PathBuf,
    },
    ReleaseExportVerify {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        receipt_out: PathBuf,
    },
    Show {
        artifact: PathBuf,
    },
}

struct DogfoodCliBoundedItems<T> {
    values: Vec<T>,
    maximum: usize,
    label: &'static str,
}

impl<T> DogfoodCliBoundedItems<T> {
    fn new(maximum: usize, label: &'static str) -> Self {
        Self {
            values: Vec::new(),
            maximum,
            label,
        }
    }

    fn push(&mut self, value: T) -> Result<()> {
        if self.values.len() >= self.maximum {
            return Err(MoltenError::invalid_harness(format!("{} count exceeds {}", self.label, self.maximum)));
        }
        self.values.push(value);
        Ok(())
    }

    fn into_vec(self) -> Vec<T> {
        self.values
    }
}

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
            let signed_member_values = read_preserves_files(&signed_members)?;
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
            write_release_export_archive(&output_path, &out, &manifest)?;
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
            let archive = read_release_export_archive(&bundle)?;
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

fn write_release_export_archive(
    output_path: &Path,
    archive_path: &Path,
    manifest: &operator_dogfood::ReleaseExportManifest,
) -> Result<()> {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    let archive_file = fs::File::create(archive_path).map_err(MoltenError::from)?;
    let encoder = zstd::stream::write::Encoder::new(archive_file, 19).map_err(MoltenError::from)?;
    let mut builder = tar::Builder::new(encoder);
    append_release_export_bytes(
        &mut builder,
        "release-export-manifest.preserves",
        to_text(&manifest.value)?.as_bytes(),
    )?;
    for (name, expected_ref) in &manifest.member_refs {
        let bytes = fs::read(output_path.join(name)).map_err(MoltenError::from)?;
        let actual_ref = operator_dogfood::release_export_file_ref(name, &bytes);
        if actual_ref != *expected_ref {
            return Err(MoltenError::invalid_harness(format!(
                "release export member {name} ref changed before archive write: manifest={expected_ref} observed={actual_ref}"
            )));
        }
        append_release_export_bytes(&mut builder, name, &bytes)?;
    }
    let encoder = builder.into_inner().map_err(MoltenError::from)?;
    encoder.finish().map_err(MoltenError::from)?;
    Ok(())
}

fn append_release_export_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o444);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder.append_data(&mut header, name, std::io::Cursor::new(bytes)).map_err(MoltenError::from)
}

#[derive(Debug)]
struct ReleaseExportArchiveRead {
    manifest_value: Option<preserves::IOValue>,
    member_refs: Vec<(String, String)>,
    diagnostics: Vec<String>,
}

fn read_release_export_archive(path: &Path) -> Result<ReleaseExportArchiveRead> {
    let archive_file = fs::File::open(path).map_err(MoltenError::from)?;
    let decoder = zstd::stream::read::Decoder::new(archive_file).map_err(MoltenError::from)?;
    let mut archive = tar::Archive::new(decoder);
    let mut manifest_value = None;
    let mut seen_names = Vec::with_capacity(operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut member_refs = Vec::with_capacity(operator_dogfood::release_export_member_names().len().saturating_add(16));
    let mut diagnostics = Vec::with_capacity(8);
    let entries = archive.entries().map_err(MoltenError::from)?;
    for entry in entries {
        let mut entry = entry.map_err(MoltenError::from)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry.path().map_err(MoltenError::from)?.to_string_lossy().replace('\\', "/");
        if seen_names.iter().any(|seen| seen == &name) {
            diagnostics.push(format!("duplicate release export archive member: {name}"));
        }
        seen_names.push(name.clone());
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).map_err(MoltenError::from)?;
        if name == "release-export-manifest.preserves" {
            if manifest_value.is_some() {
                diagnostics.push("duplicate release export manifest member".to_string());
            }
            let text = String::from_utf8(bytes).map_err(|error| {
                MoltenError::invalid_harness(format!("release export manifest is not UTF-8: {error}"))
            })?;
            manifest_value = Some(parse_text(&text)?);
        } else {
            member_refs.push((name.clone(), operator_dogfood::release_export_file_ref(&name, &bytes)));
        }
    }
    member_refs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(ReleaseExportArchiveRead {
        manifest_value,
        member_refs,
        diagnostics,
    })
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn read_preserves_files(paths: &[PathBuf]) -> Result<Vec<preserves::IOValue>> {
    let mut values = DogfoodCliBoundedItems::new(DOGFOOD_SIGNED_MEMBER_LIMIT, "dogfood signed members");
    for path in paths {
        values.push(read_preserves_file(path)?)?;
    }
    Ok(values.into_vec())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
