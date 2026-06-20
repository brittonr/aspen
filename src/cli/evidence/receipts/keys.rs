type Command = super::ReceiptKeyCommand;
type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

struct ImportInput {
    ledger: FilePath,
    key_id: String,
    signer: String,
    trust_root: String,
    key: String,
    receipt_out: Option<FilePath>,
}

struct RevokeInput {
    key_ref: String,
    ledger: FilePath,
    reason: String,
    receipt_out: Option<FilePath>,
}

struct RotateInput {
    old_key_ref: String,
    ledger: FilePath,
    new_key_id: String,
    new_key: String,
    reason: String,
    receipt_out: Option<FilePath>,
}

pub(super) fn run(command: Command) -> Outcome<()> {
    match command {
        Command::Import {
            ledger,
            key_id,
            signer,
            trust_root,
            key,
            receipt_out,
        } => import(ImportInput {
            ledger,
            key_id,
            signer,
            trust_root,
            key,
            receipt_out,
        }),
        Command::List { ledger } => list(ledger),
        Command::Show { key_ref, ledger } => show(key_ref, ledger),
        Command::Revoke {
            key_ref,
            ledger,
            reason,
            receipt_out,
        } => revoke(RevokeInput {
            key_ref,
            ledger,
            reason,
            receipt_out,
        }),
        Command::Rotate {
            old_key_ref,
            ledger,
            new_key_id,
            new_key,
            reason,
            receipt_out,
        } => rotate(RotateInput {
            old_key_ref,
            ledger,
            new_key_id,
            new_key,
            reason,
            receipt_out,
        }),
    }
}

fn import(input: ImportInput) -> Outcome<()> {
    let key_value = molten::evidence::signed_receipt_key_value(&molten::evidence::SignedReceiptKeyInput {
        key_id: &input.key_id,
        signer: &input.signer,
        trust_root: &input.trust_root,
        key: &input.key,
        generation: 1,
        predecessor_ref: None,
    })?;
    let imported = molten::ledger::import_artifact(&input.ledger, &key_value)?;
    super::io::emit_named_receipt(input.receipt_out.as_ref(), "receipts key import receipt", &imported.receipt_value)?;
    println!(
        "receipts key import ok key={} key-id={} signer={} trust-root={} status=current evidence-only=pass",
        imported.artifact_ref, input.key_id, input.signer, input.trust_root
    );
    Ok(())
}

fn list(ledger: FilePath) -> Outcome<()> {
    let keyring = super::load_signed_receipt_keyring(&ledger)?;
    for key in &keyring.keys {
        let is_revoked = super::keyring::revocation(&keyring, &key.key_ref).is_some();
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

fn show(key_ref: String, ledger: FilePath) -> Outcome<()> {
    let value = molten::ledger::read_artifact(&ledger, &key_ref)?;
    println!("{}", super::keyring::summary(&value)?);
    Ok(())
}

fn revoke(input: RevokeInput) -> Outcome<()> {
    let keyring = super::load_signed_receipt_keyring(&input.ledger)?;
    if super::keyring::revocation(&keyring, &input.key_ref).is_some() {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "signed receipt key {} is already revoked",
            input.key_ref
        )));
    }
    let key_value = molten::ledger::read_artifact(&input.ledger, &input.key_ref)?;
    let key = molten::evidence::parse_signed_receipt_key(&key_value)?;
    let revocation_value =
        molten::evidence::signed_receipt_key_revocation_value(&molten::evidence::SignedReceiptKeyRevocationInput {
            key: &key,
            reason: &input.reason,
            superseded_by: None,
        })?;
    let imported = molten::ledger::import_artifact(&input.ledger, &revocation_value)?;
    super::io::emit_named_receipt(input.receipt_out.as_ref(), "receipts key revoke receipt", &imported.receipt_value)?;
    println!(
        "receipts key revoke ok revocation={} key={} key-id={} signer={} reason={} evidence-only=pass",
        imported.artifact_ref, key.key_ref, key.key_id, key.signer, input.reason
    );
    Ok(())
}

fn rotate(input: RotateInput) -> Outcome<()> {
    let keyring = super::load_signed_receipt_keyring(&input.ledger)?;
    if super::keyring::revocation(&keyring, &input.old_key_ref).is_some() {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "signed receipt key {} is already revoked and cannot be rotated",
            input.old_key_ref
        )));
    }
    let old_value = molten::ledger::read_artifact(&input.ledger, &input.old_key_ref)?;
    let old_key = molten::evidence::parse_signed_receipt_key(&old_value)?;
    let generation = old_key
        .generation
        .checked_add(1)
        .ok_or_else(|| molten::error::MoltenError::invalid_harness("signed receipt key generation overflow"))?;
    let new_value = molten::evidence::signed_receipt_key_value(&molten::evidence::SignedReceiptKeyInput {
        key_id: &input.new_key_id,
        signer: &old_key.signer,
        trust_root: &old_key.trust_root,
        key: &input.new_key,
        generation,
        predecessor_ref: Some(&old_key.key_ref),
    })?;
    let new_import = molten::ledger::import_artifact(&input.ledger, &new_value)?;
    let revocation_value =
        molten::evidence::signed_receipt_key_revocation_value(&molten::evidence::SignedReceiptKeyRevocationInput {
            key: &old_key,
            reason: &input.reason,
            superseded_by: Some(&new_import.artifact_ref),
        })?;
    let revocation_import = molten::ledger::import_artifact(&input.ledger, &revocation_value)?;
    super::io::emit_named_receipt(
        input.receipt_out.as_ref(),
        "receipts key rotate receipt",
        &revocation_import.receipt_value,
    )?;
    println!(
        "receipts key rotate ok old-key={} new-key={} new-key-id={} signer={} trust-root={} revocation={} evidence-only=pass",
        old_key.key_ref,
        new_import.artifact_ref,
        input.new_key_id,
        old_key.signer,
        old_key.trust_root,
        revocation_import.artifact_ref
    );
    Ok(())
}
