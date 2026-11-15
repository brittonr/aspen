/// Execute an admitted immutable-object put through the shared transition core.
/// Only a confirmed provider acknowledgement permits the durable terminal.
pub async fn execute_object_put(
    input: FetchInput<'_>,
    chunks: &[Vec<u8>],
    client: &mut CachedClient,
) -> crate::error::Result<ObjectExecution> {
    let FetchInput { profile, publication, command, generation, retained } = input;
    if profile.class != ContentAdapterClass::ObjectStorage
        || !matches!(command.operation, ContentOperation::Put | ContentOperation::Import)
        || chunks.len() != publication.manifest.chunks.len()
        || publication.keys.len() != chunks.len()
        || retained.is_some_and(|state| state.terminal == ContentTerminal::Uncertain)
    {
        return Err(crate::error::Failure::invalid_harness(
            "object put requires exact inputs and refuses silent uncertain retry",
        ));
    }
    let preflight = preflight_content_operation(profile, &publication.manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(crate::error::Failure::invalid_harness(format!("object put denied: {:?}", preflight.issues)));
    }
    let mut state = begin_partial_state(profile, &publication.manifest, command, generation, retained)
        .map_err(|issues| crate::error::Failure::invalid_harness(format!("object put state denied: {issues:?}")))?;
    let chunk_size = usize::try_from(publication.manifest.chunk_size)
        .map_err(|_| crate::error::Failure::invalid_harness("object chunk size exceeds usize"))?;
    let mut events = Vec::new();
    for descriptor in publication.manifest.chunks.iter().skip(state.verified_chunk_refs.len()) {
        let bytes = &chunks[descriptor.position];
        let observation = ContentChunkObservation {
            operation_ref: command.operation_ref.clone(), manifest_ref: publication.manifest.manifest_ref.clone(),
            sequence: next_sequence(&state)?, chunk_ref: descriptor.chunk_ref.clone(), position: descriptor.position,
            observed_content_ref: crate::chunk_store::hash_chunk(bytes, chunk_size), observed_length: bytes.len() as u64,
        };
        let next = match apply_chunk_observation(profile, &publication.manifest, &state, &observation) {
            Ok(next) => next,
            Err(issues) => {
                state = classify_content_failure(profile, &state, failure_from_issues(&issues), UNACKNOWLEDGED_PUT)
                    .map_err(transition_error)?;
                return terminate(profile, publication, command, state, events, Vec::new());
            }
        };
        let key = &publication.keys[descriptor.position];
        if object_ref_from_key(key)? != crate::preserves_rail::content_ref_from_bytes(bytes) {
            return Err(crate::error::Failure::invalid_harness("put locator does not name immutable payload"));
        }
        if let Err(failure) = client.put_classified(key, bytes).await {
            // Dispatch was attempted. Even without an acknowledgement the peer may
            // have mutated; the client does not silently retry ambiguous PUTs.
            state = classify_content_failure(profile, &state, failure, ContentFailureContext {
                remote_put_acknowledged: matches!(failure, ContentFailure::TransportDisconnected | ContentFailure::Timeout),
            }).map_err(transition_error)?;
            return terminate(profile, publication, command, state, events, Vec::new());
        }
        state = next;
        events.push(bind_event(profile, &event(EventInput {
            command, sequence: observation.sequence, terminal: state.terminal,
            chunk_ref: Some(descriptor.chunk_ref.clone()), observed_bytes: observation.observed_length,
            failure: None, evidence_refs: &publication.manifest.evidence_refs,
        }))?);
    }
    if state.terminal == ContentTerminal::Verified {
        state = mark_content_durable(profile, &state).map_err(transition_error)?;
    }
    terminate(profile, publication, command, state, events, Vec::new())
}

/// Audit provider presence independently of local residency. Lifecycle deletion
/// of a protected remote object is an integrity failure, not an available copy.
pub async fn audit_protected_object(
    client: &CachedClient, retention: &Retention, key: &str, chunk_ref: &str, expected_bytes: u64,
) -> crate::error::Result<()> {
    if !retention.is_protected(chunk_ref) || client.head(key).await? != Some(expected_bytes) {
        return Err(crate::error::Failure::invalid_harness("protected remote object is absent or changed"));
    }
    Ok(())
}
