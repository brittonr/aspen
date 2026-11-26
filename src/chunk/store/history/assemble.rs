//! Lineage construction: orders a manifest's pass receipts and links them into an evidence chain.

pub fn build_chunk_lineage(
    root: &std::path::Path,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkLineage> {
    let root = crate::blocks::open_capability_chunk_root(root)?;
    build_chunk_lineage_with_root(&root, manifest_ref)
}

pub fn build_chunk_lineage_with_root(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<crate::blocks::ChunkLineage> {
    let manifest = crate::blocks::read_manifest_with_root(root, manifest_ref)?;
    let mut receipts = crate::blocks::list_receipt_refs_with_root(root)?
        .into_iter()
        .map(|receipt_ref| crate::blocks::read_receipt_with_root(root, &receipt_ref))
        .collect::<crate::error::Result<Vec<_>>>()?
        .into_iter()
        .filter(|receipt| receipt.decision == "pass" && receipt.manifest_ref.as_deref() == Some(manifest_ref))
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        lineage_operation_rank(&left.operation)
            .cmp(&lineage_operation_rank(&right.operation))
            .then_with(|| left.receipt_ref.cmp(&right.receipt_ref))
    });
    if receipts.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!(
            "no pass chunk-store receipts available for lineage manifest {manifest_ref}"
        )));
    }

    let chain = crate::evidence_chain::ChainScope::new(
        "chunk-lineage",
        manifest.manifest_ref.clone(),
        manifest.root_ref.clone(),
    );
    let producer = crate::blocks::history::scan::lineage_producer()?;
    let series = link_series(&manifest, &receipts, &chain, &producer)?;
    let evidence =
        crate::blocks::history::attest::pass_evidence(&chain, &manifest, &series.refs, &series.receipt_refs)?;
    let value = crate::blocks::history::binding::lineage_value(&crate::blocks::history::binding::LineageValueInput {
        manifest_ref: &manifest.manifest_ref,
        root_ref: &manifest.root_ref,
        link_values: &series.values,
        receipt_values: &series.receipt_values,
        verify_receipt_value: &evidence.verify_value,
        predicate_values: &evidence.predicate_values,
    });
    let lineage_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(crate::blocks::ChunkLineage {
        lineage_ref,
        manifest_ref: manifest.manifest_ref,
        root_ref: manifest.root_ref,
        link_refs: series.refs,
        receipt_refs: series.receipt_refs,
        verify_receipt_ref: evidence.verify_ref,
        predicate_receipt_refs: evidence.predicate_refs,
        value,
    })
}

struct LinkSeries {
    refs: Vec<String>,
    values: Vec<preserves::IOValue>,
    receipt_refs: Vec<String>,
    receipt_values: Vec<preserves::IOValue>,
}

fn link_series(
    manifest: &crate::blocks::ChunkManifest,
    receipts: &[crate::blocks::ChunkReceipt],
    chain: &crate::evidence_chain::ChainScope,
    producer: &crate::evidence_chain::ChainProducer,
) -> crate::error::Result<LinkSeries> {
    crate::bounded::ensure_count_at_most(
        receipts.len(),
        crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
        "chunk lineage receipts",
    )?;
    let mut links = Vec::with_capacity(receipts.len());
    let mut values = Vec::with_capacity(receipts.len());
    let mut receipt_refs = Vec::with_capacity(receipts.len());
    let mut receipt_values = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let payload = crate::evidence_chain::ChainPayload::new(
            "chunk-store-receipt",
            receipt.receipt_ref.clone(),
            crate::preserves_rail::CHUNK_STORE_RECEIPT_SCHEMA,
        );
        let trellis_input_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-lineage-input", vec![
                crate::preserves_rail::string(&manifest.manifest_ref),
                crate::preserves_rail::string(&manifest.root_ref),
                crate::preserves_rail::string(&receipt.receipt_ref),
                crate::preserves_rail::string(&receipt.operation),
            ]))?;
        let input = if let Some(previous) = links.last() {
            crate::evidence_chain::ChainLinkInput::append(
                previous,
                payload,
                crate::blocks::history::scan::lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        } else {
            crate::evidence_chain::ChainLinkInput::genesis(
                chain.clone(),
                payload,
                crate::blocks::history::scan::lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        };
        let link_value = crate::evidence_chain::chain_link_value(&input);
        let link = crate::evidence_chain::parse_chain_link(&link_value)?;
        crate::bounded::push(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        crate::bounded::push(
            &mut receipt_values,
            receipt.value.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt values",
        )?;
        crate::bounded::push(
            &mut values,
            link_value,
            crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage link values",
        )?;
        crate::bounded::push(&mut links, link, crate::blocks::limits::MAX_CHUNK_STORE_RECEIPTS, "chunk lineage links")?;
    }
    Ok(LinkSeries {
        refs: links.iter().map(|link| link.link_ref.clone()).collect(),
        values,
        receipt_refs,
        receipt_values,
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
