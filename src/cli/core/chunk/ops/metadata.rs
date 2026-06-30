pub(super) fn pin(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Pin {
        manifest_ref,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("pin");
    };
    let pin = molten::chunk_store::pin_manifest(&store, &manifest_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
    println!("chunk pin ok manifest={} store={}", manifest_ref, store.display());
    Ok(())
}

pub(super) fn unpin(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Unpin {
        manifest_ref,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("unpin");
    };
    let pin = molten::chunk_store::unpin_manifest(&store, &manifest_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
    println!("chunk unpin ok manifest={} store={}", manifest_ref, store.display());
    Ok(())
}

pub(super) fn pin_one(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::PinChunk {
        chunk_ref,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("pin chunk");
    };
    let pin = molten::chunk_store::pin_chunk(&store, &chunk_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
    println!("chunk pin-chunk ok chunk={} store={}", chunk_ref, store.display());
    Ok(())
}

pub(super) fn unpin_one(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::UnpinChunk {
        chunk_ref,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("unpin chunk");
    };
    let pin = molten::chunk_store::unpin_chunk(&store, &chunk_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
    println!("chunk unpin-chunk ok chunk={} store={}", chunk_ref, store.display());
    Ok(())
}

pub(super) fn index_status(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::IndexStatus { store } = command else {
        return super::wrong_handler("index status");
    };
    let status = molten::chunk_store::index_status(&store)?;
    println!(
        "chunk index status manifests={} chunks={} available={} missing={} manifest_pins={} chunk_pins={} partial_fetches={} receipts={} store={}",
        status.manifests,
        status.chunks,
        status.available_chunks,
        status.missing_chunks,
        status.manifest_pins,
        status.chunk_pins,
        status.partial_fetches,
        status.receipts,
        store.display()
    );
    Ok(())
}

pub(super) fn index_rebuild(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::IndexRebuild { store, receipt_out } = command else {
        return super::wrong_handler("index rebuild");
    };
    let rebuild = molten::chunk_store::rebuild_index(&store)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &rebuild.receipt_value)?;
    println!(
        "chunk index rebuild ok manifests={} chunks={} available={} missing={} receipts={} store={}",
        rebuild.status.manifests,
        rebuild.status.chunks,
        rebuild.status.available_chunks,
        rebuild.status.missing_chunks,
        rebuild.status.receipts,
        store.display()
    );
    Ok(())
}

pub(super) fn receipt_list(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::ReceiptList { store } = command else {
        return super::wrong_handler("receipt list");
    };
    let refs = molten::chunk_store::list_receipt_refs(&store)?;
    for receipt_ref in &refs {
        println!("{receipt_ref}");
    }
    println!("chunk receipt-list ok receipts={} store={}", refs.len(), store.display());
    Ok(())
}

pub(super) fn receipt_show(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::ReceiptShow { receipt_ref, store } = command else {
        return super::wrong_handler("receipt show");
    };
    let receipt = molten::chunk_store::read_receipt(&store, &receipt_ref)?;
    println!("{}", molten::preserves_rail::to_text(&receipt.value)?);
    eprintln!(
        "chunk receipt-show ok receipt={} operation={} decision={} chunks={} store={}",
        receipt.receipt_ref,
        receipt.operation,
        receipt.decision,
        receipt.chunk_refs.len(),
        store.display()
    );
    Ok(())
}

pub(super) fn lineage(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Lineage {
        manifest_ref,
        store,
        lineage_out,
    } = command
    else {
        return super::wrong_handler("lineage");
    };
    let lineage = molten::chunk_store::build_chunk_lineage(&store, &manifest_ref)?;
    super::emit_named_receipt(lineage_out.as_ref(), "chunk lineage", &lineage.value)?;
    println!(
        "chunk lineage ok lineage={} manifest={} links={} receipts={} predicates={}",
        lineage.lineage_ref,
        lineage.manifest_ref,
        lineage.link_refs.len(),
        lineage.receipt_refs.len(),
        lineage.predicate_receipt_refs.len()
    );
    Ok(())
}
