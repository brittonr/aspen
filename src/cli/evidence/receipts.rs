use std::fs;
use std::path::Path;
use std::path::PathBuf;

use molten::error::MoltenError;
use molten::error::Result;
use molten::evidence::SignReceiptInput;
use molten::evidence::SignedReceiptKeyInput;
use molten::evidence::SignedReceiptKeyRevocationInput;
use molten::evidence::VerifySignedReceiptKeyringPolicy;
use molten::evidence::VerifySignedReceiptPolicy;
use molten::evidence::parse_signed_receipt_key;
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

#[path = "receipts/command.rs"]
mod command;
#[path = "receipts/keyring.rs"]
mod keyring;

pub(crate) type ReceiptsCommand = command::Top;
pub(crate) type ReceiptCommand = command::Test;
type ReceiptKeyCommand = command::Key;
pub(crate) type SignedReceiptKeyring = keyring::Set;

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
                let is_revoked = keyring::revocation(&keyring, &key.key_ref).is_some();
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
            println!("{}", keyring::summary(&value)?);
            Ok(())
        }
        ReceiptKeyCommand::Revoke {
            key_ref,
            ledger,
            reason,
            receipt_out,
        } => {
            let keyring = load_signed_receipt_keyring(&ledger)?;
            if keyring::revocation(&keyring, &key_ref).is_some() {
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
            if keyring::revocation(&keyring, &old_key_ref).is_some() {
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
    keyring::ensure_selector_has_ledger(ledger, key_ref, key_id)
}

pub(crate) fn load_signed_receipt_keyring(ledger: &Path) -> Result<SignedReceiptKeyring> {
    keyring::load(ledger)
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
