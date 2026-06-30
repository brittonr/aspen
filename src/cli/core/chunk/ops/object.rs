pub(super) fn put(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Put {
        input,
        store,
        kind,
        chunk_size,
        manifest_out,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("put");
    };
    let bytes = super::read_bytes(&input)?;
    let put = molten::chunk_store::put_bytes(&store, &kind, &bytes, chunk_size)?;
    if let Some(path) = manifest_out.as_ref() {
        super::super::io::write_file(path, &molten::preserves_rail::to_text(&put.manifest_value)?)?;
    }
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &put.receipt_value)?;
    println!(
        "chunk put ok manifest={} chunks={} bytes={} store={}",
        put.manifest_ref,
        put.chunk_refs.len(),
        put.total_len,
        store.display()
    );
    Ok(())
}

pub(super) fn verify(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Verify {
        manifest_ref,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("verify");
    };
    let verified = molten::chunk_store::verify_manifest(&store, &manifest_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &verified.receipt_value)?;
    println!(
        "chunk verify ok manifest={} chunks={} bytes={}",
        verified.manifest_ref,
        verified.chunk_refs.len(),
        verified.total_len
    );
    Ok(())
}

pub(super) fn read(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Read {
        manifest_ref,
        store,
        out,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("read");
    };
    let read = molten::chunk_store::read_object(&store, &manifest_ref)?;
    super::write_bytes(&out, &read.bytes)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
    println!("chunk read ok manifest={} bytes={} out={}", read.manifest_ref, read.bytes.len(), out.display());
    Ok(())
}

pub(super) fn range(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Range {
        manifest_ref,
        store,
        offset,
        length,
        out,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("range");
    };
    let read = molten::chunk_store::range_read(&store, &manifest_ref, offset, length)?;
    super::write_bytes(&out, &read.bytes)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
    println!(
        "chunk range ok manifest={} offset={} length={} out={}",
        read.manifest_ref,
        read.offset,
        read.length,
        out.display()
    );
    Ok(())
}

pub(super) fn sync(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Sync {
        manifest_ref,
        from,
        store,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("sync");
    };
    let sync = molten::chunk_store::sync_missing_chunks(&from, &store, &manifest_ref)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &sync.receipt_value)?;
    println!(
        "chunk sync ok manifest={} missing_before={} fetched={}",
        sync.manifest_ref,
        sync.missing_before.len(),
        sync.fetched_chunks.len()
    );
    Ok(())
}

pub(super) fn iroh_publish(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::IrohPublish {
        manifest_ref,
        store,
        iroh_store,
        node,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("iroh publish");
    };
    let published = molten::chunk_store::publish_iroh_blobs(&store, &iroh_store, &manifest_ref, &node)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &published.receipt_value)?;
    println!(
        "chunk iroh-publish ok manifest={} chunks={} ticket={} iroh_store={}",
        published.manifest_ref,
        published.chunk_blob_refs.len(),
        published.ticket,
        iroh_store.display()
    );
    Ok(())
}

pub(super) fn iroh_fetch(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::IrohFetch {
        ticket,
        iroh_store,
        store,
        expected_manifest_ref,
        peer,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("iroh fetch");
    };
    let fetched =
        molten::chunk_store::fetch_iroh_blobs(&iroh_store, &store, &ticket, expected_manifest_ref.as_deref(), &peer)?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &fetched.receipt_value)?;
    println!(
        "chunk iroh-fetch ok manifest={} missing_before={} fetched={} store={}",
        fetched.manifest_ref,
        fetched.missing_before.len(),
        fetched.fetched_chunks.len(),
        store.display()
    );
    Ok(())
}

pub(super) fn gc(command: super::super::Top) -> super::Outcome<()> {
    let super::super::Top::Gc {
        store,
        dry_run,
        apply_refs,
        retention,
        receipt_out,
    } = command
    else {
        return super::wrong_handler("gc");
    };
    let retention_evidence = retention.into_retention_evidence();
    let gc = molten::chunk_store::gc(&store, molten::chunk_store::ChunkStoreGcInput {
        dry_run,
        retention_evidence: &retention_evidence,
        apply_refs: &apply_refs,
    })?;
    super::emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &gc.receipt_value)?;
    println!(
        "chunk gc ok decision={} dry_run={} removed_manifests={} removed_chunks={} retention_receipts={}",
        gc.decision,
        gc.dry_run,
        gc.removed_manifests.len(),
        gc.removed_chunks.len(),
        gc.retention_receipt_refs.len()
    );
    Ok(())
}
