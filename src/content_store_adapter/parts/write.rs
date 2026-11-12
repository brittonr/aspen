use molten_core::content_store_adapter::*;

use super::*;

// r[impl molten.content_store_adapter.partial_state]
// r[impl molten.content_store_adapter.live_sim_conformance]
/// Drive a put through the same transition core as the stream path.
///
/// A successful put applies every chunk observation and marks the completed
/// prefix durable. A provider throttle is retryable, a torn put is uncertain
/// because the remote may have accepted bytes, and a multipart abort is failed.
/// No failure arm returns verified payloads or a durable terminal.
pub fn execute_simulated_put(input: SimulatedStreamInput<'_>) -> crate::error::Result<ModelExecution> {
    let SimulatedStreamInput {
        profile,
        manifest,
        command,
        fault,
        ..
    } = input;
    if command.operation != ContentOperation::Put && command.operation != ContentOperation::Import {
        return Err(crate::error::Failure::invalid_harness("simulated put requires put or import operation"));
    }
    let mut state = match begin_simulated_state(input)? {
        std::ops::ControlFlow::Continue(state) => state,
        std::ops::ControlFlow::Break(execution) => return Ok(execution),
    };
    let resume_position = state.verified_chunk_refs.len();
    let remaining_chunks = manifest.chunks.len().saturating_sub(resume_position);
    let mut events = Vec::with_capacity(remaining_chunks.saturating_add(1));
    for descriptor in manifest.chunks.iter().skip(resume_position) {
        if let Some(failure) = failure_at(fault, descriptor.position) {
            state =
                classify_content_failure(profile, &state, failure, context_for(failure)).map_err(transition_error)?;
            events.push(terminal_stream_event(input, &state, Some(descriptor.chunk_ref.clone()), Some(failure))?);
            return finish(profile, manifest, state, events);
        }
        let (_bytes, observation) = simulated_chunk_observation(input, descriptor, &state)?;
        match apply_chunk_observation(profile, manifest, &state, &observation) {
            Ok(next) => {
                state = next;
                events.push(bind_event(
                    profile,
                    &event(EventInput {
                        command,
                        sequence: observation.sequence,
                        terminal: state.terminal,
                        chunk_ref: Some(descriptor.chunk_ref.clone()),
                        observed_bytes: observation.observed_length,
                        failure: None,
                        evidence_refs: &manifest.evidence_refs,
                    }),
                )?);
            }
            Err(issues) => {
                let failure = failure_from_issues(&issues);
                state =
                    classify_content_failure(profile, &state, failure, UNACKNOWLEDGED_PUT).map_err(transition_error)?;
                events.push(terminal_stream_event(input, &state, Some(descriptor.chunk_ref.clone()), Some(failure))?);
                return finish(profile, manifest, state, events);
            }
        }
    }
    if state.terminal == ContentTerminal::Verified {
        state = mark_content_durable(profile, &state).map_err(transition_error)?;
        events.push(terminal_stream_event(input, &state, None, None)?);
    }
    finish(profile, manifest, state, events)
}

fn failure_at(fault: Option<Fault>, position: usize) -> Option<ContentFailure> {
    match fault {
        Some(Fault::ProviderThrottleAt(at)) if at == position => Some(ContentFailure::ProviderThrottle),
        Some(Fault::TornPutAt(at)) if at == position => Some(ContentFailure::TransportDisconnected),
        Some(Fault::MultipartAbortAt(at)) if at == position => Some(ContentFailure::AdapterFailure),
        Some(
            Fault::CorruptAt(_)
            | Fault::TruncateAt(_)
            | Fault::CancelAt(_)
            | Fault::DisconnectAt(_)
            | Fault::CapacityExceeded
            | Fault::LatencyTicks(_)
            | Fault::ProviderThrottleAt(_)
            | Fault::TornPutAt(_)
            | Fault::MultipartAbortAt(_),
        )
        | None => None,
    }
}

fn context_for(failure: ContentFailure) -> ContentFailureContext {
    match failure {
        ContentFailure::TransportDisconnected => ACKNOWLEDGED_PUT,
        ContentFailure::CorruptChunk
        | ContentFailure::TruncatedChunk
        | ContentFailure::ReorderedChunk
        | ContentFailure::UnexpectedChunk
        | ContentFailure::StaleTicket
        | ContentFailure::UnsupportedTransform
        | ContentFailure::RootEscape
        | ContentFailure::Overload
        | ContentFailure::ProviderThrottle
        | ContentFailure::PermissionDenied
        | ContentFailure::Timeout
        | ContentFailure::AdapterFailure => UNACKNOWLEDGED_PUT,
    }
}

fn finish(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: ContentPartialState,
    events: Vec<Artifact<ContentEvent>>,
) -> crate::error::Result<ModelExecution> {
    Ok(ModelExecution {
        state: partial_state(profile, manifest, &state)?,
        events,
        verified_chunks: Vec::new(),
    })
}
