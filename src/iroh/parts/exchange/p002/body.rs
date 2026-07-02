
fn receipts(
    refs: &[String],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
) -> Result<Vec<ParsedChainVerifyReceipt>> {
    refs.iter()
        .map(|verify_ref| {
            let artifact = artifacts.get(verify_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing verify receipt {verify_ref}"))
            })?;
            parse_chain_verify_receipt_value(&artifact.value)
        })
        .collect()
}

fn primary(receipts: &[ParsedChainVerifyReceipt]) -> Result<&ParsedChainVerifyReceipt> {
    receipts
        .first()
        .ok_or_else(|| MoltenError::invalid_harness("chain bundle must contain a verify receipt"))
}

fn check_primary(input: PrimaryCheck<'_>) -> Result<()> {
    let primary = input.primary;
    if primary.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "chain bundle verify receipt decision must be pass, got {}",
            primary.decision
        )));
    }
    if &primary.chain != input.chain
        || primary.anchor_ref.as_deref() != input.anchor_ref
        || primary.expected_head.as_deref() != input.head_ref
    {
        return Err(MoltenError::invalid_harness("chain bundle verify receipt does not match requested chain range"));
    }
    let has_fork_diagnostics = primary
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.as_str(), "fork" | "sequence-conflict"));
    if has_fork_diagnostics && input.fork_policy == crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks {
        return Err(MoltenError::invalid_harness(
            "chain bundle contains fork diagnostics rejected by production policy",
        ));
    }
    if has_fork_diagnostics && !input.has_forks {
        return Err(MoltenError::invalid_harness("chain bundle fork diagnostics require fork evidence artifacts"));
    }
    Ok(())
}

fn payloads(verify: &ParsedChainVerifyReceipt, artifacts: &OrderedMap<String, &ChainBundleArtifact>) -> Result<()> {
    for payload_ref in &verify.payload_refs {
        if !artifacts.contains_key(payload_ref) {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle missing payload artifact {payload_ref} named by verify receipt"
            )));
        }
    }
    Ok(())
}

fn predicates(
    verify: &ParsedChainVerifyReceipt,
    predicates: &OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
) -> Result<()> {
    for predicate_ref in &verify.predicate_refs {
        let Some(predicate) = predicates.get(predicate_ref) else {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle missing predicate receipt {predicate_ref} named by verify receipt"
            )));
        };
        if predicate.decision == "fail" {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle predicate receipt {predicate_ref} has failing decision"
            )));
        }
    }
    Ok(())
}

fn checkpoints(
    refs: &[String],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
) -> Result<Vec<crate::evidence_chain::ChainCheckpoint>> {
    refs.iter()
        .map(|checkpoint_ref| {
            let artifact = artifacts.get(checkpoint_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing checkpoint {checkpoint_ref}"))
            })?;
            crate::evidence_chain::parse_chain_checkpoint(&artifact.value)
        })
        .collect()
}

fn validate_chain_links(
    links: &OrderedMap<String, crate::evidence_chain::ChainLink>,
    verify: &ParsedChainVerifyReceipt,
    anchor_ref: Option<&str>,
    head_ref: Option<&str>,
) -> Result<()> {
    if verify.verified_links.is_empty() {
        return Err(MoltenError::invalid_harness("chain bundle verify receipt contains no verified links"));
    }
    if let Some(anchor_ref) = anchor_ref
        && verify.verified_links.first().map(String::as_str) != Some(anchor_ref)
    {
        return Err(MoltenError::invalid_harness("chain bundle segment does not begin at requested anchor"));
    }
    if let Some(head_ref) = head_ref
        && verify.verified_links.last().map(String::as_str) != Some(head_ref)
    {
        return Err(MoltenError::invalid_harness("chain bundle segment does not end at requested head"));
    }
    for (position, link_ref) in verify.verified_links.iter().enumerate() {
        let link = links
            .get(link_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("chain bundle missing verified link {link_ref}")))?;
        if link.chain != verify.chain {
            return Err(MoltenError::invalid_harness("chain bundle link belongs to a different chain scope"));
        }
        if position == 0 && anchor_ref.is_none() {
            crate::evidence_chain::validate_genesis(link)?;
        }
        if position > 0 {
            let previous_ref = &verify.verified_links[position - 1];
            let previous = links.get(previous_ref).ok_or_else(|| {
                MoltenError::invalid_harness(format!("chain bundle missing previous link {previous_ref}"))
            })?;
            crate::evidence_chain::validate_append(previous, link)?;
        }
    }
    Ok(())
}

fn validate_bundle_checkpoints(
    chain: &crate::evidence_chain::ChainScope,
    checkpoints: &[crate::evidence_chain::ChainCheckpoint],
    artifacts: &OrderedMap<String, &ChainBundleArtifact>,
    links: &OrderedMap<String, crate::evidence_chain::ChainLink>,
    predicates: &OrderedMap<String, crate::evidence_chain::ChainPredicateReceipt>,
) -> Result<()> {
    for checkpoint in checkpoints {
        if &checkpoint.chain != chain {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint belongs to a different chain"));
        }
        if !links.contains_key(&checkpoint.anchor_link_ref) || !links.contains_key(&checkpoint.head_ref) {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint anchor/head links are unavailable"));
        }
        let Some(verify_artifact) = artifacts.get(&checkpoint.verify_receipt_ref) else {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint verify receipt is unavailable"));
        };
        let verify = parse_chain_verify_receipt_value(&verify_artifact.value)?;
        if verify.decision != "pass"
            || verify.anchor_ref.as_deref() != Some(&checkpoint.anchor_link_ref)
            || verify.expected_head.as_deref() != Some(&checkpoint.head_ref)
        {
            return Err(MoltenError::invalid_harness(
                "chain bundle checkpoint verify receipt does not bind checkpoint range",
            ));
        }
        let Some(predicate) = predicates.get(&checkpoint.range_predicate_ref) else {
            return Err(MoltenError::invalid_harness("chain bundle checkpoint range predicate is unavailable"));
        };
        if predicate.predicate != crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE
            || predicate.decision != "pass"
        {
            return Err(MoltenError::invalid_harness(
                "chain bundle checkpoint range predicate must be a passing checkpoint coverage receipt",
            ));
        }
    }
    Ok(())
}

fn parse_chain_bundle_artifacts(value: &Value<IoValue>) -> Result<Vec<ChainBundleArtifact>> {
    let artifacts = value
        .collect_simple_record("artifacts", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifacts ...> field"))?;
    let sequence = artifacts[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain bundle artifacts must be a sequence"))?;
    let mut parsed = Vec::new();
    for item in sequence.iter() {
        let item = value_to_iovalue(item);
        let artifact = item
            .collect_simple_record("artifact", Some(3))
            .ok_or_else(|| MoltenError::invalid_harness("expected <artifact kind ref value> in chain bundle"))?;
        let kind = required_string(&artifact[0], "chain bundle artifact kind")?;
        let artifact_ref = required_ref(&artifact[1], "chain bundle artifact ref")?;
        let value = value_to_iovalue(&artifact[2]);
        let actual_ref = canonical_hash(&value)?;
        if actual_ref != artifact_ref {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle artifact ref mismatch: got {actual_ref}, expected {artifact_ref}"
            )));
        }
        let actual_kind = crate::ledger::artifact_kind(&value);
        if actual_kind != kind {
            return Err(MoltenError::invalid_harness(format!(
                "chain bundle artifact {artifact_ref} kind mismatch: declared {kind}, parsed {actual_kind}"
            )));
        }
        push_bounded(
            &mut parsed,
            ChainBundleArtifact {
                kind,
                artifact_ref,
                value,
            },
            MAX_CHAIN_BUNDLE_ARTIFACTS,
            "chain bundle artifacts",
        )?;
    }
    if parsed.is_empty() {
        return Err(MoltenError::invalid_harness("chain bundle must contain artifacts"));
    }
    Ok(parsed)
}

fn parse_chain_verify_receipt_value(value: &IoValue) -> Result<ParsedChainVerifyReceipt> {
    let receipt = value
        .collect_simple_record("chain-verify-receipt-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify receipt in chain bundle"))?;
    require_schema(&receipt[0], EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA, "chain verify receipt schema")?;
    let diagnostics = parse_diagnostic_kinds(&receipt[9])?;
    Ok(ParsedChainVerifyReceipt {
        decision: record_string(&receipt[1], "decision")?,
        chain: parse_chain_scope(&receipt[2])?,
        anchor_ref: parse_optional_ref_field(&receipt[3], "anchor")?,
        expected_head: parse_optional_ref_field(&receipt[4], "expected-head")?,
        discovered_heads: parse_ref_sequence_field(&receipt[5], "discovered-heads")?,
        verified_links: parse_ref_sequence_field(&receipt[6], "verified-links")?,
        payload_refs: parse_ref_sequence_field(&receipt[7], "payloads")?,
        predicate_refs: parse_ref_sequence_field(&receipt[8], "predicates")?,
        diagnostics,
    })
}

fn parse_diagnostic_kinds(value: &Value<IoValue>) -> Result<Vec<String>> {
    let diagnostics = value
        .collect_simple_record("diagnostics", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness("expected chain verify diagnostics field"))?;
    let values = diagnostics[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness("chain verify diagnostics must be a sequence"))?;
    values
        .iter()
        .map(|diagnostic| {
            let diagnostic = value_to_iovalue(diagnostic);
            let record = diagnostic
                .collect_simple_record("diagnostic", Some(3))
                .ok_or_else(|| MoltenError::invalid_harness("expected diagnostic record"))?;
            required_string(&record[0], "diagnostic kind")
        })
        .collect()
}

fn chain_receipt_value(input: &ChainReceiptValueInput<'_>) -> IoValue {
    let mut refs = vec![record("artifact-ref", vec![
        string("chain-segment-bundle"),
        string(input.bundle_ref),
    ])];
    refs.extend(
        input
            .verify_receipt_refs
            .iter()
            .map(|verify_ref| record("artifact-ref", vec![string("chain-verify-receipt"), string(verify_ref)])),
    );
    refs.extend(
        input
            .checkpoint_refs
            .iter()
            .map(|checkpoint_ref| record("artifact-ref", vec![string("chain-checkpoint"), string(checkpoint_ref)])),
    );
    record("iroh-chain-exchange-receipt-v1", vec![
        string(CHAIN_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("node", vec![string(input.node)]),
        record("peer", vec![string(input.peer)]),
        record("ticket", vec![string(input.ticket)]),
        record("artifact-refs", vec![sequence(refs)]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("content-addressed-chain-bundle"), string("pass")]),
            record("check", vec![string("chain-segment-verified"), string("pass")]),
            record("check", vec![string("predicate-receipts-bound"), string("pass")]),
            record("check", vec![string("checkpoint-binding"), string("pass")]),
            record("check", vec![string("transport-does-not-grant-trust"), string("pass")]),
        ])]),
    ])
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual != expected {
        return Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")));
    }
    Ok(())
}
