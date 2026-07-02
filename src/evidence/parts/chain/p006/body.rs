
fn detect_forks_sequence_conflicts_and_store_evidence(input: ForkDetectionInput<'_>) -> Result<()> {
    let root = input.root;
    let index = input.index;
    let chain = input.chain;
    let chain_refs = input.chain_refs;
    let discovered_heads = input.discovered_heads;
    let selected_head = input.selected_head;
    let fork_policy = input.fork_policy;
    let diagnostics = input.diagnostics;
    if discovered_heads.len() > 1 {
        let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
            root,
            chain,
            parent_ref: None,
            child_refs: discovered_heads,
            selected_head,
            fork_policy,
        })?;
        let mut refs = discovered_heads.to_vec();
        refs.push(evidence_ref);
        diagnostics.push(ChainDiagnostic::new("fork", "chain has more than one derived head", refs));
    }
    for link_ref in chain_refs {
        let children = index
            .children_for_parent(link_ref)
            .into_iter()
            .filter(|child_ref| index.links_by_ref.get(child_ref).is_some_and(|child| &child.chain == chain))
            .collect::<Vec<_>>();
        if children.len() > 1 {
            let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
                root,
                chain,
                parent_ref: Some(link_ref),
                child_refs: &children,
                selected_head,
                fork_policy,
            })?;
            let mut refs = children;
            refs.push(evidence_ref);
            diagnostics.push(ChainDiagnostic::new(
                "fork",
                "chain parent has multiple children under fork verification policy",
                refs,
            ));
        }
    }
    for ((entry_chain, sequence), occupants) in &index.links_by_sequence {
        if entry_chain == chain && occupants.len() > 1 {
            let occupants = occupants.iter().cloned().collect::<Vec<_>>();
            let evidence_ref = store_chain_fork_evidence(StoreChainForkEvidenceInput {
                root,
                chain,
                parent_ref: None,
                child_refs: &occupants,
                selected_head,
                fork_policy,
            })?;
            let mut refs = occupants;
            refs.push(evidence_ref);
            diagnostics.push(ChainDiagnostic::new(
                "sequence-conflict",
                format!("chain sequence {sequence} has multiple links"),
                refs,
            ));
        }
    }
    Ok(())
}

fn store_chain_fork_evidence(input: StoreChainForkEvidenceInput<'_>) -> Result<String> {
    let value = chain_fork_evidence_value(&ChainForkEvidenceValueInput {
        chain: input.chain,
        parent_ref: input.parent_ref,
        child_refs: input.child_refs,
        selected_head: input.selected_head,
        fork_policy: input.fork_policy,
    });
    let parsed = parse_chain_fork_evidence(&value)?;
    let imported = crate::ledger::import_artifact(input.root, &value)?;
    if imported.artifact_ref != parsed.evidence_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported fork evidence ref mismatch: got {}, expected {}",
            imported.artifact_ref, parsed.evidence_ref
        )));
    }
    Ok(parsed.evidence_ref)
}

fn store_chain_predicate_receipt(input: StoreChainPredicateReceiptInput<'_>) -> Result<String> {
    let value = chain_predicate_receipt_value(&input.receipt);
    let parsed = parse_chain_predicate_receipt(&value)?;
    let imported = crate::ledger::import_artifact(input.root, &value)?;
    if imported.artifact_ref != parsed.receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain predicate receipt ref mismatch: got {}, expected {}",
            imported.artifact_ref, parsed.receipt_ref
        )));
    }
    Ok(parsed.receipt_ref)
}

fn predicate_input_refs(link: &ChainLink) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(previous_link_ref) = &link.previous_link_ref {
        refs.push(previous_link_ref.clone());
    }
    refs.push(link.payload.artifact_ref.clone());
    refs.push(link.trellis.input_ref.clone());
    refs
}

fn predicate_context_refs(link: &ChainLink) -> Vec<String> {
    link.context_refs.iter().map(|context| context.artifact_ref.clone()).collect()
}

fn store_segment_predicate_receipts(input: SegmentPredicateReceiptsInput<'_>) -> Result<Vec<String>> {
    let scope_refs = scope_context_refs(input.chain)?;
    let mut refs = Vec::with_capacity(4);
    refs.push(gap_receipt_ref(&input, &scope_refs)?);
    refs.push(fork_receipt_ref(&input, &scope_refs)?);
    if let Some(receipt_ref) = anchor_receipt_ref(&input, &scope_refs)? {
        refs.push(receipt_ref);
    }
    if let Some(receipt_ref) = checkpoint_receipt_ref(&input, &scope_refs)? {
        refs.push(receipt_ref);
    }
    Ok(refs)
}

fn gap_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<String> {
    let decision = predicate_decision(input.diagnostics, input.fork_policy, &[
        "gap",
        "genesis-invalid",
        "append-invalid",
        "cycle",
    ]);
    let checks = vec![
        ChainCheck::pass("segment-contiguity"),
        ChainCheck::pass("canonical-link-order"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: SEGMENT_NO_GAP_PREDICATE,
            decision,
            subject_refs: input.verified_links,
            input_refs: input.payload_refs,
            context_refs: scope_refs,
            checks: &checks,
        },
    })
}

fn fork_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<String> {
    let decision = match (
        input
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.kind.as_str(), "fork" | "sequence-conflict")),
        input.fork_policy,
    ) {
        (false, _) => "pass",
        (true, ChainForkPolicy::RejectUnexpectedForks) => "fail",
        (true, ChainForkPolicy::RetainForkEvidence) => "retained",
    };
    let checks = vec![
        ChainCheck::pass("fork-policy-profile"),
        ChainCheck::pass("fork-evidence-binding"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: SEGMENT_NO_FORK_PREDICATE,
            decision,
            subject_refs: input.discovered_heads,
            input_refs: input.verified_links,
            context_refs: scope_refs,
            checks: &checks,
        },
    })
}

fn anchor_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<Option<String>> {
    if input.anchor_ref.is_none() && input.expected_head.is_none() {
        return Ok(None);
    }
    let mut subjects = Vec::with_capacity(2);
    if let Some(anchor_ref) = input.anchor_ref {
        subjects.push(anchor_ref.to_string());
    }
    if let Some(expected_head) = input.expected_head {
        subjects.push(expected_head.to_string());
    }
    let checks = vec![ChainCheck::pass("anchor-descent"), ChainCheck::pass("head-binding")];
    let receipt_ref = store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: predicate_decision(input.diagnostics, input.fork_policy, &[
                "missing-anchor",
                "anchor-chain-mismatch",
                "anchor-descent",
                "missing-head",
                "head-chain-mismatch",
                "stale-head",
            ]),
            subject_refs: &subjects,
            input_refs: input.verified_links,
            context_refs: scope_refs,
            checks: &checks,
        },
    })?;
    Ok(Some(receipt_ref))
}

fn checkpoint_receipt_ref(input: &SegmentPredicateReceiptsInput<'_>, scope_refs: &[String]) -> Result<Option<String>> {
    if input.anchor_ref.is_none() || input.expected_head.is_none() || input.decision != "pass" {
        return Ok(None);
    }
    let checks = vec![
        ChainCheck::pass("checkpoint-range-coverage"),
        ChainCheck::pass("verified-range"),
    ];
    let receipt_ref = store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root: input.root,
        receipt: ChainPredicateReceiptValueInput {
            predicate: CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: input.verified_links,
            input_refs: input.payload_refs,
            context_refs: scope_refs,
            checks: &checks,
        },
    })?;
    Ok(Some(receipt_ref))
}

fn predicate_decision(
    diagnostics: &[ChainDiagnostic],
    fork_policy: ChainForkPolicy,
    failing_kinds: &[&str],
) -> &'static str {
    if diagnostics.iter().any(|diagnostic| {
        failing_kinds.contains(&diagnostic.kind.as_str()) && diagnostic_is_fatal(diagnostic, fork_policy)
    }) {
        "fail"
    } else {
        "pass"
    }
}

fn scope_context_refs(chain: &ChainScope) -> Result<Vec<String>> {
    Ok(vec![canonical_hash(&chain_record(chain))?])
}
