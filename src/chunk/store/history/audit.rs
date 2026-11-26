//! Lineage parsing: re-checks every link, receipt, predicate, and verify receipt before accepting a
//! lineage.

pub fn parse_chunk_lineage_value(value: &preserves::IOValue) -> crate::error::Result<crate::blocks::ChunkLineage> {
    let fields = crate::blocks::wire::fields::simple_record(value, "chunk-lineage-v1", 8)?;
    crate::blocks::wire::fields::require_schema(
        &fields[0],
        crate::preserves_rail::CHUNK_LINEAGE_SCHEMA,
        "chunk lineage",
    )?;
    let manifest_ref = crate::blocks::wire::fields::record_string(&fields[1], "manifest")?;
    let root_ref = crate::blocks::wire::fields::record_string(&fields[2], "root")?;
    crate::blocks::layout::filename_for_ref(&manifest_ref)?;
    crate::blocks::layout::filename_for_ref(&root_ref)?;
    let link_values = crate::blocks::wire::fields::record_sequence(&fields[3], "links")?;
    let receipt_values = crate::blocks::wire::fields::record_sequence(&fields[4], "receipts")?;
    let verify_receipt_value = crate::blocks::history::binding::lineage_record_value(&fields[5], "verify-receipt")?;
    let predicate_values = crate::blocks::wire::fields::record_sequence(&fields[6], "predicates")?;
    let checks = crate::blocks::history::binding::parse_lineage_checks(&fields[7])?;
    crate::blocks::history::binding::require_lineage_check(&checks, "manifest-root-binding")?;
    crate::blocks::history::binding::require_lineage_check(&checks, "receipt-payload-binding")?;
    crate::blocks::history::binding::require_lineage_check(&checks, "lineage-no-global-head")?;
    crate::blocks::history::binding::require_lineage_check(&checks, "lineage-continuity")?;
    crate::blocks::history::binding::require_lineage_check(&checks, "lineage-predicate-receipts")?;
    if link_values.is_empty() || link_values.len() != receipt_values.len() {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage must contain matching non-empty link and receipt sequences",
        ));
    }

    let receipts = receipt_values
        .iter()
        .map(|receipt_value| crate::blocks::parse_receipt_value(receipt_value, None))
        .collect::<crate::error::Result<Vec<_>>>()?;
    let entries = parsed_entries(EntryInput {
        manifest_ref: &manifest_ref,
        root_ref: &root_ref,
        link_values: &link_values,
        receipts: &receipts,
    })?;

    let predicate_receipt_refs = bound_predicate_refs(&predicate_values, &entries)?;
    crate::blocks::history::binding::validate_chunk_lineage_verify_receipt(
        &verify_receipt_value,
        &entries.first_chain,
        &entries.link_refs,
        &entries.receipt_refs,
        &predicate_receipt_refs,
    )?;
    let verify_receipt_ref = crate::preserves_rail::canonical_hash(&verify_receipt_value)?;
    Ok(crate::blocks::ChunkLineage {
        lineage_ref: crate::preserves_rail::canonical_hash(value)?,
        manifest_ref,
        root_ref,
        link_refs: entries.link_refs,
        receipt_refs: entries.receipt_refs,
        verify_receipt_ref,
        predicate_receipt_refs,
        value: value.clone(),
    })
}

/// Parses the predicate receipts, requires the four passing lineage predicates, and requires the
/// range predicate to bind exactly the lineage links and receipts. Returns the predicate receipt
/// refs in lineage order.
fn bound_predicate_refs(
    predicate_values: &[preserves::IOValue],
    entries: &ParsedEntries,
) -> crate::error::Result<Vec<String>> {
    let predicates = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<crate::error::Result<Vec<_>>>()?;
    let predicate_receipt_refs = predicates.iter().map(|predicate| predicate.receipt_ref.clone()).collect::<Vec<_>>();
    crate::blocks::history::binding::require_chunk_lineage_predicate(
        &predicates,
        crate::evidence_chain::SEGMENT_NO_GAP_PREDICATE,
    )?;
    crate::blocks::history::binding::require_chunk_lineage_predicate(
        &predicates,
        crate::evidence_chain::SEGMENT_NO_FORK_PREDICATE,
    )?;
    crate::blocks::history::binding::require_chunk_lineage_predicate(
        &predicates,
        crate::evidence_chain::DESCENDS_FROM_ANCHOR_PREDICATE,
    )?;
    let range_predicate = crate::blocks::history::binding::require_chunk_lineage_predicate(
        &predicates,
        crate::evidence_chain::CHECKPOINT_COVERS_RANGE_PREDICATE,
    )?;
    if range_predicate.subject_refs.as_slice() != entries.link_refs.as_slice()
        || range_predicate.input_refs.as_slice() != entries.receipt_refs.as_slice()
    {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage range predicate does not bind lineage links and receipts",
        ));
    }
    Ok(predicate_receipt_refs)
}

struct EntryInput<'a> {
    manifest_ref: &'a str,
    root_ref: &'a str,
    link_values: &'a [preserves::IOValue],
    receipts: &'a [crate::blocks::ChunkReceipt],
}

struct ParsedEntries {
    first_chain: crate::evidence_chain::ChainScope,
    link_refs: Vec<String>,
    receipt_refs: Vec<String>,
}

fn parsed_entries(input: EntryInput<'_>) -> crate::error::Result<ParsedEntries> {
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
        crate::bounded::push(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        crate::bounded::push(
            &mut link_refs,
            entry.link_ref,
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage link refs",
        )?;
    }
    let first_chain =
        first_chain.ok_or_else(|| crate::error::Failure::invalid_harness("chunk lineage missing first link"))?;
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
    value: &'a preserves::IOValue,
    receipt: &'a crate::blocks::ChunkReceipt,
}

struct CheckedEntry {
    chain: crate::evidence_chain::ChainScope,
    link_ref: String,
}

fn checked_entry(input: LinkInput<'_>) -> crate::error::Result<CheckedEntry> {
    if input.receipt.manifest_ref.as_deref() != Some(input.manifest_ref) || input.receipt.decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage receipt does not bind the lineage manifest as pass evidence",
        ));
    }
    let link = crate::evidence_chain::parse_chain_link(input.value)?;
    if link.chain.scope != "chunk-lineage" || link.chain.id != input.manifest_ref || link.chain.epoch != input.root_ref
    {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage link scope must be per manifest/root, not global",
        ));
    }
    if link.sequence != input.position as u64 {
        return Err(crate::error::Failure::invalid_harness("chunk lineage link sequence is not contiguous"));
    }
    if input.position == 0 {
        if link.previous_link_ref.is_some() {
            return Err(crate::error::Failure::invalid_harness(
                "chunk lineage genesis link must not name a previous link",
            ));
        }
    } else if link.previous_link_ref.as_deref() != input.previous_ref {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage link does not bind previous lineage receipt",
        ));
    }
    if link.payload.artifact_ref != input.receipt.receipt_ref
        || link.payload.schema != crate::preserves_rail::CHUNK_STORE_RECEIPT_SCHEMA
    {
        return Err(crate::error::Failure::invalid_harness(
            "chunk lineage link payload does not bind embedded chunk-store receipt",
        ));
    }
    crate::blocks::history::binding::require_lineage_context(&link.context_refs, "manifest", input.manifest_ref)?;
    crate::blocks::history::binding::require_lineage_context(&link.context_refs, "chunk-root", input.root_ref)?;
    for chunk_ref in &input.receipt.chunk_refs {
        crate::blocks::history::binding::require_lineage_context(&link.context_refs, "chunk", chunk_ref)?;
    }
    Ok(CheckedEntry {
        chain: link.chain,
        link_ref: link.link_ref,
    })
}
