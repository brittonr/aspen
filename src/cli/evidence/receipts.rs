use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::error::MoltenError;
use molten::error::Result;
use molten::evidence::PASS_EVIDENCE_PURPOSE;
use molten::evidence::SignReceiptInput;
use molten::evidence::SignedReceiptKey;
use molten::evidence::SignedReceiptKeyInput;
use molten::evidence::SignedReceiptKeyRevocation;
use molten::evidence::SignedReceiptKeyRevocationInput;
use molten::evidence::VerifySignedReceiptKeyringPolicy;
use molten::evidence::VerifySignedReceiptPolicy;
use molten::evidence::parse_signed_receipt_key;
use molten::evidence::parse_signed_receipt_key_revocation;
use molten::evidence::sign_receipt;
use molten::evidence::signed_receipt_key_revocation_value;
use molten::evidence::signed_receipt_key_value;
use molten::evidence::signed_receipt_summary;
use molten::evidence::verify_signed_receipt_with_keyring_policy;
use molten::evidence::verify_signed_receipt_with_policy;
use molten::ledger;
use molten::operator_dogfood;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::parse_text;
use molten::preserves_rail::to_text;

const SIGNED_KEYRING_CLI_ENTRY_LIMIT: usize = 4096;
const _: () = assert!(SIGNED_KEYRING_CLI_ENTRY_LIMIT <= 100_000);

#[derive(Debug, Subcommand)]
pub(crate) enum ReceiptsCommand {
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Show {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Validate {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Export {
        receipt_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Key {
        #[command(subcommand)]
        command: ReceiptKeyCommand,
    },
    Sign {
        receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    VerifySigned {
        signed_receipt: PathBuf,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReceiptCommand {
    Sign {
        receipt: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "local-signer")]
        signer: String,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
    },
    Verify {
        signed_receipt: PathBuf,
        #[arg(long, default_value = PASS_EVIDENCE_PURPOSE)]
        purpose: String,
        #[arg(long, default_value = "local-trust-root")]
        trust_root: String,
        #[arg(long, default_value = "local-dev-key")]
        key: String,
        #[arg(long)]
        key_ledger: Option<PathBuf>,
        #[arg(long)]
        key_ref: Option<String>,
        #[arg(long)]
        key_id: Option<String>,
        #[arg(long)]
        signer: Option<String>,
        #[arg(long)]
        subject_ref: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ReceiptKeyCommand {
    Import {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        key_id: String,
        #[arg(long)]
        signer: String,
        #[arg(long)]
        trust_root: String,
        #[arg(long)]
        key: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        ledger: PathBuf,
    },
    Show {
        key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
    },
    Revoke {
        key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long, default_value = "operator-revoked")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Rotate {
        old_key_ref: String,
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        new_key_id: String,
        #[arg(long)]
        new_key: String,
        #[arg(long, default_value = "rotated")]
        reason: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) struct SignedReceiptKeyring {
    pub(crate) keys: Vec<SignedReceiptKey>,
    pub(crate) revocations: Vec<SignedReceiptKeyRevocation>,
}

pub(crate) fn run_receipts_command(command: ReceiptsCommand) -> Result<()> {
    match command {
        ReceiptsCommand::List { ledger } => {
            for entry in ledger::list_artifacts(&ledger)? {
                if is_operator_receipt_kind(&entry.artifact_kind) {
                    println!("{} {}", entry.artifact_ref, entry.artifact_kind);
                }
            }
            Ok(())
        }
        ReceiptsCommand::Show { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            let summary = validate_operator_receipt_value(&value)?;
            println!("{summary}");
            Ok(())
        }
        ReceiptsCommand::Validate { receipt_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            let summary = validate_operator_receipt_value(&value)?;
            println!(
                "receipts validate ok artifact={} kind={} summary={}",
                receipt_ref,
                ledger::artifact_kind(&value),
                summary
            );
            Ok(())
        }
        ReceiptsCommand::Export {
            receipt_ref,
            ledger,
            out,
            receipt_out,
        } => {
            let value = ledger::read_artifact(&ledger, &receipt_ref)?;
            validate_operator_receipt_value(&value)?;
            let exported = ledger::export_artifact(&ledger, &receipt_ref, &out)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts export receipt", &exported.receipt_value)?;
            println!(
                "receipts export ok artifact={} kind={} out={} redaction=pass logs=auxiliary",
                exported.artifact_ref,
                exported.artifact_kind,
                out.display()
            );
            Ok(())
        }
        ReceiptsCommand::Key { command } => run_receipt_key_command(command),
        ReceiptsCommand::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => {
            let receipt_value = read_preserves_file(&receipt)?;
            let signed = sign_receipt(&SignReceiptInput {
                receipt: &receipt_value,
                signer: &signer,
                purpose: &purpose,
                trust_root: &trust_root,
                key: &key,
                parents: &parents,
            })?;
            let signed_ref = canonical_hash(&signed)?;
            let subject_ref = canonical_hash(&receipt_value)?;
            write_file(&out, &to_text(&signed)?)?;
            println!(
                "receipts sign ok signed={} subject={} signer={} purpose={} out={} evidence-only=pass",
                signed_ref,
                subject_ref,
                signer,
                purpose,
                out.display()
            );
            Ok(())
        }
        ReceiptsCommand::VerifySigned {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => {
            let signed_value = read_preserves_file(&signed_receipt)?;
            ensure_keyring_selector_has_ledger(key_ledger.as_deref(), key_ref.as_deref(), key_id.as_deref())?;
            if let Some(ledger) = key_ledger {
                let keyring = load_signed_receipt_keyring(&ledger)?;
                let verified =
                    verify_signed_receipt_with_keyring_policy(&signed_value, &VerifySignedReceiptKeyringPolicy {
                        required_purpose: &purpose,
                        trust_root: &trust_root,
                        expected_signer: signer.as_deref(),
                        expected_subject_ref: subject_ref.as_deref(),
                        required_key_ref: key_ref.as_deref(),
                        required_key_id: key_id.as_deref(),
                        keys: &keyring.keys,
                        revocations: &keyring.revocations,
                    })?;
                println!(
                    "receipts verify-signed ok envelope={} subject={} signer={} purpose={} key={} key-id={} keyring=current evidence-only=pass",
                    verified.receipt.envelope_ref,
                    verified.receipt.subject_ref,
                    verified.receipt.signer,
                    verified.receipt.purpose,
                    verified.key_ref,
                    verified.key_id
                );
            } else {
                let verified = verify_signed_receipt_with_policy(&signed_value, &VerifySignedReceiptPolicy {
                    required_purpose: &purpose,
                    trust_root: &trust_root,
                    key: &key,
                    expected_signer: signer.as_deref(),
                    expected_subject_ref: subject_ref.as_deref(),
                })?;
                println!(
                    "receipts verify-signed ok envelope={} subject={} signer={} purpose={} evidence-only=pass",
                    verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
                );
            }
            Ok(())
        }
    }
}

pub(crate) fn run_receipt_command(command: ReceiptCommand) -> Result<()> {
    match command {
        ReceiptCommand::Sign {
            receipt,
            out,
            signer,
            purpose,
            trust_root,
            key,
            parents,
        } => {
            let receipt_value = read_preserves_file(&receipt)?;
            let signed = sign_receipt(&SignReceiptInput {
                receipt: &receipt_value,
                signer: &signer,
                purpose: &purpose,
                trust_root: &trust_root,
                key: &key,
                parents: &parents,
            })?;
            write_file(&out, &to_text(&signed)?)?;
            println!("signed receipt written to {}", out.display());
            Ok(())
        }
        ReceiptCommand::Verify {
            signed_receipt,
            purpose,
            trust_root,
            key,
            key_ledger,
            key_ref,
            key_id,
            signer,
            subject_ref,
        } => {
            let signed_value = read_preserves_file(&signed_receipt)?;
            ensure_keyring_selector_has_ledger(key_ledger.as_deref(), key_ref.as_deref(), key_id.as_deref())?;
            if let Some(ledger) = key_ledger {
                let keyring = load_signed_receipt_keyring(&ledger)?;
                let verified =
                    verify_signed_receipt_with_keyring_policy(&signed_value, &VerifySignedReceiptKeyringPolicy {
                        required_purpose: &purpose,
                        trust_root: &trust_root,
                        expected_signer: signer.as_deref(),
                        expected_subject_ref: subject_ref.as_deref(),
                        required_key_ref: key_ref.as_deref(),
                        required_key_id: key_id.as_deref(),
                        keys: &keyring.keys,
                        revocations: &keyring.revocations,
                    })?;
                println!(
                    "signed receipt verify ok envelope={} subject={} signer={} purpose={} key={} key-id={}",
                    verified.receipt.envelope_ref,
                    verified.receipt.subject_ref,
                    verified.receipt.signer,
                    verified.receipt.purpose,
                    verified.key_ref,
                    verified.key_id
                );
            } else {
                let verified = verify_signed_receipt_with_policy(&signed_value, &VerifySignedReceiptPolicy {
                    required_purpose: &purpose,
                    trust_root: &trust_root,
                    key: &key,
                    expected_signer: signer.as_deref(),
                    expected_subject_ref: subject_ref.as_deref(),
                })?;
                println!(
                    "signed receipt verify ok envelope={} subject={} signer={} purpose={}",
                    verified.envelope_ref, verified.subject_ref, verified.signer, verified.purpose
                );
            }
            Ok(())
        }
    }
}

fn run_receipt_key_command(command: ReceiptKeyCommand) -> Result<()> {
    match command {
        ReceiptKeyCommand::Import {
            ledger,
            key_id,
            signer,
            trust_root,
            key,
            receipt_out,
        } => {
            let key_value = signed_receipt_key_value(&SignedReceiptKeyInput {
                key_id: &key_id,
                signer: &signer,
                trust_root: &trust_root,
                key: &key,
                generation: 1,
                predecessor_ref: None,
            })?;
            let imported = ledger::import_artifact(&ledger, &key_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key import receipt", &imported.receipt_value)?;
            println!(
                "receipts key import ok key={} key-id={} signer={} trust-root={} status=current evidence-only=pass",
                imported.artifact_ref, key_id, signer, trust_root
            );
            Ok(())
        }
        ReceiptKeyCommand::List { ledger } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            for key in &keyring.keys {
                let is_revoked = signed_key_revocation(&keyring, &key.key_ref).is_some();
                println!(
                    "{} signed-receipt-key key-id={} signer={} trust-root={} status={} generation={} revoked={} predecessor={}",
                    key.key_ref,
                    key.key_id,
                    key.signer,
                    key.trust_root,
                    key.status,
                    key.generation,
                    is_revoked,
                    key.predecessor_ref.as_deref().unwrap_or("none")
                );
            }
            for revocation in &keyring.revocations {
                println!(
                    "{} signed-receipt-key-revocation key={} key-id={} signer={} trust-root={} reason={} superseded-by={}",
                    revocation.revocation_ref,
                    revocation.key_ref,
                    revocation.key_id,
                    revocation.signer,
                    revocation.trust_root,
                    revocation.reason,
                    revocation.superseded_by.as_deref().unwrap_or("none")
                );
            }
            Ok(())
        }
        ReceiptKeyCommand::Show { key_ref, ledger } => {
            let value = ledger::read_artifact(&ledger, &key_ref)?;
            println!("{}", signed_key_summary(&value)?);
            Ok(())
        }
        ReceiptKeyCommand::Revoke {
            key_ref,
            ledger,
            reason,
            receipt_out,
        } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            if signed_key_revocation(&keyring, &key_ref).is_some() {
                return Err(MoltenError::invalid_harness(format!("signed receipt key {key_ref} is already revoked")));
            }
            let key_value = ledger::read_artifact(&ledger, &key_ref)?;
            let key = parse_signed_receipt_key(&key_value)?;
            let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
                key: &key,
                reason: &reason,
                superseded_by: None,
            })?;
            let imported = ledger::import_artifact(&ledger, &revocation_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key revoke receipt", &imported.receipt_value)?;
            println!(
                "receipts key revoke ok revocation={} key={} key-id={} signer={} reason={} evidence-only=pass",
                imported.artifact_ref, key.key_ref, key.key_id, key.signer, reason
            );
            Ok(())
        }
        ReceiptKeyCommand::Rotate {
            old_key_ref,
            ledger,
            new_key_id,
            new_key,
            reason,
            receipt_out,
        } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            if signed_key_revocation(&keyring, &old_key_ref).is_some() {
                return Err(MoltenError::invalid_harness(format!(
                    "signed receipt key {old_key_ref} is already revoked and cannot be rotated"
                )));
            }
            let old_value = ledger::read_artifact(&ledger, &old_key_ref)?;
            let old_key = parse_signed_receipt_key(&old_value)?;
            let generation = old_key
                .generation
                .checked_add(1)
                .ok_or_else(|| MoltenError::invalid_harness("signed receipt key generation overflow"))?;
            let new_value = signed_receipt_key_value(&SignedReceiptKeyInput {
                key_id: &new_key_id,
                signer: &old_key.signer,
                trust_root: &old_key.trust_root,
                key: &new_key,
                generation,
                predecessor_ref: Some(&old_key.key_ref),
            })?;
            let new_import = ledger::import_artifact(&ledger, &new_value)?;
            let revocation_value = signed_receipt_key_revocation_value(&SignedReceiptKeyRevocationInput {
                key: &old_key,
                reason: &reason,
                superseded_by: Some(&new_import.artifact_ref),
            })?;
            let revocation_import = ledger::import_artifact(&ledger, &revocation_value)?;
            emit_named_receipt(receipt_out.as_ref(), "receipts key rotate receipt", &revocation_import.receipt_value)?;
            println!(
                "receipts key rotate ok old-key={} new-key={} new-key-id={} signer={} trust-root={} revocation={} evidence-only=pass",
                old_key.key_ref,
                new_import.artifact_ref,
                new_key_id,
                old_key.signer,
                old_key.trust_root,
                revocation_import.artifact_ref
            );
            Ok(())
        }
    }
}

pub(crate) fn ensure_keyring_selector_has_ledger(
    ledger: Option<&Path>,
    key_ref: Option<&str>,
    key_id: Option<&str>,
) -> Result<()> {
    if ledger.is_none() && (key_ref.is_some() || key_id.is_some()) {
        Err(MoltenError::invalid_harness(
            "signed receipt key selectors require --key-ledger or --signed-key-ledger",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn load_signed_receipt_keyring(ledger: &Path) -> Result<SignedReceiptKeyring> {
    let mut keys = Vec::with_capacity(SIGNED_KEYRING_CLI_ENTRY_LIMIT);
    let mut revocations = Vec::with_capacity(SIGNED_KEYRING_CLI_ENTRY_LIMIT);
    for entry in ledger::list_artifacts(ledger)? {
        match entry.artifact_kind.as_str() {
            "signed-receipt-key" => {
                ensure_signed_keyring_entry_count(keys.len().saturating_add(1), "signed receipt key records")?;
                keys.push(parse_signed_receipt_key(&ledger::read_artifact(ledger, &entry.artifact_ref)?)?);
            }
            "signed-receipt-key-revocation" => {
                ensure_signed_keyring_entry_count(
                    revocations.len().saturating_add(1),
                    "signed receipt key revocation records",
                )?;
                revocations
                    .push(parse_signed_receipt_key_revocation(&ledger::read_artifact(ledger, &entry.artifact_ref)?)?);
            }
            _ => {}
        }
    }
    Ok(SignedReceiptKeyring { keys, revocations })
}

fn ensure_signed_keyring_entry_count(count: usize, label: &str) -> Result<()> {
    if count > SIGNED_KEYRING_CLI_ENTRY_LIMIT {
        return Err(MoltenError::invalid_harness(format!(
            "{label} count {count} exceeds {SIGNED_KEYRING_CLI_ENTRY_LIMIT}"
        )));
    }
    Ok(())
}

fn signed_key_revocation<'a>(
    keyring: &'a SignedReceiptKeyring,
    key_ref: &str,
) -> Option<&'a SignedReceiptKeyRevocation> {
    keyring.revocations.iter().find(|revocation| revocation.key_ref == key_ref)
}

fn signed_key_summary(value: &preserves::IOValue) -> Result<String> {
    match ledger::artifact_kind(value) {
        "signed-receipt-key" => {
            let key = parse_signed_receipt_key(value)?;
            Ok(format!(
                "signed receipt key {}\nkey-id={}\nsigner={}\ntrust-root={}\nstatus={}\ngeneration={}\npredecessor={}\nevidence-only=pass",
                key.key_ref,
                key.key_id,
                key.signer,
                key.trust_root,
                key.status,
                key.generation,
                key.predecessor_ref.as_deref().unwrap_or("none")
            ))
        }
        "signed-receipt-key-revocation" => {
            let revocation = parse_signed_receipt_key_revocation(value)?;
            Ok(format!(
                "signed receipt key revocation {}\nkey={}\nkey-id={}\nsigner={}\ntrust-root={}\nreason={}\nsuperseded-by={}\nevidence-only=pass",
                revocation.revocation_ref,
                revocation.key_ref,
                revocation.key_id,
                revocation.signer,
                revocation.trust_root,
                revocation.reason,
                revocation.superseded_by.as_deref().unwrap_or("none")
            ))
        }
        kind => Err(MoltenError::invalid_harness(format!(
            "unsupported signed receipt keyring artifact kind {kind}; expected signed-receipt-key or signed-receipt-key-revocation"
        ))),
    }
}

fn validate_operator_receipt_value(value: &preserves::IOValue) -> Result<String> {
    match ledger::artifact_kind(value) {
        "dogfood-report"
        | "operator-workflow"
        | "operator-checkpoint"
        | "release-gate-receipt"
        | "nix-dogfood-release-evidence"
        | "nix-dogfood-release-verify-receipt"
        | "release-evidence-bundle"
        | "release-evidence-bundle-verify-receipt"
        | "release-promotion-gate-receipt" => operator_dogfood::operator_dogfood_summary(value),
        "signed-receipt" => signed_receipt_summary(value),
        "operator-step" => {
            let step = operator_dogfood::parse_operator_step(value)?;
            Ok(format!(
                "operator step ref={} name={} decision={} receipt={} (summary is non-normative)",
                step.step_ref,
                step.name,
                step.decision,
                step.receipt_ref.as_deref().unwrap_or("none")
            ))
        }
        kind => Err(MoltenError::invalid_harness(format!(
            "unsupported operator receipt kind {kind}; expected dogfood/operator receipt artifact"
        ))),
    }
}

fn is_operator_receipt_kind(kind: &str) -> bool {
    matches!(
        kind,
        "dogfood-report"
            | "operator-workflow"
            | "operator-step"
            | "operator-checkpoint"
            | "release-gate-receipt"
            | "nix-dogfood-release-evidence"
            | "nix-dogfood-release-verify-receipt"
            | "release-evidence-bundle"
            | "release-evidence-bundle-verify-receipt"
            | "release-promotion-gate-receipt"
            | "signed-receipt"
    )
}

fn read_preserves_file(path: &Path) -> Result<preserves::IOValue> {
    let text = fs::read_to_string(path).map_err(MoltenError::from)?;
    parse_text(&text)
}

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
