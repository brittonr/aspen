use std::time::Duration;

use bao_tree::io::BaoContentItem;
use iroh::protocol::Router;
use iroh_blobs::BlobFormat;
use iroh_blobs::BlobsProtocol;
use iroh_blobs::get::request::GetBlobItem;
use iroh_blobs::store::mem::MemStore;
use iroh_blobs::ticket::BlobTicket;
use molten_core::content_store_adapter::*;
use n0_future::StreamExt;

use super::*;
use crate::chunk_store::CapabilityChunkRoot;
use crate::error::MoltenError;
use crate::error::Result;
use crate::node_state::NodeStateNamespace;

#[derive(Debug, Clone)]
struct LiveChunkLocator {
    chunk_ref: String,
    position: usize,
    ticket: BlobTicket,
}

pub struct LiveIrohIdentity<'a> {
    pub namespace: &'a NodeStateNamespace,
    pub endpoint_id: &'a str,
    pub handle_ref: &'a str,
    pub backend_ref: &'a str,
}

pub struct LiveIrohPublication {
    router: Router,
    _store: MemStore,
    manifest: ContentManifestDescriptor,
    locators: Vec<LiveChunkLocator>,
    backend_hint_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveIrohContentExecution {
    pub state: CanonicalContentArtifact<ContentPartialState>,
    pub events: Vec<CanonicalContentArtifact<ContentEvent>>,
    pub verified_chunks: Vec<VerifiedChunkPayload>,
    pub backend_hint_ref: String,
}

impl LiveIrohPublication {
    pub fn manifest(&self) -> &ContentManifestDescriptor {
        &self.manifest
    }

    pub fn backend_hint_ref(&self) -> &str {
        &self.backend_hint_ref
    }

    #[cfg(test)]
    pub(crate) fn invalidate_first_locator(&mut self) {
        if let Some(locator) = self.locators.first_mut() {
            locator.ticket = BlobTicket::new(
                locator.ticket.addr().clone(),
                iroh_blobs::Hash::new(b"missing-live-iroh-blob"),
                BlobFormat::Raw,
            );
        }
    }

    pub async fn shutdown(self) -> Result<()> {
        self.router
            .shutdown()
            .await
            .map_err(|error| MoltenError::invalid_harness(format!("live Iroh blob router shutdown failed: {error}")))
    }
}

// r[impl molten.content_store_adapter.identity_boundary]
// r[impl molten.content_store_adapter.verify_before_available]
pub async fn publish_live_iroh_chunks(
    profile: &ContentAdapterProfile,
    root: &CapabilityChunkRoot,
    manifest_ref: &str,
    identity: LiveIrohIdentity<'_>,
) -> Result<LiveIrohPublication> {
    if profile.class != ContentAdapterClass::IrohBlobs {
        return Err(MoltenError::invalid_harness("live Iroh publication requires iroh-blobs adapter profile"));
    }
    let source_manifest = crate::chunk_store::read_manifest_with_root(root, manifest_ref)?;
    let manifest = manifest_descriptor(&source_manifest);
    let profile_issues = validate_content_profile(profile);
    if !profile_issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("live Iroh profile denied: {profile_issues:?}")));
    }
    let manifest_issues = validate_manifest_descriptor(&manifest);
    if !manifest_issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("live Iroh manifest denied: {manifest_issues:?}")));
    }
    if manifest.total_length > profile.bounds.max_total_bytes || manifest.chunks.len() > profile.bounds.max_chunk_count
    {
        return Err(MoltenError::invalid_harness("live Iroh publication exceeds adapter bounds"));
    }
    let secret_key = crate::fabric_crypto_identity::load_transport_secret_for_identity(
        identity.namespace,
        identity.endpoint_id,
        identity.handle_ref,
        identity.backend_ref,
    )?;
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(secret_key)
        .relay_mode(iroh::RelayMode::Disabled)
        .bind()
        .await
        .map_err(iroh_error)?;
    let store = MemStore::new();
    let mut locators = Vec::with_capacity(source_manifest.chunks.len());
    for (position, chunk) in source_manifest.chunks.iter().enumerate() {
        let manifest_chunk_size = usize::try_from(source_manifest.chunk_size)
            .map_err(|_| MoltenError::invalid_harness("live Iroh manifest chunk size does not fit usize"))?;
        let bytes = crate::chunk_store::read_verified_chunk(root, chunk, manifest_chunk_size)?;
        let tag = store
            .blobs()
            .add_bytes(bytes)
            .await
            .map_err(|error| MoltenError::invalid_harness(format!("live Iroh blob import failed: {error}")))?;
        locators.push(LiveChunkLocator {
            chunk_ref: chunk.chunk_ref.clone(),
            position,
            ticket: BlobTicket::new(endpoint.addr(), tag.hash, BlobFormat::Raw),
        });
    }
    let blobs = BlobsProtocol::new(&store, None);
    let router = Router::builder(endpoint).accept(iroh_blobs::ALPN, blobs).spawn();
    let backend_hint_ref = backend_hint_ref(
        ContentAdapterClass::IrohBlobs,
        &format!("{}\0{}", identity.endpoint_id, identity.backend_ref),
    );
    Ok(LiveIrohPublication {
        router,
        _store: store,
        manifest,
        locators,
        backend_hint_ref,
    })
}

// r[impl molten.content_store_adapter.verify_before_available]
// r[impl molten.content_store_adapter.live_sim_conformance]
pub async fn execute_live_iroh_stream_get(
    profile: &ContentAdapterProfile,
    publication: &LiveIrohPublication,
    command: &ContentCommand,
    generation: u64,
    retained: Option<&ContentPartialState>,
    timeout: Duration,
) -> Result<LiveIrohContentExecution> {
    if profile.class != ContentAdapterClass::IrohBlobs {
        return Err(MoltenError::invalid_harness("live Iroh get requires iroh-blobs adapter profile"));
    }
    let preflight = preflight_content_operation(profile, &publication.manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("live Iroh preflight denied: {:?}", preflight.issues)));
    }
    let mut state = begin_partial_state(profile, &publication.manifest, command, generation, retained)
        .map_err(|issues| MoltenError::invalid_harness(format!("live Iroh partial state denied: {issues:?}")))?;
    let client = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .bind()
        .await
        .map_err(iroh_error)?;
    let resume_position = state.verified_chunk_refs.len();
    let mut events = Vec::new();
    let mut verified_chunks = Vec::new();
    for locator in publication.locators.iter().skip(resume_position) {
        let Some(descriptor) = publication.manifest.chunks.get(locator.position) else {
            state =
                classify_content_failure(profile, &state, ContentFailure::AdapterFailure).map_err(transition_error)?;
            events.push(terminal_event(profile, command, &publication.manifest, &state, None)?);
            client.close().await;
            return finish_live_execution(profile, publication, state, events, verified_chunks);
        };
        if descriptor.chunk_ref != locator.chunk_ref {
            state = classify_content_failure(profile, &state, ContentFailure::StaleTicket).map_err(transition_error)?;
            events.push(terminal_event(
                profile,
                command,
                &publication.manifest,
                &state,
                Some(descriptor.chunk_ref.clone()),
            )?);
            client.close().await;
            return finish_live_execution(profile, publication, state, events, verified_chunks);
        }
        let connection = match tokio::time::timeout(
            timeout,
            client.connect(locator.ticket.addr().clone(), iroh_blobs::ALPN),
        )
        .await
        {
            Ok(Ok(connection)) => connection,
            Ok(Err(_)) => {
                state = classify_content_failure(profile, &state, ContentFailure::TransportDisconnected)
                    .map_err(transition_error)?;
                events.push(terminal_event(profile, command, &publication.manifest, &state, None)?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
            Err(_) => {
                state = classify_content_failure(profile, &state, ContentFailure::Timeout).map_err(transition_error)?;
                events.push(terminal_event(profile, command, &publication.manifest, &state, None)?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
        };
        let bytes = match tokio::time::timeout(
            timeout,
            receive_bounded_blob(connection, locator.ticket.hash(), profile.bounds.max_chunk_bytes),
        )
        .await
        {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(_)) => {
                state =
                    classify_content_failure(profile, &state, ContentFailure::StaleTicket).map_err(transition_error)?;
                events.push(terminal_event(
                    profile,
                    command,
                    &publication.manifest,
                    &state,
                    Some(descriptor.chunk_ref.clone()),
                )?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
            Err(_) => {
                state = classify_content_failure(profile, &state, ContentFailure::Timeout).map_err(transition_error)?;
                events.push(terminal_event(
                    profile,
                    command,
                    &publication.manifest,
                    &state,
                    Some(descriptor.chunk_ref.clone()),
                )?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
        };
        let sequence = next_sequence(&state)?;
        let observation = ContentChunkObservation {
            operation_ref: command.operation_ref.clone(),
            manifest_ref: publication.manifest.manifest_ref.clone(),
            sequence,
            chunk_ref: descriptor.chunk_ref.clone(),
            position: descriptor.position,
            observed_content_ref: crate::chunk_store::hash_chunk(
                &bytes,
                usize::try_from(publication.manifest.chunk_size)
                    .map_err(|_| MoltenError::invalid_harness("live Iroh chunk size does not fit usize"))?,
            ),
            observed_length: u64::try_from(bytes.len())
                .map_err(|_| MoltenError::invalid_harness("live Iroh chunk length does not fit u64"))?,
        };
        match apply_chunk_observation(profile, &publication.manifest, &state, &observation) {
            Ok(next) => state = next,
            Err(issues) => {
                let failure = verification_failure(&issues);
                state = classify_content_failure(profile, &state, failure).map_err(transition_error)?;
                events.push(terminal_event(
                    profile,
                    command,
                    &publication.manifest,
                    &state,
                    Some(descriptor.chunk_ref.clone()),
                )?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
        }
        events.push(canonical_content_event(
            profile,
            &content_event(
                command,
                sequence,
                state.terminal,
                Some(descriptor.chunk_ref.clone()),
                observation.observed_length,
                None,
                &publication.manifest.evidence_refs,
            ),
        )?);
        verified_chunks.push(VerifiedChunkPayload {
            chunk_ref: descriptor.chunk_ref.clone(),
            position: descriptor.position,
            bytes,
        });
    }
    client.close().await;
    if !content_is_available(&publication.manifest, &state) {
        return Err(MoltenError::invalid_harness("live Iroh transfer ended before full verification"));
    }
    finish_live_execution(profile, publication, state, events, verified_chunks)
}

fn finish_live_execution(
    profile: &ContentAdapterProfile,
    publication: &LiveIrohPublication,
    state: ContentPartialState,
    events: Vec<CanonicalContentArtifact<ContentEvent>>,
    verified_chunks: Vec<VerifiedChunkPayload>,
) -> Result<LiveIrohContentExecution> {
    Ok(LiveIrohContentExecution {
        state: canonical_partial_state(profile, &publication.manifest, &state)?,
        events,
        verified_chunks,
        backend_hint_ref: publication.backend_hint_ref.clone(),
    })
}

fn terminal_event(
    profile: &ContentAdapterProfile,
    command: &ContentCommand,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
    chunk_ref: Option<String>,
) -> Result<CanonicalContentArtifact<ContentEvent>> {
    let sequence = state
        .last_sequence
        .ok_or_else(|| MoltenError::invalid_harness("terminal live Iroh state lacks event sequence"))?;
    canonical_content_event(
        profile,
        &content_event(command, sequence, state.terminal, chunk_ref, 0, state.failure, &manifest.evidence_refs),
    )
}

fn verification_failure(issues: &[ContentIssue]) -> ContentFailure {
    if issues.iter().any(|issue| matches!(issue, ContentIssue::CorruptChunk(_))) {
        ContentFailure::CorruptChunk
    } else if issues.iter().any(|issue| matches!(issue, ContentIssue::TruncatedChunk(_))) {
        ContentFailure::TruncatedChunk
    } else if issues.iter().any(|issue| matches!(issue, ContentIssue::ReorderedChunk(_))) {
        ContentFailure::ReorderedChunk
    } else {
        ContentFailure::AdapterFailure
    }
}

fn transition_error(issue: ContentIssue) -> MoltenError {
    MoltenError::invalid_harness(format!("live Iroh transition denied: {issue:?}"))
}

async fn receive_bounded_blob(
    connection: iroh::endpoint::Connection,
    hash: iroh_blobs::Hash,
    maximum_bytes: u64,
) -> Result<Vec<u8>> {
    let maximum = usize::try_from(maximum_bytes)
        .map_err(|_| MoltenError::invalid_harness("live Iroh receive bound does not fit usize"))?;
    let mut bytes = Vec::new();
    let mut progress = iroh_blobs::get::request::get_blob(connection, hash);
    loop {
        match progress.next().await {
            Some(GetBlobItem::Item(BaoContentItem::Leaf(leaf))) => {
                let next_length = bytes
                    .len()
                    .checked_add(leaf.data.len())
                    .ok_or_else(|| MoltenError::invalid_harness("live Iroh receive length overflow"))?;
                if next_length > maximum {
                    return Err(MoltenError::invalid_harness("live Iroh blob exceeds admitted chunk bound"));
                }
                bytes.extend_from_slice(&leaf.data);
            }
            Some(GetBlobItem::Item(BaoContentItem::Parent(_))) => {}
            Some(GetBlobItem::Done(_)) => break,
            Some(GetBlobItem::Error(error)) => {
                return Err(MoltenError::invalid_harness(format!("live Iroh blob stream failed: {error}")));
            }
            None => return Err(MoltenError::invalid_harness("live Iroh blob stream ended without terminal item")),
        }
    }
    Ok(bytes)
}

fn next_sequence(state: &ContentPartialState) -> Result<u64> {
    state
        .last_sequence
        .map_or(Some(0), |sequence| sequence.checked_add(1))
        .ok_or_else(|| MoltenError::invalid_harness("live Iroh event sequence overflow"))
}

fn iroh_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("live Iroh content adapter failed: {error}"))
}
