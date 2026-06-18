const ENTRY_LIMIT: usize = 4096;
const _: () = assert!(ENTRY_LIMIT <= 100_000);

pub(crate) struct Set {
    pub(crate) keys: Vec<molten::evidence::SignedReceiptKey>,
    pub(crate) revocations: Vec<molten::evidence::SignedReceiptKeyRevocation>,
}

pub(crate) fn ensure_selector_has_ledger(
    ledger: Option<&std::path::Path>,
    key_ref: Option<&str>,
    key_id: Option<&str>,
) -> molten::error::Result<()> {
    if ledger.is_none() && (key_ref.is_some() || key_id.is_some()) {
        Err(molten::error::MoltenError::invalid_harness(
            "signed receipt key selectors require --key-ledger or --signed-key-ledger",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn load(ledger: &std::path::Path) -> molten::error::Result<Set> {
    let mut keys = Vec::with_capacity(ENTRY_LIMIT);
    let mut revocations = Vec::with_capacity(ENTRY_LIMIT);
    for entry in molten::ledger::list_artifacts(ledger)? {
        match entry.artifact_kind.as_str() {
            "signed-receipt-key" => {
                ensure_entry_count(keys.len().saturating_add(1), "signed receipt key records")?;
                keys.push(molten::evidence::parse_signed_receipt_key(&molten::ledger::read_artifact(
                    ledger,
                    &entry.artifact_ref,
                )?)?);
            }
            "signed-receipt-key-revocation" => {
                ensure_entry_count(revocations.len().saturating_add(1), "signed receipt key revocation records")?;
                revocations.push(molten::evidence::parse_signed_receipt_key_revocation(
                    &molten::ledger::read_artifact(ledger, &entry.artifact_ref)?,
                )?);
            }
            _ => {}
        }
    }
    Ok(Set { keys, revocations })
}

fn ensure_entry_count(count: usize, label: &str) -> molten::error::Result<()> {
    if count > ENTRY_LIMIT {
        return Err(molten::error::MoltenError::invalid_harness(format!(
            "{label} count {count} exceeds {ENTRY_LIMIT}"
        )));
    }
    Ok(())
}

pub(crate) fn revocation<'a>(
    keyring: &'a Set,
    key_ref: &str,
) -> Option<&'a molten::evidence::SignedReceiptKeyRevocation> {
    keyring.revocations.iter().find(|revocation| revocation.key_ref == key_ref)
}

pub(crate) fn summary(value: &preserves::IOValue) -> molten::error::Result<String> {
    match molten::ledger::artifact_kind(value) {
        "signed-receipt-key" => {
            let key = molten::evidence::parse_signed_receipt_key(value)?;
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
            let revocation = molten::evidence::parse_signed_receipt_key_revocation(value)?;
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
        kind => Err(molten::error::MoltenError::invalid_harness(format!(
            "unsupported signed receipt keyring artifact kind {kind}; expected signed-receipt-key or signed-receipt-key-revocation"
        ))),
    }
}
