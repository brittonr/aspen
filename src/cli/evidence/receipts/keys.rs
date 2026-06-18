pub(super) fn run(command: super::ReceiptKeyCommand) -> molten::error::Result<()> {
    match command {
        super::ReceiptKeyCommand::Import {
            ledger,
            key_id,
            signer,
            trust_root,
            key,
            receipt_out,
        } => {
            let key_value = molten::evidence::signed_receipt_key_value(&molten::evidence::SignedReceiptKeyInput {
                key_id: &key_id,
                signer: &signer,
                trust_root: &trust_root,
                key: &key,
                generation: 1,
                predecessor_ref: None,
            })?;
            let imported = molten::ledger::import_artifact(&ledger, &key_value)?;
            super::io::emit_named_receipt(
                receipt_out.as_ref(),
                "receipts key import receipt",
                &imported.receipt_value,
            )?;
            println!(
                "receipts key import ok key={} key-id={} signer={} trust-root={} status=current evidence-only=pass",
                imported.artifact_ref, key_id, signer, trust_root
            );
            Ok(())
        }
        super::ReceiptKeyCommand::List { ledger } => {
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
        super::ReceiptKeyCommand::Show { key_ref, ledger } => {
            let value = molten::ledger::read_artifact(&ledger, &key_ref)?;
            println!("{}", super::keyring::summary(&value)?);
            Ok(())
        }
        super::ReceiptKeyCommand::Revoke {
            key_ref,
            ledger,
            reason,
            receipt_out,
        } => {
            let keyring = super::load_signed_receipt_keyring(&ledger)?;
            if super::keyring::revocation(&keyring, &key_ref).is_some() {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "signed receipt key {key_ref} is already revoked"
                )));
            }
            let key_value = molten::ledger::read_artifact(&ledger, &key_ref)?;
            let key = molten::evidence::parse_signed_receipt_key(&key_value)?;
            let revocation_value = molten::evidence::signed_receipt_key_revocation_value(
                &molten::evidence::SignedReceiptKeyRevocationInput {
                    key: &key,
                    reason: &reason,
                    superseded_by: None,
                },
            )?;
            let imported = molten::ledger::import_artifact(&ledger, &revocation_value)?;
            super::io::emit_named_receipt(
                receipt_out.as_ref(),
                "receipts key revoke receipt",
                &imported.receipt_value,
            )?;
            println!(
                "receipts key revoke ok revocation={} key={} key-id={} signer={} reason={} evidence-only=pass",
                imported.artifact_ref, key.key_ref, key.key_id, key.signer, reason
            );
            Ok(())
        }
        super::ReceiptKeyCommand::Rotate {
            old_key_ref,
            ledger,
            new_key_id,
            new_key,
            reason,
            receipt_out,
        } => {
            let keyring = super::load_signed_receipt_keyring(&ledger)?;
            if super::keyring::revocation(&keyring, &old_key_ref).is_some() {
                return Err(molten::error::MoltenError::invalid_harness(format!(
                    "signed receipt key {old_key_ref} is already revoked and cannot be rotated"
                )));
            }
            let old_value = molten::ledger::read_artifact(&ledger, &old_key_ref)?;
            let old_key = molten::evidence::parse_signed_receipt_key(&old_value)?;
            let generation = old_key
                .generation
                .checked_add(1)
                .ok_or_else(|| molten::error::MoltenError::invalid_harness("signed receipt key generation overflow"))?;
            let new_value = molten::evidence::signed_receipt_key_value(&molten::evidence::SignedReceiptKeyInput {
                key_id: &new_key_id,
                signer: &old_key.signer,
                trust_root: &old_key.trust_root,
                key: &new_key,
                generation,
                predecessor_ref: Some(&old_key.key_ref),
            })?;
            let new_import = molten::ledger::import_artifact(&ledger, &new_value)?;
            let revocation_value = molten::evidence::signed_receipt_key_revocation_value(
                &molten::evidence::SignedReceiptKeyRevocationInput {
                    key: &old_key,
                    reason: &reason,
                    superseded_by: Some(&new_import.artifact_ref),
                },
            )?;
            let revocation_import = molten::ledger::import_artifact(&ledger, &revocation_value)?;
            super::io::emit_named_receipt(
                receipt_out.as_ref(),
                "receipts key rotate receipt",
                &revocation_import.receipt_value,
            )?;
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
