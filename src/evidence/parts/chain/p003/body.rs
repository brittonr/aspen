
pub fn chain_append_receipt_value(link: &ChainLink, head_before: Option<&str>, predicate_receipt_ref: &str) -> IoValue {
    let mut checks = vec![
        record("check", vec![string("canonical-link-ref"), string("pass")]),
        record("check", vec![string("payload-ref-available"), string("pass")]),
        record("check", vec![string("immutable-ledger-artifact"), string("pass")]),
        record("check", vec![string("head-advance"), string("pass")]),
        record("check", vec![string("no-unexpected-fork"), string("pass")]),
    ];
    if head_before.is_some() {
        checks.push(record("check", vec![string("append-valid"), string("pass")]));
    } else {
        checks.push(record("check", vec![string("genesis-valid"), string("pass")]));
    }

    record("chain-append-receipt-v1", vec![
        string(EVIDENCE_CHAIN_APPEND_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        chain_record(&link.chain),
        record("link", vec![string(&link.link_ref)]),
        record("payload", vec![string(&link.payload.artifact_ref)]),
        record("head-before", vec![optional_ref_value(head_before)]),
        record("head-after", vec![string(&link.link_ref)]),
        record("predicates", vec![ref_sequence_value(&[predicate_receipt_ref.to_string()])]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_anchor_value(
    chain: &ChainScope,
    link_ref: &str,
    policy_refs: &[String],
    producer: &ChainProducer,
) -> IoValue {
    record("chain-anchor-v1", vec![
        string(EVIDENCE_CHAIN_ANCHOR_SCHEMA),
        chain_record(chain),
        record("anchor", vec![string(link_ref)]),
        record("policy", vec![ref_sequence_value(policy_refs)]),
        producer_record(producer),
        record("checks", vec![sequence(vec![
            record("check", vec![string("trusted-anchor"), string("pass")]),
            record("check", vec![string("anchor-link-available"), string("pass")]),
        ])]),
    ])
}

pub fn publish_chain_anchor(
    root: &Path,
    chain: &ChainScope,
    link_ref: &str,
    policy_refs: &[String],
    producer: &ChainProducer,
) -> Result<ChainAnchor> {
    let link_value = crate::ledger::read_artifact(root, link_ref)?;
    let link = parse_chain_link(&link_value)?;
    if &link.chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "anchor link {link_ref} belongs to {:?}, expected {:?}",
            link.chain, chain
        )));
    }
    for policy_ref in policy_refs {
        require_ref(policy_ref, "anchor policy ref")?;
    }
    validate_producer(producer)?;
    let value = chain_anchor_value(chain, link_ref, policy_refs, producer);
    let imported = crate::ledger::import_artifact(root, &value)?;
    parse_chain_anchor(&crate::ledger::read_artifact(root, &imported.artifact_ref)?)
}

pub fn chain_checkpoint_value(input: &ChainCheckpointInput) -> IoValue {
    record("chain-checkpoint-v1", vec![
        string(EVIDENCE_CHAIN_CHECKPOINT_SCHEMA),
        chain_record(&input.chain),
        record("prior-checkpoint", vec![optional_ref_value(input.prior_checkpoint_ref.as_deref())]),
        record("range", vec![
            record("anchor", vec![string(&input.anchor_link_ref)]),
            record("head", vec![string(&input.head_ref)]),
            record("verify-receipt", vec![string(&input.verify_receipt_ref)]),
            record("predicate", vec![string(&input.range_predicate_ref)]),
        ]),
        record("policy", vec![ref_sequence_value(&input.policy_refs)]),
        record("membership", vec![ref_sequence_value(&input.membership_refs)]),
        record("control-plane", vec![
            record("mode", vec![string("trellis-raft")]),
            record("command", vec![string("accept-chain-head")]),
        ]),
        producer_record(&input.producer),
        record("checks", vec![sequence(input.checks.iter().map(check_value).collect())]),
    ])
}

pub fn accept_chain_checkpoint(root: &Path, input: &ChainCheckpointInput) -> Result<ChainCheckpoint> {
    validate_checkpoint_input(root, input)?;
    let value = chain_checkpoint_value(input);
    let imported = crate::ledger::import_artifact(root, &value)?;
    parse_chain_checkpoint(&crate::ledger::read_artifact(root, &imported.artifact_ref)?)
}

pub fn validate_chain_checkpoint_freshness(
    root: &Path,
    chain: &ChainScope,
    checkpoint_ref: &str,
    expected_head: Option<&str>,
) -> Result<ChainCheckpoint> {
    let value = crate::ledger::read_artifact(root, checkpoint_ref)?;
    let checkpoint = parse_chain_checkpoint(&value)?;
    if &checkpoint.chain != chain {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint {checkpoint_ref} belongs to {:?}, expected {:?}",
            checkpoint.chain, chain
        )));
    }
    if let Some(expected_head) = expected_head
        && checkpoint.head_ref != expected_head
    {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint head {} does not match expected head {expected_head}",
            checkpoint.head_ref
        )));
    }
    let index = build_chain_index(root)?;
    let heads = index.heads_for_chain(chain);
    if heads != vec![checkpoint.head_ref.clone()] {
        return Err(MoltenError::invalid_harness(format!(
            "checkpoint {checkpoint_ref} is stale: checkpoint head {}, current heads {:?}",
            checkpoint.head_ref, heads
        )));
    }
    Ok(checkpoint)
}

pub fn verify_chain_segment(
    root: &Path,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
) -> Result<ChainVerify> {
    verify_chain_segment_with_policy(root, chain, anchor_ref, expected_head, ChainForkPolicy::RejectUnexpectedForks)
}

pub fn verify_chain_segment_with_policy(
    root: &Path,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    expected_head: Option<&str>,
    fork_policy: ChainForkPolicy,
) -> Result<ChainVerify> {
    let index = build_chain_index(root)?;
    let chain_refs = index.links_for_chain(chain);
    let discovered_heads = index.heads_for_chain(chain);
    let mut diagnostics = Vec::new();

    if chain_refs.is_empty() {
        diagnostics.push(ChainDiagnostic::new("missing-chain", "no links found for requested chain", Vec::new()));
    }

    note_anchor(&index, chain, anchor_ref, &mut diagnostics);

    let selected_head = select_head_for_verification(expected_head, &discovered_heads, &index, chain, &mut diagnostics);
    detect_forks_sequence_conflicts_and_store_evidence(ForkDetectionInput {
        root,
        index: &index,
        chain,
        chain_refs: &chain_refs,
        discovered_heads: &discovered_heads,
        selected_head: selected_head.as_deref(),
        fork_policy,
        diagnostics: &mut diagnostics,
    })?;

    let (verified_links, payload_refs) = if let Some(head_ref) = selected_head {
        let walked = walk_links(
            LinkWalkInput {
                root,
                index: &index,
                chain,
                anchor_ref,
                head_ref: &head_ref,
            },
            &mut diagnostics,
        )?;
        (walked.verified_links, walked.payload_refs)
    } else {
        (Vec::new(), Vec::new())
    };
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic_is_fatal(diagnostic, fork_policy)) {
        "fail"
    } else {
        "pass"
    }
    .to_string();
    let predicate_receipt_refs = store_segment_predicate_receipts(SegmentPredicateReceiptsInput {
        root,
        decision: &decision,
        chain,
        anchor_ref,
        expected_head,
        discovered_heads: &discovered_heads,
        verified_links: &verified_links,
        payload_refs: &payload_refs,
        diagnostics: &diagnostics,
        fork_policy,
    })?;
    finish_verify(FinishInput {
        root,
        decision,
        chain,
        anchor_ref,
        expected_head,
        discovered_heads,
        verified_links,
        payload_refs,
        diagnostics,
        predicate_receipt_refs,
        fork_policy,
    })
}

fn note_anchor(
    index: &ChainIndex,
    chain: &ChainScope,
    anchor_ref: Option<&str>,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) {
    if let Some(anchor_ref) = anchor_ref {
        match index.links_by_ref.get(anchor_ref) {
            Some(anchor) if &anchor.chain == chain => {}
            Some(_) => diagnostics.push_item(ChainDiagnostic::new(
                "anchor-chain-mismatch",
                "anchor link belongs to a different chain scope/id/epoch",
                vec![anchor_ref.to_string()],
            )),
            None => diagnostics.push_item(ChainDiagnostic::new(
                "missing-anchor",
                "anchor link is unavailable in the ledger",
                vec![anchor_ref.to_string()],
            )),
        }
    }
}

fn walk_links(
    input: LinkWalkInput<'_>,
    diagnostics: &mut impl crate::bounded::VecSink<ChainDiagnostic>,
) -> Result<WalkOutput> {
    ensure_count_at_most(input.index.links_by_ref.len(), MAX_EVIDENCE_CHAIN_LINKS, "evidence chain links")?;
    let mut state = WalkState::default();
    let mut current_ref = input.head_ref.to_string();
    for _ in 0..=input.index.links_by_ref.len() {
        let Some(previous_ref) = walk_next(input, &current_ref, &mut state, diagnostics)? else {
            break;
        };
        current_ref = previous_ref;
    }
    if state.reverse_segment.len() > input.index.links_by_ref.len() {
        diagnostics.push_item(ChainDiagnostic::new(
            "chain-bound",
            "chain segment traversal exceeded available link count",
            vec![input.head_ref.to_string()],
        ));
    }
    let verified_links = state.reverse_segment.into_iter().rev().collect::<Vec<_>>();
    validate_verified_segment(input.index, input.anchor_ref, &verified_links, diagnostics);
    Ok(WalkOutput {
        verified_links,
        payload_refs: state.payload_refs.into_iter().collect(),
    })
}
