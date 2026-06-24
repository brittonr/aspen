type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: super::Top) -> Outcome<()> {
    match command {
        super::Top::Put {
            input,
            store,
            kind,
            chunk_size,
            manifest_out,
            receipt_out,
        } => {
            let bytes = read_bytes(&input)?;
            let put = molten::chunk_store::put_bytes(&store, &kind, &bytes, chunk_size)?;
            if let Some(path) = manifest_out.as_ref() {
                super::io::write_file(path, &molten::preserves_rail::to_text(&put.manifest_value)?)?;
            }
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &put.receipt_value)?;
            println!(
                "chunk put ok manifest={} chunks={} bytes={} store={}",
                put.manifest_ref,
                put.chunk_refs.len(),
                put.total_len,
                store.display()
            );
            Ok(())
        }
        super::Top::Verify {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let verified = molten::chunk_store::verify_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &verified.receipt_value)?;
            println!(
                "chunk verify ok manifest={} chunks={} bytes={}",
                verified.manifest_ref,
                verified.chunk_refs.len(),
                verified.total_len
            );
            Ok(())
        }
        super::Top::Read {
            manifest_ref,
            store,
            out,
            receipt_out,
        } => {
            let read = molten::chunk_store::read_object(&store, &manifest_ref)?;
            write_bytes(&out, &read.bytes)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
            println!("chunk read ok manifest={} bytes={} out={}", read.manifest_ref, read.bytes.len(), out.display());
            Ok(())
        }
        super::Top::Range {
            manifest_ref,
            store,
            offset,
            length,
            out,
            receipt_out,
        } => {
            let read = molten::chunk_store::range_read(&store, &manifest_ref, offset, length)?;
            write_bytes(&out, &read.bytes)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
            println!(
                "chunk range ok manifest={} offset={} length={} out={}",
                read.manifest_ref,
                read.offset,
                read.length,
                out.display()
            );
            Ok(())
        }
        super::Top::Sync {
            manifest_ref,
            from,
            store,
            receipt_out,
        } => {
            let sync = molten::chunk_store::sync_missing_chunks(&from, &store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &sync.receipt_value)?;
            println!(
                "chunk sync ok manifest={} missing_before={} fetched={}",
                sync.manifest_ref,
                sync.missing_before.len(),
                sync.fetched_chunks.len()
            );
            Ok(())
        }
        super::Top::IrohPublish {
            manifest_ref,
            store,
            iroh_store,
            node,
            receipt_out,
        } => {
            let published = molten::chunk_store::publish_iroh_blobs(&store, &iroh_store, &manifest_ref, &node)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &published.receipt_value)?;
            println!(
                "chunk iroh-publish ok manifest={} chunks={} ticket={} iroh_store={}",
                published.manifest_ref,
                published.chunk_blob_refs.len(),
                published.ticket,
                iroh_store.display()
            );
            Ok(())
        }
        super::Top::IrohFetch {
            ticket,
            iroh_store,
            store,
            expected_manifest_ref,
            peer,
            receipt_out,
        } => {
            let fetched = molten::chunk_store::fetch_iroh_blobs(
                &iroh_store,
                &store,
                &ticket,
                expected_manifest_ref.as_deref(),
                &peer,
            )?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &fetched.receipt_value)?;
            println!(
                "chunk iroh-fetch ok manifest={} missing_before={} fetched={} store={}",
                fetched.manifest_ref,
                fetched.missing_before.len(),
                fetched.fetched_chunks.len(),
                store.display()
            );
            Ok(())
        }
        super::Top::Pin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = molten::chunk_store::pin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        super::Top::Unpin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = molten::chunk_store::unpin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        super::Top::PinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = molten::chunk_store::pin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        super::Top::UnpinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = molten::chunk_store::unpin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        super::Top::IndexStatus { store } => {
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
        super::Top::IndexRebuild { store, receipt_out } => {
            let rebuild = molten::chunk_store::rebuild_index(&store)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &rebuild.receipt_value)?;
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
        super::Top::ReceiptList { store } => {
            let refs = molten::chunk_store::list_receipt_refs(&store)?;
            for receipt_ref in &refs {
                println!("{receipt_ref}");
            }
            println!("chunk receipt-list ok receipts={} store={}", refs.len(), store.display());
            Ok(())
        }
        super::Top::ReceiptShow { receipt_ref, store } => {
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
        super::Top::Lineage {
            manifest_ref,
            store,
            lineage_out,
        } => {
            let lineage = molten::chunk_store::build_chunk_lineage(&store, &manifest_ref)?;
            emit_named_receipt(lineage_out.as_ref(), "chunk lineage", &lineage.value)?;
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
        super::Top::Gc {
            store,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = molten::chunk_store::gc(&store, molten::chunk_store::ChunkStoreGcInput {
                dry_run,
                retention_evidence: &retention_evidence,
                apply_refs: &apply_refs,
            })?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &gc.receipt_value)?;
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
    }
}

fn read_bytes(path: &std::path::Path) -> Outcome<Vec<u8>> {
    std::fs::read(path).map_err(molten::error::MoltenError::from)
}

fn write_bytes(path: &std::path::Path, bytes: &[u8]) -> Outcome<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(molten::error::MoltenError::from)?;
    }
    std::fs::write(path, bytes).map_err(molten::error::MoltenError::from)
}

fn emit_named_receipt(path: Option<&FilePath>, label: &str, receipt: &preserves::IOValue) -> Outcome<()> {
    let receipt_text = molten::preserves_rail::to_text(receipt)?;
    let receipt_ref = molten::preserves_rail::canonical_hash(receipt)?;
    if let Some(path) = path {
        super::io::write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}
