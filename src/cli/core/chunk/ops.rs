#[path = "ops/metadata.rs"]
mod metadata;
#[path = "ops/object.rs"]
mod object;

type FilePath = std::path::PathBuf;
type Outcome<T> = molten::error::Result<T>;

pub(super) fn run(command: super::Top) -> Outcome<()> {
    match command {
        super::Top::Put { .. } => object::put(command),
        super::Top::Verify { .. } => object::verify(command),
        super::Top::Read { .. } => object::read(command),
        super::Top::Range { .. } => object::range(command),
        super::Top::Sync { .. } => object::sync(command),
        super::Top::IrohPublish { .. } => object::iroh_publish(command),
        super::Top::IrohFetch { .. } => object::iroh_fetch(command),
        super::Top::Pin { .. } => metadata::pin(command),
        super::Top::Unpin { .. } => metadata::unpin(command),
        super::Top::PinChunk { .. } => metadata::pin_one(command),
        super::Top::UnpinChunk { .. } => metadata::unpin_one(command),
        super::Top::IndexStatus { .. } => metadata::index_status(command),
        super::Top::IndexRebuild { .. } => metadata::index_rebuild(command),
        super::Top::ReceiptList { .. } => metadata::receipt_list(command),
        super::Top::ReceiptShow { .. } => metadata::receipt_show(command),
        super::Top::Lineage { .. } => metadata::lineage(command),
        super::Top::Gc { .. } => object::gc(command),
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

fn wrong_handler(name: &'static str) -> Outcome<()> {
    Err(molten::error::MoltenError::invalid_harness(format!(
        "chunk {name} handler received a mismatched command"
    )))
}
