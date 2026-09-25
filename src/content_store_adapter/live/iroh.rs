use molten_core::content_store_adapter::*;
use n0_future::StreamExt;

use super::*;

#[derive(Debug, Clone)]
struct LiveChunkLocator {
    chunk_ref: String,
    position: usize,
    ticket: iroh_blobs::ticket::BlobTicket,
}

pub struct LiveIrohIdentity<'a> {
    pub namespace: &'a crate::node_state::NodeStateNamespace,
    pub endpoint_id: &'a str,
    pub handle_ref: &'a str,
    pub backend_ref: &'a str,
}

pub struct LiveIrohPublication {
    router: iroh::protocol::Router,
    _store: iroh_blobs::store::mem::MemStore,
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
            locator.ticket = iroh_blobs::ticket::BlobTicket::new(
                locator.ticket.addr().clone(),
                iroh_blobs::Hash::new(b"missing-live-iroh-blob"),
                iroh_blobs::BlobFormat::Raw,
            );
        }
    }

    pub async fn shutdown(self) -> crate::error::Result<()> {
        self.router.shutdown().await.map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("live Iroh blob router shutdown failed: {error}"))
        })
    }
}

// r[impl molten.content_store_adapter.identity_boundary]
// r[impl molten.content_store_adapter.verify_before_available]
#[cfg_attr(
    any(feature = "profiler", feature = "profiler-disabled"),
    flux_profiler::timed("molten_iroh_publish_chunks")
)]
pub async fn publish_live_iroh_chunks(
    profile: &ContentAdapterProfile,
    root: &crate::chunk_store::CapabilityChunkRoot,
    manifest_ref: &str,
    identity: LiveIrohIdentity<'_>,
) -> crate::error::Result<LiveIrohPublication> {
    if profile.class != ContentAdapterClass::IrohBlobs {
        return Err(crate::error::MoltenError::invalid_harness(
            "live Iroh publication requires iroh-blobs adapter profile",
        ));
    }
    let source_manifest = crate::chunk_store::read_manifest_with_root(root, manifest_ref)?;
    let manifest = manifest_descriptor(&source_manifest);
    let profile_issues = validate_content_profile(profile);
    if !profile_issues.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "live Iroh profile denied: {profile_issues:?}"
        )));
    }
    let manifest_issues = validate_manifest_descriptor(&manifest);
    if !manifest_issues.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "live Iroh manifest denied: {manifest_issues:?}"
        )));
    }
    if manifest.total_length > profile.bounds.max_total_bytes || manifest.chunks.len() > profile.bounds.max_chunk_count
    {
        return Err(crate::error::MoltenError::invalid_harness("live Iroh publication exceeds adapter bounds"));
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
    let store = iroh_blobs::store::mem::MemStore::new();
    let mut locators = Vec::with_capacity(source_manifest.chunks.len());
    for (position, chunk) in source_manifest.chunks.iter().enumerate() {
        let manifest_chunk_size = usize::try_from(source_manifest.chunk_size).map_err(|_| {
            crate::error::MoltenError::invalid_harness("live Iroh manifest chunk size does not fit usize")
        })?;
        let bytes = crate::chunk_store::read_verified_chunk(root, chunk, manifest_chunk_size)?;
        let tag = store.blobs().add_bytes(bytes).await.map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("live Iroh blob import failed: {error}"))
        })?;
        locators.push(LiveChunkLocator {
            chunk_ref: chunk.chunk_ref.clone(),
            position,
            ticket: iroh_blobs::ticket::BlobTicket::new(endpoint.addr(), tag.hash, iroh_blobs::BlobFormat::Raw),
        });
    }
    let blobs = iroh_blobs::BlobsProtocol::new(&store, None);
    let router = iroh::protocol::Router::builder(endpoint).accept(iroh_blobs::ALPN, blobs).spawn();
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

/// The profile, publication, command, generation, retained state, and timeout of one live Iroh
/// stream get.
#[derive(Clone, Copy)]
pub struct StreamGetInput<'a> {
    pub profile: &'a ContentAdapterProfile,
    pub publication: &'a LiveIrohPublication,
    pub command: &'a ContentCommand,
    pub generation: u64,
    pub retained: Option<&'a ContentPartialState>,
    pub timeout: std::time::Duration,
}

// r[impl molten.content_store_adapter.verify_before_available]
// r[impl molten.content_store_adapter.live_sim_conformance]
#[cfg_attr(
    any(feature = "profiler", feature = "profiler-disabled"),
    flux_profiler::timed("molten_iroh_stream_get")
)]
pub async fn execute_live_iroh_stream_get(input: StreamGetInput<'_>) -> crate::error::Result<LiveIrohContentExecution> {
    let StreamGetInput {
        profile,
        publication,
        command,
        generation,
        retained,
        ..
    } = input;
    if profile.class != ContentAdapterClass::IrohBlobs {
        return Err(crate::error::MoltenError::invalid_harness("live Iroh get requires iroh-blobs adapter profile"));
    }
    let preflight = preflight_content_operation(profile, &publication.manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "live Iroh preflight denied: {:?}",
            preflight.issues
        )));
    }
    let mut state =
        begin_partial_state(profile, &publication.manifest, command, generation, retained).map_err(|issues| {
            crate::error::MoltenError::invalid_harness(format!("live Iroh partial state denied: {issues:?}"))
        })?;
    let client = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .bind()
        .await
        .map_err(iroh_error)?;
    let resume_position = state.verified_chunk_refs.len();
    // Every remaining locator adds exactly one event and at most one verified chunk.
    let remaining_locators = publication.locators.len().saturating_sub(resume_position);
    let mut events = Vec::with_capacity(remaining_locators);
    let mut verified_chunks = Vec::with_capacity(remaining_locators);
    for locator in publication.locators.iter().skip(resume_position) {
        let verified = match verify_located_chunk(&client, input, locator, &state).await? {
            Ok(verified) => verified,
            Err((failure, chunk_ref)) => {
                state = classify_content_failure(profile, &state, failure).map_err(transition_error)?;
                events.push(terminal_event(profile, command, &publication.manifest, &state, chunk_ref)?);
                client.close().await;
                return finish_live_execution(profile, publication, state, events, verified_chunks);
            }
        };
        state = verified.next_state;
        events.push(canonical_content_event(
            profile,
            &content_event(EventInput {
                command,
                sequence: verified.observation.sequence,
                terminal: state.terminal,
                chunk_ref: Some(verified.descriptor.chunk_ref.clone()),
                observed_bytes: verified.observation.observed_length,
                failure: None,
                evidence_refs: &publication.manifest.evidence_refs,
            }),
        )?);
        verified_chunks.push(VerifiedChunkPayload {
            chunk_ref: verified.descriptor.chunk_ref.clone(),
            position: verified.descriptor.position,
            bytes: verified.bytes,
        });
    }
    client.close().await;
    if !content_is_available(&publication.manifest, &state) {
        return Err(crate::error::MoltenError::invalid_harness("live Iroh transfer ended before full verification"));
    }
    finish_live_execution(profile, publication, state, events, verified_chunks)
}

/// A located chunk that was fetched, measured, and admitted into the next partial state.
struct VerifiedChunk<'a> {
    descriptor: &'a ContentChunkDescriptor,
    bytes: Vec<u8>,
    observation: ContentChunkObservation,
    next_state: ContentPartialState,
}

/// Why a located chunk could not be verified, with the chunk ref when the failure is tied to the
/// chunk.
type ChunkFailure = (ContentFailure, Option<String>);

/// Fetches the chunk a locator names within the timeout, measures it, and applies it to `state`. A
/// missing or stale descriptor, a failed or timed-out connection or read, or a verification failure
/// yields the failure the stream terminates with.
async fn verify_located_chunk<'a>(
    client: &iroh::Endpoint,
    input: StreamGetInput<'a>,
    locator: &LiveChunkLocator,
    state: &ContentPartialState,
) -> crate::error::Result<std::result::Result<VerifiedChunk<'a>, ChunkFailure>> {
    let StreamGetInput {
        profile,
        publication,
        command,
        timeout,
        ..
    } = input;
    let Some(descriptor) = publication.manifest.chunks.get(locator.position) else {
        return Ok(Err((ContentFailure::AdapterFailure, None)));
    };
    if descriptor.chunk_ref != locator.chunk_ref {
        return Ok(Err((ContentFailure::StaleTicket, Some(descriptor.chunk_ref.clone()))));
    }
    let connection =
        match tokio::time::timeout(timeout, client.connect(locator.ticket.addr().clone(), iroh_blobs::ALPN)).await {
            Ok(Ok(connection)) => connection,
            Ok(Err(_)) => return Ok(Err((ContentFailure::TransportDisconnected, None))),
            Err(_) => return Ok(Err((ContentFailure::Timeout, None))),
        };
    let bytes = match tokio::time::timeout(
        timeout,
        receive_bounded_blob(connection, locator.ticket.hash(), profile.bounds.max_chunk_bytes),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return Ok(Err((ContentFailure::StaleTicket, Some(descriptor.chunk_ref.clone())))),
        Err(_) => return Ok(Err((ContentFailure::Timeout, Some(descriptor.chunk_ref.clone())))),
    };
    let sequence = next_sequence(state)?;
    let observation = ContentChunkObservation {
        operation_ref: command.operation_ref.clone(),
        manifest_ref: publication.manifest.manifest_ref.clone(),
        sequence,
        chunk_ref: descriptor.chunk_ref.clone(),
        position: descriptor.position,
        observed_content_ref: crate::chunk_store::hash_chunk(
            &bytes,
            usize::try_from(publication.manifest.chunk_size)
                .map_err(|_| crate::error::MoltenError::invalid_harness("live Iroh chunk size does not fit usize"))?,
        ),
        observed_length: u64::try_from(bytes.len())
            .map_err(|_| crate::error::MoltenError::invalid_harness("live Iroh chunk length does not fit u64"))?,
    };
    match apply_chunk_observation(profile, &publication.manifest, state, &observation) {
        Ok(next_state) => Ok(Ok(VerifiedChunk {
            descriptor,
            bytes,
            observation,
            next_state,
        })),
        Err(issues) => Ok(Err((verification_failure(&issues), Some(descriptor.chunk_ref.clone())))),
    }
}

fn finish_live_execution(
    profile: &ContentAdapterProfile,
    publication: &LiveIrohPublication,
    state: ContentPartialState,
    events: Vec<CanonicalContentArtifact<ContentEvent>>,
    verified_chunks: Vec<VerifiedChunkPayload>,
) -> crate::error::Result<LiveIrohContentExecution> {
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
) -> crate::error::Result<CanonicalContentArtifact<ContentEvent>> {
    let sequence = state
        .last_sequence
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("terminal live Iroh state lacks event sequence"))?;
    canonical_content_event(
        profile,
        &content_event(EventInput {
            command,
            sequence,
            terminal: state.terminal,
            chunk_ref,
            observed_bytes: 0,
            failure: state.failure,
            evidence_refs: &manifest.evidence_refs,
        }),
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

fn transition_error(issue: ContentIssue) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("live Iroh transition denied: {issue:?}"))
}

async fn receive_bounded_blob(
    connection: iroh::endpoint::Connection,
    hash: iroh_blobs::Hash,
    maximum_bytes: u64,
) -> crate::error::Result<Vec<u8>> {
    let maximum = usize::try_from(maximum_bytes)
        .map_err(|_| crate::error::MoltenError::invalid_harness("live Iroh receive bound does not fit usize"))?;
    let mut bytes = Vec::new();
    let mut progress = iroh_blobs::get::request::get_blob(connection, hash);
    loop {
        match progress.next().await {
            Some(iroh_blobs::get::request::GetBlobItem::Item(bao_tree::io::BaoContentItem::Leaf(leaf))) => {
                let next_length = bytes
                    .len()
                    .checked_add(leaf.data.len())
                    .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Iroh receive length overflow"))?;
                if next_length > maximum {
                    return Err(crate::error::MoltenError::invalid_harness(
                        "live Iroh blob exceeds admitted chunk bound",
                    ));
                }
                bytes.extend_from_slice(&leaf.data);
            }
            Some(iroh_blobs::get::request::GetBlobItem::Item(bao_tree::io::BaoContentItem::Parent(_))) => {}
            Some(iroh_blobs::get::request::GetBlobItem::Done(_)) => break,
            Some(iroh_blobs::get::request::GetBlobItem::Error(error)) => {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "live Iroh blob stream failed: {error}"
                )));
            }
            None => {
                return Err(crate::error::MoltenError::invalid_harness(
                    "live Iroh blob stream ended without terminal item",
                ));
            }
        }
    }
    Ok(bytes)
}

fn next_sequence(state: &ContentPartialState) -> crate::error::Result<u64> {
    state
        .last_sequence
        .map_or(Some(0), |sequence| sequence.checked_add(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("live Iroh event sequence overflow"))
}

fn iroh_error(error: impl std::fmt::Display) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("live Iroh content adapter failed: {error}"))
}
