
fn build_gate_chain_evidence(
    report_ref: &str,
    suite_ref: &str,
    final_state_hash: &str,
    profile: &str,
) -> Result<ChainEvidence> {
    let link = pass_link(report_ref, suite_ref, final_state_hash, profile)?;
    let predicates = pass_predicates(&link)?;
    let artifacts = pass_artifacts(&link, &predicates, suite_ref)?;
    Ok(ChainEvidence {
        link_ref: link.link_ref,
        anchor_ref: artifacts.anchor_ref,
        verify_receipt_ref: artifacts.verify_ref,
        checkpoint_ref: artifacts.checkpoint_ref,
        range_predicate_ref: predicates.range_ref,
        predicate_receipt_refs: predicates.refs,
        link_value: link.link_value,
        anchor_value: artifacts.anchor_value,
        verify_receipt_value: artifacts.verify_value,
        checkpoint_value: artifacts.checkpoint_value,
        predicate_values: predicates.values,
    })
}

fn pass_link(report_ref: &str, suite_ref: &str, final_state_hash: &str, profile: &str) -> Result<PassLink> {
    let chain = crate::evidence_chain::ChainScope::new("harness-pass-evidence", report_ref, profile);
    let producer_key_ref = canonical_hash(&record("gate-chain-producer-key", vec![string("molten")]))?;
    let producer = crate::evidence_chain::ChainProducer::new("molten-gate", producer_key_ref);
    let trellis_input_ref = canonical_hash(&record("gate-chain-input", vec![
        string(report_ref),
        string(suite_ref),
        string(final_state_hash),
    ]))?;
    let link_value = crate::evidence_chain::chain_link_value(&crate::evidence_chain::ChainLinkInput::genesis(
        chain.clone(),
        crate::evidence_chain::ChainPayload::new("harness-report", report_ref, HARNESS_REPORT_SCHEMA),
        vec![
            crate::evidence_chain::ChainContextRef::new("suite", suite_ref),
            crate::evidence_chain::ChainContextRef::new("final-state", final_state_hash),
        ],
        producer.clone(),
        trellis_input_ref,
    ));
    let link = crate::evidence_chain::parse_chain_link(&link_value)?;
    let link_ref = link.link_ref.clone();
    let scope_context_ref = canonical_hash(&record("gate-chain-scope", vec![
        string(&chain.scope),
        string(&chain.id),
        string(&chain.epoch),
    ]))?;
    Ok(PassLink {
        chain,
        producer,
        link_ref: link_ref.clone(),
        link_value,
        payload_refs: vec![report_ref.to_string()],
        subject_refs: vec![link_ref],
        context_refs: vec![scope_context_ref, suite_ref.to_string(), final_state_hash.to_string()],
    })
}

fn pass_predicate(input: Pred<'_>) -> IoValue {
    crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
        predicate: input.predicate,
        decision: "pass",
        subject_refs: input.subject_refs,
        input_refs: input.input_refs,
        context_refs: input.context_refs,
        checks: input.checks,
    })
}
