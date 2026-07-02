
fn chain_ends(link_refs: &[String]) -> Result<Ends> {
    let head_ref = link_refs
        .last()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage requires at least one chain link"))?;
    let anchor_ref = link_refs
        .first()
        .cloned()
        .ok_or_else(|| MoltenError::invalid_harness("chunk lineage requires at least one chain link"))?;
    Ok(Ends { head_ref, anchor_ref })
}

struct PredicateInput<'a> {
    manifest: &'a ChunkManifest,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
}

fn predicate_set(input: PredicateInput<'_>) -> Vec<IoValue> {
    let context_refs = vec![
        input.manifest.manifest_ref.clone(),
        input.manifest.root_ref.clone(),
        input.manifest.metadata_ref.clone(),
    ];
    let segment_checks = vec![
        crate::evidence_chain::ChainCheck::pass("segment-contiguity"),
        crate::evidence_chain::ChainCheck::pass("canonical-link-order"),
    ];
    let fork_checks = vec![
        crate::evidence_chain::ChainCheck::pass("fork-policy-profile"),
        crate::evidence_chain::ChainCheck::pass("fork-evidence-binding"),
    ];
    let anchor_subject_refs = vec![input.ends.anchor_ref.clone(), input.ends.head_ref.clone()];
    let anchor_checks = vec![
        crate::evidence_chain::ChainCheck::pass("anchor-descent"),
        crate::evidence_chain::ChainCheck::pass("head-binding"),
    ];
    let checkpoint_checks = vec![
        crate::evidence_chain::ChainCheck::pass("checkpoint-range-coverage"),
        crate::evidence_chain::ChainCheck::pass("verified-range"),
    ];
    vec![
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &segment_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
            decision: "pass",
            subject_refs: std::slice::from_ref(&input.ends.head_ref),
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &fork_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
            decision: "pass",
            subject_refs: &anchor_subject_refs,
            input_refs: input.link_refs,
            context_refs: &context_refs,
            checks: &anchor_checks,
        }),
        crate::evidence_chain::chain_predicate_receipt_value(&crate::evidence_chain::ChainPredicateReceiptValueInput {
            predicate: crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
            decision: "pass",
            subject_refs: input.link_refs,
            input_refs: input.receipt_refs,
            context_refs: &context_refs,
            checks: &checkpoint_checks,
        }),
    ]
}

struct VerifyInput<'a> {
    chain: &'a crate::evidence_chain::ChainScope,
    link_refs: &'a [String],
    receipt_refs: &'a [String],
    ends: &'a Ends,
    predicate_refs: &'a [String],
}

fn verify_value(input: VerifyInput<'_>) -> IoValue {
    let verify_diagnostics = Vec::new();
    let verify_receipt = crate::evidence_chain::ChainVerifyReceiptValueInput {
        decision: "pass",
        chain: input.chain,
        anchor_ref: Some(&input.ends.anchor_ref),
        expected_head: Some(&input.ends.head_ref),
        discovered_heads: std::slice::from_ref(&input.ends.head_ref),
        verified_links: input.link_refs,
        payload_refs: input.receipt_refs,
        diagnostics: &verify_diagnostics,
    };
    crate::evidence_chain::chain_verify_receipt_value_with_policy(
        &crate::evidence_chain::ChainVerifyReceiptPolicyValueInput {
            receipt: verify_receipt,
            predicate_receipt_refs: input.predicate_refs,
            fork_policy: crate::evidence_chain::ChainForkPolicy::RejectUnexpectedForks,
        },
    )
}

pub fn parse_chunk_lineage_value(value: &IoValue) -> Result<ChunkLineage> {
    let fields = simple_record(value, "chunk-lineage-v1", 8)?;
    require_schema(&fields[0], CHUNK_LINEAGE_SCHEMA, "chunk lineage")?;
    let manifest_ref = record_string(&fields[1], "manifest")?;
    let root_ref = record_string(&fields[2], "root")?;
    filename_for_ref(&manifest_ref)?;
    filename_for_ref(&root_ref)?;
    let link_values = record_sequence(&fields[3], "links")?;
    let receipt_values = record_sequence(&fields[4], "receipts")?;
    let verify_receipt_value = lineage_record_value(&fields[5], "verify-receipt")?;
    let predicate_values = record_sequence(&fields[6], "predicates")?;
    let checks = parse_lineage_checks(&fields[7])?;
    require_lineage_check(&checks, "manifest-root-binding")?;
    require_lineage_check(&checks, "receipt-payload-binding")?;
    require_lineage_check(&checks, "lineage-no-global-head")?;
    require_lineage_check(&checks, "lineage-continuity")?;
    require_lineage_check(&checks, "lineage-predicate-receipts")?;
    if link_values.is_empty() || link_values.len() != receipt_values.len() {
        return Err(MoltenError::invalid_harness(
            "chunk lineage must contain matching non-empty link and receipt sequences",
        ));
    }

    let receipts = receipt_values
        .iter()
        .map(|receipt_value| parse_receipt_value(receipt_value, None))
        .collect::<Result<Vec<_>>>()?;
    let entries = parsed_entries(EntryInput {
        manifest_ref: &manifest_ref,
        root_ref: &root_ref,
        link_values: &link_values,
        receipts: &receipts,
    })?;

    let predicates = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?;
    let predicate_receipt_refs = predicates.iter().map(|predicate| predicate.receipt_ref.clone()).collect::<Vec<_>>();
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE)?;
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE)?;
    require_chunk_lineage_predicate(&predicates, crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE)?;
    let range_predicate =
        require_chunk_lineage_predicate(&predicates, crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE)?;
    if range_predicate.subject_refs.as_slice() != entries.link_refs.as_slice()
        || range_predicate.input_refs.as_slice() != entries.receipt_refs.as_slice()
    {
        return Err(MoltenError::invalid_harness(
            "chunk lineage range predicate does not bind lineage links and receipts",
        ));
    }
    validate_chunk_lineage_verify_receipt(
        &verify_receipt_value,
        &entries.first_chain,
        &entries.link_refs,
        &entries.receipt_refs,
        &predicate_receipt_refs,
    )?;
    let verify_receipt_ref = canonical_hash(&verify_receipt_value)?;
    Ok(ChunkLineage {
        lineage_ref: canonical_hash(value)?,
        manifest_ref,
        root_ref,
        link_refs: entries.link_refs,
        receipt_refs: entries.receipt_refs,
        verify_receipt_ref,
        predicate_receipt_refs,
        value: value.clone(),
    })
}

struct EntryInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    link_values: &'a [IoValue],
    receipts: &'a [ChunkStoreReceipt],
}

struct ParsedEntries {
    first_chain: crate::evidence_chain::ChainScope,
    link_refs: Vec<String>,
    receipt_refs: Vec<String>,
}

fn parsed_entries(input: EntryInput<'_>) -> Result<ParsedEntries> {
    let mut first_chain = None;
    let mut link_refs = Vec::with_capacity(input.link_values.len());
    let mut receipt_refs = Vec::with_capacity(input.receipts.len());
    for (position, (link_value, receipt)) in input.link_values.iter().zip(input.receipts.iter()).enumerate() {
        let previous_ref = if position == 0 {
            None
        } else {
            link_refs.get(position - 1).map(String::as_str)
        };
        let entry = checked_entry(LinkInput {
            manifest_ref: input.manifest_ref,
            root_ref: input.root_ref,
            position,
            previous_ref,
            value: link_value,
            receipt,
        })?;
        if position == 0 {
            first_chain = Some(entry.chain);
        }
        push_bounded(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        push_bounded(&mut link_refs, entry.link_ref, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage link refs")?;
    }
    let first_chain = first_chain.ok_or_else(|| MoltenError::invalid_harness("chunk lineage missing first link"))?;
    Ok(ParsedEntries {
        first_chain,
        link_refs,
        receipt_refs,
    })
}

struct LinkInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    position: usize,
    previous_ref: Option<&'a str>,
    value: &'a IoValue,
    receipt: &'a ChunkStoreReceipt,
}

struct CheckedEntry {
    chain: crate::evidence_chain::ChainScope,
    link_ref: String,
}

fn checked_entry(input: LinkInput<'_>) -> Result<CheckedEntry> {
    if input.receipt.manifest_ref.as_deref() != Some(input.manifest_ref) || input.receipt.decision != "pass" {
        return Err(MoltenError::invalid_harness(
            "chunk lineage receipt does not bind the lineage manifest as pass evidence",
        ));
    }
    let link = crate::evidence_chain::parse_chain_link(input.value)?;
    if link.chain.scope != "chunk-lineage" || link.chain.id != input.manifest_ref || link.chain.epoch != input.root_ref
    {
        return Err(MoltenError::invalid_harness("chunk lineage link scope must be per manifest/root, not global"));
    }
    if link.sequence != input.position as u64 {
        return Err(MoltenError::invalid_harness("chunk lineage link sequence is not contiguous"));
    }
    if input.position == 0 {
        if link.previous_link_ref.is_some() {
            return Err(MoltenError::invalid_harness("chunk lineage genesis link must not name a previous link"));
        }
    } else if link.previous_link_ref.as_deref() != input.previous_ref {
        return Err(MoltenError::invalid_harness("chunk lineage link does not bind previous lineage receipt"));
    }
    if link.payload.artifact_ref != input.receipt.receipt_ref || link.payload.schema != CHUNK_STORE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(
            "chunk lineage link payload does not bind embedded chunk-store receipt",
        ));
    }
    require_lineage_context(&link.context_refs, "manifest", input.manifest_ref)?;
    require_lineage_context(&link.context_refs, "chunk-root", input.root_ref)?;
    for chunk_ref in &input.receipt.chunk_refs {
        require_lineage_context(&link.context_refs, "chunk", chunk_ref)?;
    }
    Ok(CheckedEntry {
        chain: link.chain,
        link_ref: link.link_ref,
    })
}

fn lineage_operation_rank(operation: &str) -> (u8, &str) {
    let rank = match operation {
        "manifest-create" => 0,
        "verify" => 1,
        "fetch" => 2,
        "iroh-publish" => 3,
        "iroh-fetch" => 4,
        "range-read" => 5,
        "dedup-hit" => 6,
        "pin" => 7,
        "unpin" => 8,
        "gc" => 9,
        _ => 100,
    };
    (rank, operation)
}
