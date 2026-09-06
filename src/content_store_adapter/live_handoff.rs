use super::*;
use crate::preserves_rail as rail;

pub const MAX_LIVE_HANDOFF_BYTES: usize = 262_144;
const MAX_LIVE_TICKET_BYTES: usize = 2_048;
const HANDOFF_RECORD: &str = "live-content-handoff-v1";
const LOCATOR_RECORD: &str = "live-content-locator-v1";

/// An admitted public locator set. Contains no server router, store, or key.
/// The canonical manifest remains the content owner; these are transport hints.
#[derive(Debug, Clone)]
pub struct LiveIrohRemote {
    pub(super) manifest: ContentManifestDescriptor,
    pub(super) locators: Vec<LiveChunkLocator>,
    pub(super) backend_hint_ref: String,
}

impl LiveIrohRemote {
    pub fn manifest(&self) -> &ContentManifestDescriptor {
        &self.manifest
    }
}

impl LiveIrohPublication {
    // r[impl molten.content_live.handoff]
    pub fn export_handoff(&self) -> Result<Vec<u8>> {
        let provider = self.router.endpoint().id().to_string();
        let locators = self
            .locators
            .iter()
            .map(|locator| {
                rail::record(
                    LOCATOR_RECORD,
                    vec![
                        rail::string(locator.position.to_string()),
                        rail::string(&locator.chunk_ref),
                        rail::string(locator.ticket.to_string()),
                    ],
                )
            })
            .collect();
        let value = rail::record(
            HANDOFF_RECORD,
            vec![
                self.manifest_value.clone(),
                rail::string(&self.backend_hint_ref),
                rail::string(provider),
                rail::sequence(locators),
            ],
        );
        let bytes = rail::canonical_bytes(&value)?;
        if bytes.len() > MAX_LIVE_HANDOFF_BYTES {
            return Err(MoltenError::invalid_harness("live handoff exceeds byte bound"));
        }
        Ok(bytes)
    }
}

/// The expected manifest, provider, and socket are separate caller admissions.
/// Tickets cannot redirect a client to arbitrary host addresses or relay URLs.
pub struct LiveHandoffExpectation<'a> {
    pub manifest_ref: &'a str,
    pub provider: iroh::EndpointId,
    pub address: SocketAddr,
}

// r[impl molten.content_live.handoff]
pub fn admit_live_handoff(
    profile: &ContentAdapterProfile,
    bytes: &[u8],
    expected: LiveHandoffExpectation<'_>,
) -> Result<LiveIrohRemote> {
    if bytes.is_empty()
        || bytes.len() > MAX_LIVE_HANDOFF_BYTES
        || profile.class != ContentAdapterClass::IrohBlobs
        || !validate_content_profile(profile).is_empty()
        || expected.address.ip().is_unspecified()
        || expected.address.port() == 0
    {
        return Err(MoltenError::invalid_harness("live handoff input/profile/address denied"));
    }
    let value = rail::parse_canonical_bytes(bytes)?;
    let fields = rail::simple_record_fields(&value, HANDOFF_RECORD, 4)?;
    let source =
        crate::chunk_store::parse_manifest_value(&rail::value_to_iovalue(&fields[0]), Some(expected.manifest_ref))?;
    let manifest = manifest_descriptor(&source);
    if !validate_manifest_descriptor(&manifest).is_empty()
        || manifest.total_length > profile.bounds.max_total_bytes
        || manifest.total_length > profile.bounds.max_memory_bytes
        || manifest.chunks.len() > profile.bounds.max_chunk_count
        || manifest
            .chunks
            .iter()
            .any(|chunk| chunk.length > profile.bounds.max_chunk_bytes || chunk.transform != "identity")
    {
        return Err(MoltenError::invalid_harness("live handoff manifest bounds denied"));
    }
    let backend_hint_ref = rail::required_content_ref_string(&fields[1], "backend hint ref")?;
    if rail::required_string_field(&fields[2], "provider")? != expected.provider.to_string() {
        return Err(MoltenError::invalid_harness("live handoff provider mismatch"));
    }
    let entries = rail::required_sequence_field(&fields[3], "locators")?;
    if entries.len() != manifest.chunks.len() {
        return Err(MoltenError::invalid_harness("live handoff locator count mismatch"));
    }
    let address = iroh::EndpointAddr::new(expected.provider).with_ip_addr(expected.address);
    let mut locators = Vec::with_capacity(entries.len());
    for (position, entry) in entries.iter().enumerate() {
        let entry = rail::value_to_iovalue(entry);
        let fields = rail::simple_record_fields(&entry, LOCATOR_RECORD, 3)?;
        let chunk_ref = rail::required_content_ref_string(&fields[1], "locator chunk")?;
        if rail::required_string_field(&fields[0], "locator position")? != position.to_string()
            || chunk_ref != manifest.chunks[position].chunk_ref
        {
            return Err(MoltenError::invalid_harness("live handoff locator order/membership mismatch"));
        }
        let text = rail::required_string_field(&fields[2], "blob ticket")?;
        if text.len() > MAX_LIVE_TICKET_BYTES {
            return Err(MoltenError::invalid_harness("live handoff ticket exceeds byte bound"));
        }
        let ticket: BlobTicket = text.parse().map_err(iroh_error)?;
        if ticket.addr().id != expected.provider || ticket.format() != BlobFormat::Raw {
            return Err(MoltenError::invalid_harness("live handoff ticket provider/format mismatch"));
        }
        locators.push(LiveChunkLocator {
            chunk_ref,
            position,
            ticket: BlobTicket::new(address.clone(), ticket.hash(), BlobFormat::Raw),
        });
    }
    Ok(LiveIrohRemote {
        manifest,
        locators,
        backend_hint_ref,
    })
}
