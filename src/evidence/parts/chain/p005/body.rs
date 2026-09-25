
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
    let index = build_chain_index(root)?;
    validate_checkpoint_prior(&index, input)?;
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
    validate_checkpoint_head_freshness(&index, input)?;
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

fn validate_checkpoint_prior(index: &ChainIndex, input: &ChainCheckpointInput) -> Result<()> {
    match input.prior_checkpoint_ref.as_deref() {
        Some(prior_checkpoint_ref) => {
            let prior = index.checkpoints_by_ref.get(prior_checkpoint_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!(
                    "prior checkpoint {prior_checkpoint_ref} is unavailable in ledger"
                ))
            })?;
            if prior.chain != input.chain {
                return Err(MoltenError::invalid_harness(format!(
                    "prior checkpoint {prior_checkpoint_ref} belongs to {:?}, expected {:?}",
                    prior.chain, input.chain
                )));
            }
            if checkpoint_child_count(index, prior_checkpoint_ref, &input.chain) > 0 {
                return Err(MoltenError::invalid_harness(format!(
                    "prior checkpoint {prior_checkpoint_ref} already has an accepted successor"
                )));
            }
            ensure_link_descends_from(index, &input.chain, &input.head_ref, &prior.head_ref)
        }
        None => {
            let existing = index.checkpoints_for_chain(&input.chain);
            if existing.is_empty() {
                Ok(())
            } else {
                Err(MoltenError::invalid_harness(format!(
                    "checkpoint prior checkpoint is required for {:?}; existing checkpoints {:?}",
                    input.chain, existing
                )))
            }
        }
    }
}

fn checkpoint_child_count(index: &ChainIndex, prior_checkpoint_ref: &str, chain: &ChainScope) -> usize {
    index
        .checkpoints_by_ref
        .values()
        .filter(|checkpoint| {
            &checkpoint.chain == chain && checkpoint.prior_checkpoint_ref.as_deref() == Some(prior_checkpoint_ref)
        })
        .count()
}

fn validate_checkpoint_head_freshness(index: &ChainIndex, input: &ChainCheckpointInput) -> Result<()> {
    let heads = index.heads_for_chain(&input.chain);
    if heads == vec![input.head_ref.clone()] {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} is not the current chain head for {:?}: current heads {:?}",
            input.head_ref, input.chain, heads
        )))
    }
}

fn ensure_link_descends_from(
    index: &ChainIndex,
    chain: &ChainScope,
    descendant_ref: &str,
    ancestor_ref: &str,
) -> Result<()> {
    let ancestor = index.links_by_ref.get(ancestor_ref).ok_or_else(|| {
        MoltenError::invalid_harness(format!("prior checkpoint head {ancestor_ref} is unavailable in ledger"))
    })?;
    if &ancestor.chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "prior checkpoint head {ancestor_ref} belongs to {:?}, expected {:?}",
            ancestor.chain, chain
        )));
    }
    if descendant_ref == ancestor_ref {
        return Ok(());
    }

    let mut current_ref = descendant_ref.to_string();
    let mut seen = OrderedSet::new();
    for _ in 0..=index.links_by_ref.len() {
        if !seen.insert(current_ref.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "checkpoint descent from {descendant_ref} to {ancestor_ref} encountered a cycle at {current_ref}"
            )));
        }
        let current = index.links_by_ref.get(&current_ref).ok_or_else(|| {
            MoltenError::invalid_harness(format!("checkpoint head walk reached unavailable link {current_ref}"))
        })?;
        if &current.chain != chain {
            return Err(MoltenError::invalid_harness(format!(
                "checkpoint head walk crossed into {:?}, expected {:?}",
                current.chain, chain
            )));
        }
        let Some(previous_ref) = current.previous_link_ref.as_ref() else {
            break;
        };
        if previous_ref == ancestor_ref {
            return Ok(());
        }
        current_ref.clone_from(previous_ref);
    }
    Err(MoltenError::invalid_harness(format!(
        "checkpoint head {descendant_ref} does not descend from prior checkpoint head {ancestor_ref}"
    )))
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
