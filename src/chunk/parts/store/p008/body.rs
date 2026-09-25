pub fn list_manifest_refs(root: &Path) -> Result<Vec<String>> {
    let root = open_capability_chunk_root(root)?;
    list_manifest_refs_with_root(&root)
}

pub fn list_manifest_refs_with_root(root: &CapabilityChunkRoot) -> Result<Vec<String>> {
    refs_from_dir(root, &store_path("manifests")?)
}

pub fn list_chunk_refs(root: &Path) -> Result<Vec<String>> {
    let root = open_capability_chunk_root(root)?;
    list_chunk_refs_with_root(&root)
}

pub fn list_chunk_refs_with_root(root: &CapabilityChunkRoot) -> Result<Vec<String>> {
    refs_from_dir(root, &store_path("chunks")?)
}

pub fn list_receipt_refs(root: &Path) -> Result<Vec<String>> {
    let root = open_capability_chunk_root(root)?;
    list_receipt_refs_with_root(&root)
}

pub fn list_receipt_refs_with_root(root: &CapabilityChunkRoot) -> Result<Vec<String>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let mut refs = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (key, _value) = item.map_err(index_error)?;
        push_bounded(&mut refs, key.value().to_string(), MAX_CHUNK_STORE_RECEIPTS, "chunk store receipt refs")?;
    }
    refs.sort();
    Ok(refs)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ChunkStoreReceipt> {
    let root = open_capability_chunk_root(root)?;
    read_receipt_with_root(&root, receipt_ref)
}

pub fn read_receipt_with_root(root: &CapabilityChunkRoot, receipt_ref: &str) -> Result<ChunkStoreReceipt> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(index_error)? else {
        return Err(Failure::invalid_harness(format!("unknown chunk store receipt {receipt_ref}")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_receipt_value(&value, Some(receipt_ref))
}

pub fn build_chunk_lineage(root: &Path, manifest_ref: &str) -> Result<ChunkLineage> {
    let root = open_capability_chunk_root(root)?;
    build_chunk_lineage_with_root(&root, manifest_ref)
}

pub fn build_chunk_lineage_with_root(root: &CapabilityChunkRoot, manifest_ref: &str) -> Result<ChunkLineage> {
    let manifest = read_manifest_with_root(root, manifest_ref)?;
    let mut receipts = list_receipt_refs_with_root(root)?
        .into_iter()
        .map(|receipt_ref| read_receipt_with_root(root, &receipt_ref))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|receipt| receipt.decision == "pass" && receipt.manifest_ref.as_deref() == Some(manifest_ref))
        .collect::<Vec<_>>();
    receipts.sort_by(|left, right| {
        lineage_operation_rank(&left.operation)
            .cmp(&lineage_operation_rank(&right.operation))
            .then_with(|| left.receipt_ref.cmp(&right.receipt_ref))
    });
    if receipts.is_empty() {
        return Err(Failure::invalid_harness(format!(
            "no pass chunk-store receipts available for lineage manifest {manifest_ref}"
        )));
    }

    let chain = crate::evidence_chain::ChainScope::new(
        "chunk-lineage",
        manifest.manifest_ref.clone(),
        manifest.root_ref.clone(),
    );
    let producer = lineage_producer()?;
    let series = link_series(&manifest, &receipts, &chain, &producer)?;
    let evidence = pass_evidence(&chain, &manifest, &series.refs, &series.receipt_refs)?;
    let value = lineage_value(&LineageValueInput {
        manifest_ref: &manifest.manifest_ref,
        root_ref: &manifest.root_ref,
        link_values: &series.values,
        receipt_values: &series.receipt_values,
        verify_receipt_value: &evidence.verify_value,
        predicate_values: &evidence.predicate_values,
    });
    let lineage_ref = canonical_hash(&value)?;
    Ok(ChunkLineage {
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
    values: Vec<IoValue>,
    receipt_refs: Vec<String>,
    receipt_values: Vec<IoValue>,
}

fn link_series(
    manifest: &ChunkManifest,
    receipts: &[ChunkStoreReceipt],
    chain: &crate::evidence_chain::ChainScope,
    producer: &crate::evidence_chain::ChainProducer,
) -> Result<LinkSeries> {
    ensure_count_at_most(receipts.len(), MAX_CHUNK_STORE_RECEIPTS, "chunk lineage receipts")?;
    let mut links = Vec::with_capacity(receipts.len());
    let mut values = Vec::with_capacity(receipts.len());
    let mut receipt_refs = Vec::with_capacity(receipts.len());
    let mut receipt_values = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        let payload = crate::evidence_chain::ChainPayload::new(
            "chunk-store-receipt",
            receipt.receipt_ref.clone(),
            CHUNK_STORE_RECEIPT_SCHEMA,
        );
        let trellis_input_ref = canonical_hash(&record("chunk-lineage-input", vec![
            string(&manifest.manifest_ref),
            string(&manifest.root_ref),
            string(&receipt.receipt_ref),
            string(&receipt.operation),
        ]))?;
        let input = if let Some(previous) = links.last() {
            crate::evidence_chain::ChainLinkInput::append(
                previous,
                payload,
                lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        } else {
            crate::evidence_chain::ChainLinkInput::genesis(
                chain.clone(),
                payload,
                lineage_context_refs(manifest, receipt)?,
                producer.clone(),
                trellis_input_ref,
            )
        };
        let link_value = crate::evidence_chain::chain_link_value(&input);
        let link = crate::evidence_chain::parse_chain_link(&link_value)?;
        push_bounded(
            &mut receipt_refs,
            receipt.receipt_ref.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt refs",
        )?;
        push_bounded(
            &mut receipt_values,
            receipt.value.clone(),
            MAX_CHUNK_STORE_RECEIPTS,
            "chunk lineage receipt values",
        )?;
        push_bounded(&mut values, link_value, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage link values")?;
        push_bounded(&mut links, link, MAX_CHUNK_STORE_RECEIPTS, "chunk lineage links")?;
    }
    Ok(LinkSeries {
        refs: links.iter().map(|link| link.link_ref.clone()).collect(),
        values,
        receipt_refs,
        receipt_values,
    })
}

struct PassEvidence {
    predicate_values: Vec<IoValue>,
    predicate_refs: Vec<String>,
    verify_value: IoValue,
    verify_ref: String,
}

struct Ends {
    head_ref: String,
    anchor_ref: String,
}

fn pass_evidence(
    chain: &crate::evidence_chain::ChainScope,
    manifest: &ChunkManifest,
    link_refs: &[String],
    receipt_refs: &[String],
) -> Result<PassEvidence> {
    let ends = chain_ends(link_refs)?;
    let predicate_values = predicate_set(PredicateInput {
        manifest,
        link_refs,
        receipt_refs,
        ends: &ends,
    });
    let predicate_refs = predicate_values
        .iter()
        .map(crate::evidence_chain::parse_chain_predicate_receipt)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|receipt| receipt.receipt_ref)
        .collect::<Vec<_>>();
    let verify_value = verify_value(VerifyInput {
        chain,
        link_refs,
        receipt_refs,
        ends: &ends,
        predicate_refs: &predicate_refs,
    });
    let verify_ref = canonical_hash(&verify_value)?;
    Ok(PassEvidence {
        predicate_values,
        predicate_refs,
        verify_value,
        verify_ref,
    })
}
