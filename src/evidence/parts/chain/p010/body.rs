
fn validate_range_binding(input: RangeBindingInput<'_>) -> Result<()> {
    let receipt = input
        .value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt for checkpoint"))?;
    let predicate_refs = record_ref_sequence(&receipt[8], "predicates")?;
    if !predicate_refs.iter().any(|predicate_ref| predicate_ref == input.range_predicate_ref) {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt does not bind range predicate {}",
            input.range_predicate_ref
        )));
    }
    let predicate_value = crate::ledger::read_artifact(input.root, input.range_predicate_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} is unavailable in ledger: {error}",
            input.range_predicate_ref
        ))
    })?;
    let predicate = parse_chain_predicate_receipt(&predicate_value)?;
    if predicate.predicate != CHECKPOINT_COVERS_RANGE_PREDICATE || predicate.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} must be a passing {CHECKPOINT_COVERS_RANGE_PREDICATE} receipt",
            input.range_predicate_ref
        )));
    }
    let verified_links = record_ref_sequence(&receipt[6], "verified-links")?;
    if verified_links.first().map(String::as_str) != Some(input.anchor_link_ref) {
        return Err(MoltenError::invalid_harness("checkpoint verify receipt segment does not begin at anchor"));
    }
    if verified_links.last().map(String::as_str) != Some(input.head_ref) {
        return Err(MoltenError::invalid_harness("checkpoint verify receipt segment does not end at head"));
    }
    let payload_refs = record_ref_sequence(&receipt[7], "payloads")?;
    if predicate.subject_refs != verified_links {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} subjects do not match verified range",
            input.range_predicate_ref
        )));
    }
    if predicate.input_refs != payload_refs {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} inputs do not match verified payload refs",
            input.range_predicate_ref
        )));
    }
    let expected_context_refs = scope_context_refs(input.chain)?;
    if predicate.context_refs != expected_context_refs {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint range predicate {} context does not match checkpoint chain scope",
            input.range_predicate_ref
        )));
    }
    Ok(())
}

fn select_head_for_verification(
    expected_head: Option<&str>,
    discovered_heads: &[String],
    index: &ChainIndex,
    chain: &ChainScope,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Option<String> {
    if let Some(expected_head) = expected_head {
        match index.links_by_ref.get(expected_head) {
            Some(head) if &head.chain == chain => {
                if !discovered_heads.iter().any(|head| head == expected_head) {
                    diagnostics.push_item(ChainDiagnostic::new(
                        "stale-head",
                        "expected head is not a current chain head",
                        vec![expected_head.to_string()],
                    ));
                }
                Some(expected_head.to_string())
            }
            Some(_) => {
                diagnostics.push_item(ChainDiagnostic::new(
                    "head-chain-mismatch",
                    "expected head belongs to a different chain scope/id/epoch",
                    vec![expected_head.to_string()],
                ));
                None
            }
            None => {
                diagnostics.push_item(ChainDiagnostic::new(
                    "missing-head",
                    "expected head is unavailable in the ledger",
                    vec![expected_head.to_string()],
                ));
                None
            }
        }
    } else {
        if discovered_heads.len() > 1 {
            diagnostics.push_item(ChainDiagnostic::new(
                "fork",
                "chain has multiple current heads under no-fork verification policy",
                discovered_heads.to_vec(),
            ));
        }
        discovered_heads.first().cloned()
    }
}
