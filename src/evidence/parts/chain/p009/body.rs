
pub fn append_chain_link(root: &Path, value: &IoValue) -> Result<ChainAppend> {
    let link = parse_chain_link(value)?;
    let index = build_chain_index(root)?;
    crate::ledger::read_artifact(root, &link.payload.artifact_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "chain link payload {} is unavailable in ledger: {error}",
            link.payload.artifact_ref
        ))
    })?;

    let head_before = if index.links_by_ref.contains_key(&link.link_ref) {
        idempotent_head_before(&index, &link)?
    } else {
        let head_before = prior_head(&index, &link)?;
        let imported = crate::ledger::import_artifact(root, value)?;
        if imported.artifact_ref != link.link_ref {
            return Err(MoltenError::invalid_harness(format!(
                "imported chain link ref mismatch: got {}, expected {}",
                imported.artifact_ref, link.link_ref
            )));
        }
        head_before
    };

    let predicate_receipt_ref = append_predicate_ref(root, &link, head_before.as_deref())?;
    let receipt_value = chain_append_receipt_value(&link, head_before.as_deref(), &predicate_receipt_ref);
    let receipt_ref = canonical_hash(&receipt_value)?;
    let imported_receipt = crate::ledger::import_artifact(root, &receipt_value)?;
    if imported_receipt.artifact_ref != receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain append receipt ref mismatch: got {}, expected {receipt_ref}",
            imported_receipt.artifact_ref
        )));
    }

    Ok(ChainAppend {
        link_ref: link.link_ref.clone(),
        payload_ref: link.payload.artifact_ref.clone(),
        head_before,
        head_after: link.link_ref,
        predicate_receipt_ref,
        receipt_ref,
        receipt_value,
    })
}
