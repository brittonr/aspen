//! The local Iroh ticket record and verified blob readback.

const CHUNK_IROH_TICKET_SCHEMA: &str = "molten.chunk-store.iroh-ticket.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IrohChunkBlob {
    pub(super) chunk_ref: String,
    pub(super) blob_ref: String,
    pub(super) length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IrohChunkTicket {
    pub(super) manifest_ref: String,
    pub(super) manifest_blob_ref: String,
    pub(super) chunks: Vec<IrohChunkBlob>,
}

pub(super) fn iroh_ticket_value(
    manifest_ref: &str,
    manifest_blob_ref: &str,
    chunks: &[IrohChunkBlob],
) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-store-iroh-ticket-v1", vec![
        crate::preserves_rail::string(CHUNK_IROH_TICKET_SCHEMA),
        crate::preserves_rail::record("adapter", vec![crate::preserves_rail::string("iroh-local")]),
        crate::preserves_rail::record("manifest-ref", vec![crate::preserves_rail::string(manifest_ref)]),
        crate::preserves_rail::record("manifest-blob-ref", vec![crate::preserves_rail::string(manifest_blob_ref)]),
        crate::preserves_rail::record("chunks", vec![crate::preserves_rail::sequence(
            chunks
                .iter()
                .map(|chunk| {
                    crate::preserves_rail::record("chunk-blob", vec![
                        crate::preserves_rail::string(&chunk.chunk_ref),
                        crate::preserves_rail::string(&chunk.blob_ref),
                        crate::preserves_rail::u64_value(chunk.length),
                    ])
                })
                .collect(),
        )]),
    ])
}

pub(super) fn parse_iroh_ticket_value(value: &preserves::IOValue) -> crate::error::Result<IrohChunkTicket> {
    let fields = crate::blocks::wire::fields::simple_record(value, "chunk-store-iroh-ticket-v1", 5)?;
    crate::blocks::wire::fields::require_schema(&fields[0], CHUNK_IROH_TICKET_SCHEMA, "chunk-store Iroh ticket")?;
    let adapter = crate::blocks::wire::fields::record_string(&fields[1], "adapter")?;
    if adapter != "iroh-local" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported chunk-store Iroh adapter {adapter}")));
    }
    let manifest_ref = crate::blocks::wire::fields::record_string(&fields[2], "manifest-ref")?;
    crate::blocks::layout::filename_for_ref(&manifest_ref)?;
    let manifest_blob_ref = crate::blocks::wire::fields::record_string(&fields[3], "manifest-blob-ref")?;
    crate::blocks::layout::filename_for_ref(&manifest_blob_ref)?;
    let chunk_values = crate::blocks::wire::fields::record_sequence(&fields[4], "chunks")?;
    let mut chunks = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for chunk_value in &chunk_values {
        let chunk_blob = crate::blocks::wire::fields::simple_record(chunk_value, "chunk-blob", 3)?;
        let chunk_ref = crate::blocks::wire::fields::required_string(&chunk_blob[0], "chunk ref")?;
        let blob_ref = crate::blocks::wire::fields::required_string(&chunk_blob[1], "blob ref")?;
        let length = crate::blocks::wire::fields::required_u64(&chunk_blob[2], "chunk blob length")?;
        crate::blocks::layout::filename_for_ref(&chunk_ref)?;
        crate::blocks::layout::filename_for_ref(&blob_ref)?;
        if length == 0 {
            return Err(crate::error::Failure::invalid_harness(format!(
                "Iroh chunk ticket maps {chunk_ref} to zero-length blob"
            )));
        }
        if !crate::blocks::limits::insert_set_bounded(
            &mut seen,
            chunk_ref.clone(),
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunk set",
        )? {
            return Err(crate::error::Failure::invalid_harness(format!(
                "Iroh chunk ticket has duplicate chunk mapping for {chunk_ref}"
            )));
        }
        crate::bounded::push(
            &mut chunks,
            IrohChunkBlob {
                chunk_ref,
                blob_ref,
                length,
            },
            crate::blocks::limits::MAX_CHUNK_STORE_CHUNKS,
            "chunk store Iroh ticket chunks",
        )?;
    }
    Ok(IrohChunkTicket {
        manifest_ref,
        manifest_blob_ref,
        chunks,
    })
}

pub(super) fn read_iroh_ticket(
    root: &crate::blocks::CapabilityChunkRoot,
    manifest_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    let bytes = root.root().read(&crate::blocks::layout::iroh_ticket_path(manifest_ref)?)?;
    crate::preserves_rail::parse_canonical_bytes(&bytes)
}

pub(super) fn read_iroh_blob(
    root: &crate::blocks::CapabilityChunkRoot,
    blob_ref: &str,
) -> crate::error::Result<Vec<u8>> {
    let bytes = root.root().read(&crate::blocks::layout::iroh_blob_path(blob_ref)?)?;
    let actual_ref = crate::blocks::integrity::hash_blob_bytes(&bytes);
    if actual_ref != blob_ref {
        return Err(crate::error::Failure::invalid_harness(format!("Iroh blob {blob_ref} hashes to {actual_ref}")));
    }
    Ok(bytes)
}
