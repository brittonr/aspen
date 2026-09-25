
fn pass_evidence(
    chain: &crate::evidence_chain::ChainScope,
    manifest: &ChunkManifest,
    link_refs: &[String],
    receipt_refs: &[String],
) -> Result<PassEvidence> {
    let ends = chain_ends(link_refs)?;
    let predicate_values = predicate_set(PredicateInput {
        manifest,
        link_refs,
        receipt_refs,
        ends: &ends,
    });
    let predicate_refs = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect::<Vec<_>>();
    let verify_value = verify_value(VerifyInput {
        chain,
        link_refs,
        receipt_refs,
        ends: &ends,
        predicate_refs: &predicate_refs,
    });
    let verify_ref = canonical_hash(&verify_value)?;
    Ok(PassEvidence {
        predicate_values,
        predicate_refs,
        verify_value,
        verify_ref,
    })
}
