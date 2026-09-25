
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
