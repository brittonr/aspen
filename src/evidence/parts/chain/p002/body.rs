
pub fn append_checks() -> Vec<ChainCheck> {
    vec![
        ChainCheck::pass("same-chain-scope"),
        ChainCheck::pass("previous-link-binding"),
        ChainCheck::pass("sequence-monotonicity"),
        ChainCheck::pass("payload-ref-binding"),
    ]
}

pub fn chain_link_value(input: &ChainLinkInput) -> IoValue {
    let context_refs = input
        .context_refs
        .iter()
        .map(|context| record("ref", vec![string(&context.label), string(&context.artifact_ref)]))
        .collect();
    let checks = input
        .checks
        .iter()
        .map(|check| record("check", vec![string(&check.name), string(&check.decision)]))
        .collect();

    record("chain-link-v1", vec![
        string(EVIDENCE_CHAIN_LINK_SCHEMA),
        record("chain", vec![
            record("scope", vec![string(&input.chain.scope)]),
            record("id", vec![string(&input.chain.id)]),
            record("epoch", vec![string(&input.chain.epoch)]),
        ]),
        record("seq", vec![u64_value(input.sequence)]),
        record("prev", vec![optional_ref_value(input.previous_link_ref.as_deref())]),
        record("payload", vec![
            record("kind", vec![string(&input.payload.kind)]),
            record("ref", vec![string(&input.payload.artifact_ref)]),
            record("schema", vec![string(&input.payload.schema)]),
        ]),
        record("context", vec![sequence(context_refs)]),
        record("producer", vec![
            record("id", vec![string(&input.producer.id)]),
            record("key", vec![string(&input.producer.key_ref)]),
        ]),
        record("trellis", vec![
            record("predicate", vec![string(&input.trellis.predicate)]),
            record("input", vec![string(&input.trellis.input_ref)]),
            record("decision", vec![string(&input.trellis.decision)]),
        ]),
        record("checks", vec![sequence(checks)]),
    ])
}

pub fn chain_link_ref(value: &IoValue) -> Result<String> {
    canonical_hash(value)
}

fn ensure_entry_ref(kind: &str, entry_ref: &str, parsed_ref: &str) -> Result<()> {
    if parsed_ref == entry_ref {
        return Ok(());
    }

    Err(MoltenError::invalid_harness(format!(
        "ledger {kind} ref mismatch: index entry {entry_ref} parsed as {parsed_ref}"
    )))
}

fn index_link_entry(index: &mut ChainIndex, entry_ref: &str, value: &IoValue) -> Result<()> {
    let link = parse_chain_link(value)?;
    ensure_entry_ref("chain-link", entry_ref, &link.link_ref)?;
    index.links_by_chain.entry(link.chain.clone()).or_default().insert(link.link_ref.clone());
    index
        .links_by_sequence
        .entry((link.chain.clone(), link.sequence))
        .or_default()
        .insert(link.link_ref.clone());
    index
        .links_by_payload
        .entry(link.payload.artifact_ref.clone())
        .or_default()
        .insert(link.link_ref.clone());
    if let Some(parent_ref) = &link.previous_link_ref {
        index.children_by_parent.entry(parent_ref.clone()).or_default().insert(link.link_ref.clone());
    }
    index.links_by_ref.insert(link.link_ref.clone(), link);
    Ok(())
}

fn index_predicate_entry(index: &mut ChainIndex, entry_ref: &str, value: &IoValue) -> Result<()> {
    let receipt = parse_chain_predicate_receipt(value)?;
    ensure_entry_ref("chain-predicate-receipt", entry_ref, &receipt.receipt_ref)?;
    index
        .predicate_receipts_by_predicate
        .entry(receipt.predicate.clone())
        .or_default()
        .insert(receipt.receipt_ref.clone());
    index.predicate_receipts_by_ref.insert(receipt.receipt_ref.clone(), receipt);
    Ok(())
}

fn index_fork_entry(index: &mut ChainIndex, entry_ref: &str, value: &IoValue) -> Result<()> {
    let fork = parse_chain_fork_evidence(value)?;
    ensure_entry_ref("chain-fork-evidence", entry_ref, &fork.evidence_ref)?;
    index
        .fork_evidence_by_chain
        .entry(fork.chain.clone())
        .or_default()
        .insert(fork.evidence_ref.clone());
    if let Some(parent_ref) = &fork.parent_ref {
        index
            .fork_evidence_by_parent
            .entry(parent_ref.clone())
            .or_default()
            .insert(fork.evidence_ref.clone());
    }
    index.fork_evidence_by_ref.insert(fork.evidence_ref.clone(), fork);
    Ok(())
}

fn index_anchor_entry(index: &mut ChainIndex, entry_ref: &str, value: &IoValue) -> Result<()> {
    let anchor = parse_chain_anchor(value)?;
    ensure_entry_ref("chain-anchor", entry_ref, &anchor.anchor_ref)?;
    index.anchors_by_chain.entry(anchor.chain.clone()).or_default().insert(anchor.anchor_ref.clone());
    index.anchor_links_by_chain.entry(anchor.chain.clone()).or_default().insert(anchor.link_ref.clone());
    index.anchors_by_ref.insert(anchor.anchor_ref.clone(), anchor);
    Ok(())
}

fn index_checkpoint_entry(index: &mut ChainIndex, entry_ref: &str, value: &IoValue) -> Result<()> {
    let checkpoint = parse_chain_checkpoint(value)?;
    ensure_entry_ref("chain-checkpoint", entry_ref, &checkpoint.checkpoint_ref)?;
    index
        .checkpoints_by_chain
        .entry(checkpoint.chain.clone())
        .or_default()
        .insert(checkpoint.checkpoint_ref.clone());
    index
        .checkpoint_heads_by_chain
        .entry(checkpoint.chain.clone())
        .or_default()
        .insert(checkpoint.head_ref.clone());
    index.checkpoints_by_ref.insert(checkpoint.checkpoint_ref.clone(), checkpoint);
    Ok(())
}

fn finish_heads(index: &mut ChainIndex) -> Result<()> {
    for (chain, links) in &index.links_by_chain {
        let mut heads = links.clone();
        for link_ref in links {
            let Some(link) = index.links_by_ref.get(link_ref) else {
                return Err(MoltenError::invalid_harness(format!("chain index missing link {link_ref}")));
            };
            if let Some(parent_ref) = &link.previous_link_ref {
                heads.remove(parent_ref);
            }
        }
        index.heads_by_chain.insert(chain.clone(), heads);
    }
    Ok(())
}

pub fn build_chain_index(root: &Path) -> Result<ChainIndex> {
    let mut index = ChainIndex::default();
    for entry in crate::ledger::list_artifacts(root)? {
        let value = crate::ledger::read_artifact(root, &entry.artifact_ref)?;
        match entry.artifact_kind.as_str() {
            "chain-link" => index_link_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-predicate-receipt" => index_predicate_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-fork-evidence" => index_fork_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-anchor" => index_anchor_entry(&mut index, &entry.artifact_ref, &value)?,
            "chain-checkpoint" => index_checkpoint_entry(&mut index, &entry.artifact_ref, &value)?,
            _ => {}
        }
    }

    finish_heads(&mut index)?;
    Ok(index)
}

fn prior_head(index: &ChainIndex, link: &ChainLink) -> Result<Option<String>> {
    if let Some(previous_ref) = &link.previous_link_ref {
        let previous = index.links_by_ref.get(previous_ref).ok_or_else(|| {
            MoltenError::invalid_harness(format!("append previous link {previous_ref} is unavailable in ledger"))
        })?;
        validate_append(previous, link)?;
        let children = index.children_for_parent(previous_ref);
        if !children.is_empty() {
            return Err(MoltenError::invalid_harness(format!(
                "unexpected fork for parent {previous_ref}: existing children {:?}",
                children
            )));
        }
        let current_heads = index.heads_for_chain(&link.chain);
        if current_heads != vec![previous_ref.clone()] {
            return Err(MoltenError::invalid_harness(format!(
                "stale chain head for {:?}: expected current head {}, found {:?}",
                link.chain, previous_ref, current_heads
            )));
        }
        ensure_sequence_unoccupied(index, link)?;
        return Ok(Some(previous_ref.clone()));
    }

    validate_genesis(link)?;
    let current_heads = index.heads_for_chain(&link.chain);
    if !current_heads.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "genesis append has stale chain head for {:?}: existing heads {:?}",
            link.chain, current_heads
        )));
    }
    ensure_sequence_unoccupied(index, link)?;
    Ok(None)
}

fn append_predicate_ref(root: &Path, link: &ChainLink, head_before: Option<&str>) -> Result<String> {
    let predicate = if head_before.is_some() {
        APPEND_VALID_PREDICATE
    } else {
        GENESIS_VALID_PREDICATE
    };
    let predicate_subject_refs = vec![link.link_ref.clone()];
    let predicate_input_refs = predicate_input_refs(link);
    let predicate_context_refs = predicate_context_refs(link);
    let predicate_checks = vec![
        ChainCheck::pass("trellis-bounded-predicate"),
        ChainCheck::pass("predicate-decision-binding"),
    ];
    store_chain_predicate_receipt(StoreChainPredicateReceiptInput {
        root,
        receipt: ChainPredicateReceiptValueInput {
            predicate,
            decision: "pass",
            subject_refs: &predicate_subject_refs,
            input_refs: &predicate_input_refs,
            context_refs: &predicate_context_refs,
            checks: &predicate_checks,
        },
    })
}

pub fn append_chain_link(root: &Path, value: &IoValue) -> Result<ChainAppend> {
    let link = parse_chain_link(value)?;
    let index = build_chain_index(root)?;
    if index.links_by_ref.contains_key(&link.link_ref) {
        return Err(MoltenError::invalid_harness(format!(
            "chain link {} is already present in the ledger",
            link.link_ref
        )));
    }
    crate::ledger::read_artifact(root, &link.payload.artifact_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "chain link payload {} is unavailable in ledger: {error}",
            link.payload.artifact_ref
        ))
    })?;

    let head_before = prior_head(&index, &link)?;
    let imported = crate::ledger::import_artifact(root, value)?;
    if imported.artifact_ref != link.link_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain link ref mismatch: got {}, expected {}",
            imported.artifact_ref, link.link_ref
        )));
    }

    let predicate_receipt_ref = append_predicate_ref(root, &link, head_before.as_deref())?;
    let receipt_value = chain_append_receipt_value(&link, head_before.as_deref(), &predicate_receipt_ref);
    let receipt_ref = canonical_hash(&receipt_value)?;
    let imported_receipt = crate::ledger::import_artifact(root, &receipt_value)?;
    if imported_receipt.artifact_ref != receipt_ref {
        return Err(MoltenError::invalid_harness(format!(
            "imported chain append receipt ref mismatch: got {}, expected {receipt_ref}",
            imported_receipt.artifact_ref
        )));
    }

    Ok(ChainAppend {
        link_ref: link.link_ref.clone(),
        payload_ref: link.payload.artifact_ref.clone(),
        head_before,
        head_after: link.link_ref,
        predicate_receipt_ref,
        receipt_ref,
        receipt_value,
    })
}
