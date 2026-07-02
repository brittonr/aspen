
pub fn parse_chain_checkpoint(value: &IoValue) -> Result<ChainCheckpoint> {
    let checkpoint = value
        .collect_simple_record("chain-checkpoint-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-checkpoint-v1 ...>"))?;
    require_schema(&checkpoint[0], EVIDENCE_CHAIN_CHECKPOINT_SCHEMA, "chain checkpoint schema")?;
    let range = parse_checkpoint_range(&checkpoint[3])?;
    let parsed = ChainCheckpoint {
        checkpoint_ref: canonical_hash(value)?,
        chain: parse_chain(&checkpoint[1])?,
        prior_checkpoint_ref: record_optional_ref(&checkpoint[2], "prior-checkpoint")?,
        anchor_link_ref: range.0,
        head_ref: range.1,
        verify_receipt_ref: range.2,
        range_predicate_ref: range.3,
        policy_refs: record_ref_sequence(&checkpoint[4], "policy")?,
        membership_refs: record_ref_sequence(&checkpoint[5], "membership")?,
        producer: parse_producer(&checkpoint[7])?,
        checks: parse_checks(&checkpoint[8])?,
    };
    parse_control_plane(&checkpoint[6])?;
    validate_chain_checkpoint_shape(&parsed)?;
    Ok(parsed)
}

pub fn validate_genesis(link: &ChainLink) -> Result<()> {
    validate_chain_link_shape(link)?;
    if link.sequence != 0 {
        return Err(MoltenError::invalid_harness(format!(
            "genesis chain link sequence must be 0, got {}",
            link.sequence
        )));
    }
    if link.previous_link_ref.is_some() {
        return Err(MoltenError::invalid_harness("genesis chain link must not name a previous link"));
    }
    require_trellis_pass(link, GENESIS_VALID_PREDICATE)?;
    require_pass_check(link, "genesis-sequence")?;
    require_pass_check(link, "no-previous-link")?;
    require_pass_check(link, "payload-ref-binding")?;
    require_pass_check(link, "scoped-chain-not-global-order")
}

pub fn validate_append(previous: &ChainLink, link: &ChainLink) -> Result<()> {
    validate_chain_link_shape(previous)?;
    validate_chain_link_shape(link)?;
    if previous.chain != link.chain {
        return Err(MoltenError::invalid_harness(format!(
            "append link must stay in the same chain scope/id/epoch: previous={:?} next={:?}",
            previous.chain, link.chain
        )));
    }
    let Some(previous_link_ref) = &link.previous_link_ref else {
        return Err(MoltenError::invalid_harness("append chain link must name a previous link"));
    };
    if previous_link_ref != &previous.link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "append previous link ref mismatch: got {previous_link_ref}, expected {}",
            previous.link_ref
        )));
    }
    let expected_sequence = previous.sequence.checked_add(1).ok_or_else(|| {
        MoltenError::invalid_harness(format!("cannot append after max sequence {}", previous.sequence))
    })?;
    if link.sequence != expected_sequence {
        return Err(MoltenError::invalid_harness(format!(
            "append sequence must be previous + 1: got {}, expected {expected_sequence}",
            link.sequence
        )));
    }
    require_trellis_pass(link, APPEND_VALID_PREDICATE)?;
    require_pass_check(link, "same-chain-scope")?;
    require_pass_check(link, "previous-link-binding")?;
    require_pass_check(link, "sequence-monotonicity")?;
    require_pass_check(link, "payload-ref-binding")
}

fn validate_checkpoint_input(root: &Path, input: &ChainCheckpointInput) -> Result<()> {
    validate_chain_scope(&input.chain)?;
    require_ref(&input.anchor_link_ref, "checkpoint anchor link ref")?;
    require_ref(&input.head_ref, "checkpoint head ref")?;
    require_ref(&input.verify_receipt_ref, "checkpoint verify receipt ref")?;
    require_ref(&input.range_predicate_ref, "checkpoint range predicate ref")?;
    if let Some(prior_checkpoint_ref) = &input.prior_checkpoint_ref {
        let prior_value = crate::ledger::read_artifact(root, prior_checkpoint_ref)?;
        let prior = parse_chain_checkpoint(&prior_value)?;
        if prior.chain != input.chain {
            return Err(MoltenError::invalid_harness(format!(
                "prior checkpoint {prior_checkpoint_ref} belongs to {:?}, expected {:?}",
                prior.chain, input.chain
            )));
        }
    }
    let index = build_chain_index(root)?;
    let Some(anchor) = index.links_by_ref.get(&input.anchor_link_ref) else {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint anchor link {} is unavailable in ledger",
            input.anchor_link_ref
        )));
    };
    if anchor.chain != input.chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint anchor link {} belongs to {:?}, expected {:?}",
            input.anchor_link_ref, anchor.chain, input.chain
        )));
    }
    let Some(head) = index.links_by_ref.get(&input.head_ref) else {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} is unavailable in ledger",
            input.head_ref
        )));
    };
    if head.chain != input.chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} belongs to {:?}, expected {:?}",
            input.head_ref, head.chain, input.chain
        )));
    }
    let verify_value = crate::ledger::read_artifact(root, &input.verify_receipt_ref)?;
    validate_checkpoint_verify_receipt(CheckpointVerifyReceiptValidationInput {
        root,
        value: &verify_value,
        chain: &input.chain,
        anchor_link_ref: &input.anchor_link_ref,
        head_ref: &input.head_ref,
        range_predicate_ref: &input.range_predicate_ref,
    })?;
    for policy_ref in &input.policy_refs {
        require_ref(policy_ref, "checkpoint policy ref")?;
    }
    for membership_ref in &input.membership_refs {
        require_ref(membership_ref, "checkpoint membership ref")?;
    }
    validate_producer(&input.producer)?;
    for check in &input.checks {
        require_non_empty(&check.name, "checkpoint check name")?;
        require_non_empty(&check.decision, "checkpoint check decision")?;
    }
    require_input_pass_check(input, "raft-control-plane-command")?;
    require_input_pass_check(input, "verified-range")?;
    require_input_pass_check(input, "checkpoint-freshness")
}

fn validate_checkpoint_verify_receipt(input: CheckpointVerifyReceiptValidationInput<'_>) -> Result<()> {
    let receipt = input
        .value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt for checkpoint"))?;
    let root = input.root;
    let chain = input.chain;
    let anchor_link_ref = input.anchor_link_ref;
    let head_ref = input.head_ref;
    let range_predicate_ref = input.range_predicate_ref;
    require_schema(&receipt[0], EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA, "chain verify receipt schema")?;
    let decision = record_string(&receipt[1], "decision", "chain verify decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt decision must be pass, got {decision}"
        )));
    }
    let receipt_chain = parse_chain(&receipt[2])?;
    if &receipt_chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt chain {:?} does not match {:?}",
            receipt_chain, chain
        )));
    }
    let anchor = record_optional_ref(&receipt[3], "anchor")?
        .ok_or_else(|| MoltenError::invalid_harness("checkpoint verify receipt must name an anchor"))?;
    if anchor != anchor_link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt anchor {anchor} does not match {anchor_link_ref}"
        )));
    }
    let expected_head = record_optional_ref(&receipt[4], "expected-head")?
        .ok_or_else(|| MoltenError::invalid_harness("checkpoint verify receipt must name expected head"))?;
    if expected_head != head_ref {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint verify receipt head {expected_head} does not match {head_ref}"
        )));
    }
    validate_range_binding(RangeBindingInput {
        root,
        value: input.value,
        chain,
        anchor_link_ref,
        head_ref,
        range_predicate_ref,
    })
}

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
