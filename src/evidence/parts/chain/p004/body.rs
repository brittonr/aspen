
fn walk_next(
    input: LinkWalkInput<'_>,
    current_ref: &str,
    state: &mut WalkState,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Result<Option<String>> {
    if !state.seen_refs.insert(current_ref.to_string()) {
        diagnostics.push_item(ChainDiagnostic::new("cycle", "chain segment contains a repeated link ref", vec![
            current_ref.to_string(),
        ]));
        return Ok(None);
    }
    let Some(link) = input.index.links_by_ref.get(current_ref) else {
        diagnostics.push_item(ChainDiagnostic::new(
            "missing-link",
            "chain segment names an unavailable previous link",
            vec![current_ref.to_string()],
        ));
        return Ok(None);
    };
    if &link.chain != input.chain {
        diagnostics.push_item(ChainDiagnostic::new(
            "link-chain-mismatch",
            "chain segment crossed into a different scope/id/epoch",
            vec![link.link_ref.clone()],
        ));
        return Ok(None);
    }
    note_payload(input.root, link, state, diagnostics);
    push_bounded(
        &mut state.reverse_segment,
        link.link_ref.clone(),
        MAX_EVIDENCE_CHAIN_LINKS,
        "evidence chain verified links",
    )?;
    if input.anchor_ref == Some(link.link_ref.as_str()) {
        return Ok(None);
    }
    if let Some(previous_ref) = &link.previous_link_ref {
        return Ok(Some(previous_ref.clone()));
    }
    if let Some(anchor_ref) = input.anchor_ref {
        diagnostics.push_item(ChainDiagnostic::new(
            "anchor-descent",
            "head does not descend from requested anchor",
            vec![anchor_ref.to_string(), link.link_ref.clone()],
        ));
    }
    Ok(None)
}

fn note_payload(
    root: &Path,
    link: &ChainLink,
    state: &mut WalkState,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if let Err(error) = crate::ledger::read_artifact(root, &link.payload.artifact_ref) {
        diagnostics.push_item(ChainDiagnostic::new(
            "missing-payload",
            format!("payload ref is unavailable in the ledger: {error}"),
            vec![link.payload.artifact_ref.clone(), link.link_ref.clone()],
        ));
    } else {
        state.payload_refs.insert(link.payload.artifact_ref.clone());
    }
}

fn finish_verify(input: FinishInput<'_>) -> Result<ChainVerify> {
    let receipt_value = finish_value(&input);
    let receipt_ref = canonical_hash(&receipt_value)?;
    let imported_receipt = crate::ledger::import_artifact(input.root, &receipt_value)?;
    if imported_receipt.artifact_ref != receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain verify receipt ref mismatch: got {}, expected {receipt_ref}",
            imported_receipt.artifact_ref
        )));
    }

    Ok(ChainVerify {
        decision: input.decision,
        chain: input.chain.clone(),
        anchor_ref: input.anchor_ref.map(ToOwned::to_owned),
        expected_head: input.expected_head.map(ToOwned::to_owned),
        discovered_heads: input.discovered_heads,
        verified_links: input.verified_links,
        payload_refs: input.payload_refs,
        diagnostics: input.diagnostics,
        predicate_receipt_refs: input.predicate_receipt_refs,
        receipt_ref,
        receipt_value,
    })
}

fn finish_value(input: &FinishInput<'_>) -> IoValue {
    let receipt = ChainVerifyReceiptValueInput {
        decision: &input.decision,
        chain: input.chain,
        anchor_ref: input.anchor_ref,
        expected_head: input.expected_head,
        discovered_heads: &input.discovered_heads,
        verified_links: &input.verified_links,
        payload_refs: &input.payload_refs,
        diagnostics: &input.diagnostics,
    };
    chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
        receipt,
        predicate_receipt_refs: &input.predicate_receipt_refs,
        fork_policy: input.fork_policy,
    })
}

pub fn chain_verify_receipt_value(input: &ChainVerifyReceiptValueInput<'_>) -> IoValue {
    chain_verify_receipt_value_with_policy(&ChainVerifyReceiptPolicyValueInput {
        receipt: *input,
        predicate_receipt_refs: &[],
        fork_policy: ChainForkPolicy::RejectUnexpectedForks,
    })
}

pub fn chain_verify_receipt_value_with_policy(input: &ChainVerifyReceiptPolicyValueInput<'_>) -> IoValue {
    let receipt = input.receipt;
    let checks = vec![
        diagnostic_check("no-gap-segment", receipt.diagnostics, &["gap", "genesis-invalid", "append-invalid", "cycle"]),
        fork_policy_check(receipt.diagnostics, input.fork_policy),
        diagnostic_check("payload-availability", receipt.diagnostics, &["missing-payload"]),
        diagnostic_check("anchor-descent", receipt.diagnostics, &[
            "missing-anchor",
            "anchor-chain-mismatch",
            "anchor-descent",
        ]),
        diagnostic_check("expected-head", receipt.diagnostics, &["missing-head", "head-chain-mismatch", "stale-head"]),
    ];
    record("chain-verify-receipt-v1", vec![
        string(EVIDENCE_CHAIN_VERIFY_RECEIPT_SCHEMA),
        record("decision", vec![string(receipt.decision)]),
        chain_record(receipt.chain),
        record("anchor", vec![optional_ref_value(receipt.anchor_ref)]),
        record("expected-head", vec![optional_ref_value(receipt.expected_head)]),
        record("discovered-heads", vec![ref_sequence_value(receipt.discovered_heads)]),
        record("verified-links", vec![ref_sequence_value(receipt.verified_links)]),
        record("payloads", vec![ref_sequence_value(receipt.payload_refs)]),
        record("predicates", vec![ref_sequence_value(input.predicate_receipt_refs)]),
        record("diagnostics", vec![sequence(receipt.diagnostics.iter().map(diagnostic_value).collect())]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_predicate_receipt_value(input: &ChainPredicateReceiptValueInput<'_>) -> IoValue {
    record("chain-predicate-receipt-v1", vec![
        string(EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA),
        record("predicate", vec![string(input.predicate)]),
        record("decision", vec![string(input.decision)]),
        record("subjects", vec![ref_sequence_value(input.subject_refs)]),
        record("inputs", vec![ref_sequence_value(input.input_refs)]),
        record("context", vec![ref_sequence_value(input.context_refs)]),
        record("checks", vec![sequence(input.checks.iter().map(check_value).collect())]),
    ])
}

pub fn parse_chain_predicate_receipt(value: &IoValue) -> Result<ChainPredicateReceipt> {
    let receipt = value
        .collect_simple_record("chain-predicate-receipt-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-predicate-receipt-v1 ...>"))?;
    require_schema(&receipt[0], EVIDENCE_CHAIN_PREDICATE_RECEIPT_SCHEMA, "chain predicate receipt schema")?;
    let parsed = ChainPredicateReceipt {
        receipt_ref: canonical_hash(value)?,
        predicate: record_string(&receipt[1], "predicate", "chain predicate name")?,
        decision: record_string(&receipt[2], "decision", "chain predicate decision")?,
        subject_refs: record_ref_sequence(&receipt[3], "subjects")?,
        input_refs: record_ref_sequence(&receipt[4], "inputs")?,
        context_refs: record_ref_sequence(&receipt[5], "context")?,
        checks: parse_checks(&receipt[6])?,
    };
    validate_chain_predicate_receipt_shape(&parsed)?;
    Ok(parsed)
}

pub fn chain_fork_evidence_value(input: &ChainForkEvidenceValueInput<'_>) -> IoValue {
    record("chain-fork-evidence-v1", vec![
        string(EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA),
        chain_record(input.chain),
        record("parent", vec![optional_ref_value(input.parent_ref)]),
        record("children", vec![ref_sequence_value(input.child_refs)]),
        record("selected-head", vec![optional_ref_value(input.selected_head)]),
        record("profile", vec![string(input.fork_policy.profile())]),
        record("decision", vec![string(input.fork_policy.decision_for_fork())]),
        record("checks", vec![sequence(vec![
            record("check", vec![string("fork-detected"), string("pass")]),
            record("check", vec![string("fork-policy-profile"), string("pass")]),
            record("check", vec![string("diagnostic-retention"), string("pass")]),
        ])]),
    ])
}

pub fn sign_chain_receipt(
    receipt: &IoValue,
    signer: &str,
    trust_root: &str,
    key: &str,
    parents: &[String],
) -> Result<IoValue> {
    sign_receipt(&SignReceiptInput {
        receipt,
        signer,
        purpose: CHAIN_EVIDENCE_PURPOSE,
        trust_root,
        key,
        parents,
    })
}

pub fn verify_signed_chain_receipt(value: &IoValue, trust_root: &str, key: &str) -> Result<SignedReceipt> {
    verify_signed_receipt(value, CHAIN_EVIDENCE_PURPOSE, trust_root, key)
}

pub fn signed_receipt_payload(signed_receipt_ref: impl Into<String>) -> ChainPayload {
    ChainPayload::new("signed-receipt", signed_receipt_ref.into(), EVIDENCE_SIGNED_RECEIPT_SCHEMA)
}

pub fn parse_chain_link(value: &IoValue) -> Result<ChainLink> {
    let link = value
        .collect_simple_record("chain-link-v1", Some(9))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-link-v1 ...>"))?;
    let schema = required_string(&link[0], "chain link schema")?;
    if schema != EVIDENCE_CHAIN_LINK_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported chain link schema {schema}; expected {EVIDENCE_CHAIN_LINK_SCHEMA}"
        )));
    }

    let parsed = ChainLink {
        link_ref: chain_link_ref(value)?,
        chain: parse_chain(&link[1])?,
        sequence: record_u64(&link[2], "seq", "chain link sequence")?,
        previous_link_ref: parse_previous_link_ref(&link[3])?,
        payload: parse_payload(&link[4])?,
        context_refs: parse_context_refs(&link[5])?,
        producer: parse_producer(&link[6])?,
        trellis: parse_trellis(&link[7])?,
        checks: parse_checks(&link[8])?,
    };
    validate_chain_link_shape(&parsed)?;
    Ok(parsed)
}

pub fn parse_chain_fork_evidence(value: &IoValue) -> Result<ChainForkEvidence> {
    let fork = value
        .collect_simple_record("chain-fork-evidence-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-fork-evidence-v1 ...>"))?;
    require_schema(&fork[0], EVIDENCE_CHAIN_FORK_EVIDENCE_SCHEMA, "chain fork evidence schema")?;
    let parsed = ChainForkEvidence {
        evidence_ref: canonical_hash(value)?,
        chain: parse_chain(&fork[1])?,
        parent_ref: record_optional_ref(&fork[2], "parent")?,
        child_refs: record_ref_sequence(&fork[3], "children")?,
        selected_head: record_optional_ref(&fork[4], "selected-head")?,
        profile: record_string(&fork[5], "profile", "fork policy profile")?,
        decision: record_string(&fork[6], "decision", "fork policy decision")?,
        checks: parse_checks(&fork[7])?,
    };
    validate_chain_fork_evidence_shape(&parsed)?;
    Ok(parsed)
}

pub fn parse_chain_anchor(value: &IoValue) -> Result<ChainAnchor> {
    let anchor = value
        .collect_simple_record("chain-anchor-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <chain-anchor-v1 ...>"))?;
    require_schema(&anchor[0], EVIDENCE_CHAIN_ANCHOR_SCHEMA, "chain anchor schema")?;
    let parsed = ChainAnchor {
        anchor_ref: canonical_hash(value)?,
        chain: parse_chain(&anchor[1])?,
        link_ref: record_string(&anchor[2], "anchor", "anchor link ref")?,
        policy_refs: record_ref_sequence(&anchor[3], "policy")?,
        producer: parse_producer(&anchor[4])?,
        checks: parse_checks(&anchor[5])?,
    };
    validate_chain_anchor_shape(&parsed)?;
    Ok(parsed)
}
