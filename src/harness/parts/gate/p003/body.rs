
fn pass_predicates(link: &PassLink) -> Result<PassPredicates> {
    let genesis_checks = vec![
        crate::evidence_chain::ChainCheck::pass("trellis-bounded-predicate"),
        crate::evidence_chain::ChainCheck::pass("predicate-decision-binding"),
    ];
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    let checkpoint_checks = vec![
        crate::evidence_chain::ChainCheck::pass("checkpoint-range-coverage"),
        crate::evidence_chain::ChainCheck::pass("verified-range"),
    ];
    let values = vec![
        pass_predicate(Pred {
            predicate: crate::evidence_chain::GENESIS_VALID_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &genesis_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &segment_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.subject_refs,
            context_refs: &link.context_refs,
            checks: &fork_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.subject_refs,
            context_refs: &link.context_refs,
            checks: &anchor_checks,
        }),
        pass_predicate(Pred {
            predicate: crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
            subject_refs: &link.subject_refs,
            input_refs: &link.payload_refs,
            context_refs: &link.context_refs,
            checks: &checkpoint_checks,
        }),
    ];
    let (refs, range_ref) = pass_predicate_refs(&values)?;
    Ok(PassPredicates {
        values,
        refs,
        range_ref,
    })
}

fn pass_predicate_refs(values: &[IoValue]) -> Result<(Vec<String>, String)> {
    let mut refs = Vec::with_capacity(values.len());
    let mut range_ref = None;
    for value in values {
        let parsed = crate::evidence_chain::parse_chain_predicate_receipt(value)?;
        if parsed.predicate == crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE {
            range_ref = Some(parsed.receipt_ref.clone());
        }
        refs.push(parsed.receipt_ref);
    }
    Ok((
        refs,
        range_ref.ok_or_else(|| {
            MoltenError::invalid_harness("gate chain evidence did not build checkpoint range predicate")
        })?,
    ))
}

fn pass_verify_value(link: &PassLink, predicates: &PassPredicates) -> IoValue {
    let diagnostics = Vec::new();
    let receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain: &link.chain,
        anchor_ref: Some(&link.link_ref),
        expected_head: Some(&link.link_ref),
        discovered_heads: std::slice::from_ref(&link.link_ref),
        verified_links: std::slice::from_ref(&link.link_ref),
        payload_refs: &link.payload_refs,
        diagnostics: &diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt,
            predicate_receipt_refs: &predicates.refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

fn pass_artifacts(link: &PassLink, predicates: &PassPredicates, suite_ref: &str) -> Result<PassArtifacts> {
    let policy_refs = vec![suite_ref.to_string()];
    let anchor_value =
        crate::evidence_chain::chain_anchor_value(&link.chain, &link.link_ref, &policy_refs, &link.producer);
    let anchor_ref = canonical_hash(&anchor_value)?;
    let verify_value = pass_verify_value(link, predicates);
    let verify_ref = canonical_hash(&verify_value)?;
    let checkpoint_value =
        crate::evidence_chain::chain_checkpoint_value(&crate::evidence_chain::ChainCheckpointInput {
            chain: link.chain.clone(),
            prior_checkpoint_ref: None,
            anchor_link_ref: link.link_ref.clone(),
            head_ref: link.link_ref.clone(),
            verify_receipt_ref: verify_ref.clone(),
            range_predicate_ref: predicates.range_ref.clone(),
            policy_refs,
            membership_refs: vec![suite_ref.to_string()],
            producer: link.producer.clone(),
            checks: vec![
                crate::evidence_chain::ChainCheck::pass("raft-control-plane-command"),
                crate::evidence_chain::ChainCheck::pass("verified-range"),
                crate::evidence_chain::ChainCheck::pass("checkpoint-freshness"),
            ],
        });
    let checkpoint_ref = canonical_hash(&checkpoint_value)?;
    Ok(PassArtifacts {
        anchor_ref,
        anchor_value,
        verify_ref,
        verify_value,
        checkpoint_ref,
        checkpoint_value,
    })
}

fn chain_evidence_value(evidence: &ChainEvidence) -> IoValue {
    record("chain-evidence", vec![
        record("profile", vec![string("local-pass-evidence-chain")]),
        record("link", vec![evidence.link_value.clone()]),
        record("anchor", vec![evidence.anchor_value.clone()]),
        record("verify-receipt", vec![evidence.verify_receipt_value.clone()]),
        record("checkpoint", vec![evidence.checkpoint_value.clone()]),
        record("predicates", vec![sequence(evidence.predicate_values.clone())]),
        record("checks", vec![sequence(
            [
                "chain-continuity",
                "chain-anchor-descent",
                "chain-checkpoint-freshness",
                "chain-predicate-receipts",
            ]
            .iter()
            .map(|name| record("check", vec![string(*name), string("pass")]))
            .collect(),
        )]),
    ])
}

struct EvidenceParts {
    link_value: IoValue,
    anchor_value: IoValue,
    verify_receipt_value: IoValue,
    checkpoint_value: IoValue,
    predicate_values: Vec<IoValue>,
}

struct ParsedPredicates {
    receipts: Vec<crate::evidence_chain::ChainPredicateReceipt>,
    refs: Vec<String>,
}

fn parse_chain_evidence(value: &Value<IoValue>) -> Result<ChainEvidence> {
    let parts = evidence_parts(value)?;
    let link = crate::evidence_chain::parse_chain_link(&parts.link_value)?;
    let link_ref = link.link_ref.clone();
    let anchor = crate::evidence_chain::parse_chain_anchor(&parts.anchor_value)?;
    if anchor.link_ref != link_ref || anchor.chain != link.chain {
        return Err(MoltenError::invalid_harness("gate chain anchor does not bind the gate chain link"));
    }
    let checkpoint = crate::evidence_chain::parse_chain_checkpoint(&parts.checkpoint_value)?;
    if checkpoint.chain != link.chain || checkpoint.anchor_link_ref != link_ref || checkpoint.head_ref != link_ref {
        return Err(MoltenError::invalid_harness("gate chain checkpoint does not bind the anchored chain head"));
    }
    let verify_receipt_ref = canonical_hash(&parts.verify_receipt_value)?;
    if checkpoint.verify_receipt_ref != verify_receipt_ref {
        return Err(MoltenError::invalid_harness("gate chain checkpoint does not bind the embedded verify receipt"));
    }

    let predicates = parsed_predicates(&parts.predicate_values)?;
    require_predicates(&predicates, &checkpoint.range_predicate_ref, &link_ref, &link.payload.artifact_ref)?;
    validate_gate_chain_verify_receipt(
        &parts.verify_receipt_value,
        &link,
        &checkpoint.range_predicate_ref,
        &predicates.refs,
    )?;

    Ok(ChainEvidence {
        link_ref,
        anchor_ref: anchor.anchor_ref,
        verify_receipt_ref,
        checkpoint_ref: checkpoint.checkpoint_ref,
        range_predicate_ref: checkpoint.range_predicate_ref,
        predicate_receipt_refs: predicates.refs,
        link_value: parts.link_value,
        anchor_value: parts.anchor_value,
        verify_receipt_value: parts.verify_receipt_value,
        checkpoint_value: parts.checkpoint_value,
        predicate_values: parts.predicate_values,
    })
}

fn evidence_parts(value: &Value<IoValue>) -> Result<EvidenceParts> {
    let value = value_to_iovalue(value);
    let evidence = simple_record(&value, "chain-evidence", 7)?;
    let profile = required_record_string(&evidence[0], "profile", "chain evidence profile")?;
    if profile != "local-pass-evidence-chain" {
        return Err(MoltenError::invalid_harness(format!("unsupported gate chain evidence profile {profile}")));
    }
    let checks = parse_checks(&evidence[6])?;
    require_check(&checks, "chain-continuity")?;
    require_check(&checks, "chain-anchor-descent")?;
    require_check(&checks, "chain-checkpoint-freshness")?;
    require_check(&checks, "chain-predicate-receipts")?;
    Ok(EvidenceParts {
        link_value: required_record_value(&evidence[1], "link")?,
        anchor_value: required_record_value(&evidence[2], "anchor")?,
        verify_receipt_value: required_record_value(&evidence[3], "verify-receipt")?,
        checkpoint_value: required_record_value(&evidence[4], "checkpoint")?,
        predicate_values: required_record_values(&evidence[5], "predicates")?,
    })
}

fn parsed_predicates(values: &[IoValue]) -> Result<ParsedPredicates> {
    let mut receipts = Vec::with_capacity(values.len());
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        let parsed = crate::evidence_chain::parse_chain_predicate_receipt(value)?;
        refs.push(parsed.receipt_ref.clone());
        receipts.push(parsed);
    }
    Ok(ParsedPredicates { receipts, refs })
}

fn require_predicates(predicates: &ParsedPredicates, range_ref: &str, link_ref: &str, payload_ref: &str) -> Result<()> {
    let range_predicate = require_chain_predicate(
        &predicates.receipts,
        range_ref,
        crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
    )?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::GENESIS_VALID_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chain_predicate_kind(&predicates.receipts, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    if range_predicate.subject_refs != vec![link_ref.to_string()] {
        return Err(MoltenError::invalid_harness("gate chain range predicate subjects do not match anchored link"));
    }
    if range_predicate.input_refs != vec![payload_ref.to_string()] {
        return Err(MoltenError::invalid_harness("gate chain range predicate inputs do not match report payload ref"));
    }
    Ok(())
}
