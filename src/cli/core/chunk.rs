use std::fs;
use std::path::Path;
use std::path::PathBuf;

use clap::Subcommand;
use molten::chunk_store;
use molten::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
use molten::error::MoltenError;
use molten::error::Result;
use molten::preserves_rail::canonical_hash;
use molten::preserves_rail::to_text;

use crate::RetentionEvidenceArgs;

#[derive(Debug, Subcommand)]
pub(crate) enum ChunkCommand {
    Put {
        input: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long, default_value = "artifact")]
        kind: String,
        #[arg(long, default_value_t = DEFAULT_FIXED_V1_CHUNK_SIZE)]
        chunk_size: u64,
        #[arg(long)]
        manifest_out: Option<PathBuf>,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Verify {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Read {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Range {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        offset: u64,
        #[arg(long)]
        length: u64,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Sync {
        manifest_ref: String,
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IrohPublish {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long, default_value = "node:local")]
        node: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IrohFetch {
        ticket: String,
        #[arg(long)]
        iroh_store: PathBuf,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        expected_manifest_ref: Option<String>,
        #[arg(long, default_value = "peer:local")]
        peer: String,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Pin {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    Unpin {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    PinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    UnpinChunk {
        chunk_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    IndexStatus {
        #[arg(long)]
        store: PathBuf,
    },
    IndexRebuild {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
    ReceiptList {
        #[arg(long)]
        store: PathBuf,
    },
    ReceiptShow {
        receipt_ref: String,
        #[arg(long)]
        store: PathBuf,
    },
    Lineage {
        manifest_ref: String,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        lineage_out: Option<PathBuf>,
    },
    Gc {
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long = "apply-ref")]
        apply_refs: Vec<String>,
        #[command(flatten)]
        retention: RetentionEvidenceArgs,
        #[arg(long)]
        receipt_out: Option<PathBuf>,
    },
}

pub(crate) fn run_chunk_command(command: ChunkCommand) -> Result<()> {
    match command {
        ChunkCommand::Put {
            input,
            store,
            kind,
            chunk_size,
            manifest_out,
            receipt_out,
        } => {
            let bytes = fs::read(&input).map_err(MoltenError::from)?;
            let put = chunk_store::put_bytes(&store, &kind, &bytes, chunk_size)?;
            if let Some(path) = manifest_out.as_ref() {
                write_file(path, &to_text(&put.manifest_value)?)?;
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
        ChunkCommand::Verify {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let verified = chunk_store::verify_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &verified.receipt_value)?;
            println!(
                "chunk verify ok manifest={} chunks={} bytes={}",
                verified.manifest_ref,
                verified.chunk_refs.len(),
                verified.total_len
            );
            Ok(())
        }
        ChunkCommand::Read {
            manifest_ref,
            store,
            out,
            receipt_out,
        } => {
            let read = chunk_store::read_object(&store, &manifest_ref)?;
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(MoltenError::from)?;
            }
            fs::write(&out, &read.bytes).map_err(MoltenError::from)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &read.receipt_value)?;
            println!("chunk read ok manifest={} bytes={} out={}", read.manifest_ref, read.bytes.len(), out.display());
            Ok(())
        }
        ChunkCommand::Range {
            manifest_ref,
            store,
            offset,
            length,
            out,
            receipt_out,
        } => {
            let read = chunk_store::range_read(&store, &manifest_ref, offset, length)?;
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(MoltenError::from)?;
            }
            fs::write(&out, &read.bytes).map_err(MoltenError::from)?;
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
        ChunkCommand::Sync {
            manifest_ref,
            from,
            store,
            receipt_out,
        } => {
            let sync = chunk_store::sync_missing_chunks(&from, &store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &sync.receipt_value)?;
            println!(
                "chunk sync ok manifest={} missing_before={} fetched={}",
                sync.manifest_ref,
                sync.missing_before.len(),
                sync.fetched_chunks.len()
            );
            Ok(())
        }
        ChunkCommand::IrohPublish {
            manifest_ref,
            store,
            iroh_store,
            node,
            receipt_out,
        } => {
            let published = chunk_store::publish_iroh_blobs(&store, &iroh_store, &manifest_ref, &node)?;
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
        ChunkCommand::IrohFetch {
            ticket,
            iroh_store,
            store,
            expected_manifest_ref,
            peer,
            receipt_out,
        } => {
            let fetched =
                chunk_store::fetch_iroh_blobs(&iroh_store, &store, &ticket, expected_manifest_ref.as_deref(), &peer)?;
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
        ChunkCommand::Pin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::pin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        ChunkCommand::Unpin {
            manifest_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::unpin_manifest(&store, &manifest_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin ok manifest={} store={}", manifest_ref, store.display());
            Ok(())
        }
        ChunkCommand::PinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::pin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk pin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        ChunkCommand::UnpinChunk {
            chunk_ref,
            store,
            receipt_out,
        } => {
            let pin = chunk_store::unpin_chunk(&store, &chunk_ref)?;
            emit_named_receipt(receipt_out.as_ref(), "chunk store receipt", &pin.receipt_value)?;
            println!("chunk unpin-chunk ok chunk={} store={}", chunk_ref, store.display());
            Ok(())
        }
        ChunkCommand::IndexStatus { store } => {
            let status = chunk_store::index_status(&store)?;
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
        ChunkCommand::IndexRebuild { store, receipt_out } => {
            let rebuild = chunk_store::rebuild_index(&store)?;
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
        ChunkCommand::ReceiptList { store } => {
            let refs = chunk_store::list_receipt_refs(&store)?;
            for receipt_ref in &refs {
                println!("{receipt_ref}");
            }
            println!("chunk receipt-list ok receipts={} store={}", refs.len(), store.display());
            Ok(())
        }
        ChunkCommand::ReceiptShow { receipt_ref, store } => {
            let receipt = chunk_store::read_receipt(&store, &receipt_ref)?;
            println!("{}", to_text(&receipt.value)?);
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
        ChunkCommand::Lineage {
            manifest_ref,
            store,
            lineage_out,
        } => {
            let lineage = chunk_store::build_chunk_lineage(&store, &manifest_ref)?;
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
        ChunkCommand::Gc {
            store,
            dry_run,
            apply_refs,
            retention,
            receipt_out,
        } => {
            let retention_evidence = retention.into_retention_evidence();
            let gc = chunk_store::gc(&store, chunk_store::ChunkStoreGcInput {
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

fn emit_named_receipt(path: Option<&PathBuf>, label: &str, receipt: &preserves::IOValue) -> Result<()> {
    let receipt_text = to_text(receipt)?;
    let receipt_ref = canonical_hash(receipt)?;
    if let Some(path) = path {
        write_file(path, &receipt_text)?;
        println!("{label} {receipt_ref} written to {}", path.display());
    } else {
        println!("{receipt_text}");
        eprintln!("{label} {receipt_ref}");
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(MoltenError::from)?;
    }
    fs::write(path, contents).map_err(MoltenError::from)
}
