use molten_core::content_store_adapter::*;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationFault {
    CorruptAt(usize),
    TruncateAt(usize),
    CancelAt(usize),
    DisconnectAt(usize),
    CapacityExceeded,
    LatencyTicks(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationContentExecution {
    pub state: CanonicalContentArtifact<ContentPartialState>,
    pub events: Vec<CanonicalContentArtifact<ContentEvent>>,
    pub verified_chunks: Vec<VerifiedChunkPayload>,
}

/// The profile, manifest, command, generation, retained state, chunks, and fault of one simulated
/// stream.
#[derive(Clone, Copy)]
pub struct SimulatedStreamInput<'a> {
    pub profile: &'a ContentAdapterProfile,
    pub manifest: &'a ContentManifestDescriptor,
    pub command: &'a ContentCommand,
    pub generation: u64,
    pub retained: Option<&'a ContentPartialState>,
    pub chunks: &'a std::collections::BTreeMap<String, Vec<u8>>,
    pub fault: Option<SimulationFault>,
}

// r[impl molten.content_store_adapter.partial_state]
// r[impl molten.content_store_adapter.live_sim_conformance]
pub fn execute_simulated_stream(input: SimulatedStreamInput<'_>) -> crate::error::Result<SimulationContentExecution> {
    let SimulatedStreamInput {
        profile,
        manifest,
        command,
        fault,
        ..
    } = input;
    let mut state = match begin_simulated_state(input)? {
        std::ops::ControlFlow::Continue(state) => state,
        std::ops::ControlFlow::Break(execution) => return Ok(execution),
    };
    let resume_position = state.verified_chunk_refs.len();
    // Every remaining chunk adds at most one event and one verified chunk before the loop ends.
    let remaining_chunks = manifest.chunks.len().saturating_sub(resume_position);
    let mut events = Vec::with_capacity(remaining_chunks);
    let mut verified_chunks = Vec::with_capacity(remaining_chunks);
    for descriptor in manifest.chunks.iter().skip(resume_position) {
        if fault == Some(SimulationFault::CancelAt(descriptor.position)) {
            state = cancel_content_operation(profile, &state).map_err(transition_error)?;
            events.push(terminal_stream_event(input, &state, None, None)?);
            break;
        }
        if fault == Some(SimulationFault::DisconnectAt(descriptor.position)) {
            state = classify_content_failure(profile, &state, ContentFailure::TransportDisconnected)
                .map_err(transition_error)?;
            events.push(terminal_stream_event(input, &state, None, state.failure)?);
            break;
        }
        let (bytes, observation) = simulated_chunk_observation(input, descriptor, &state)?;
        match apply_chunk_observation(profile, manifest, &state, &observation) {
            Ok(next) => {
                state = next;
                verified_chunks.push(VerifiedChunkPayload {
                    chunk_ref: descriptor.chunk_ref.clone(),
                    position: descriptor.position,
                    bytes: bytes.clone(),
                });
                events.push(canonical_content_event(
                    profile,
                    &content_event(EventInput {
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
                state = classify_content_failure(profile, &state, failure).map_err(transition_error)?;
                events.push(terminal_stream_event(input, &state, Some(descriptor.chunk_ref.clone()), Some(failure))?);
                break;
            }
        }
    }
    let state = canonical_partial_state(profile, manifest, &state)?;
    Ok(SimulationContentExecution {
        state,
        events,
        verified_chunks,
    })
}

/// Admits the simulation and begins its partial state, or finishes early for an overload or a
/// latency fault past the command deadline.
fn begin_simulated_state(
    input: SimulatedStreamInput<'_>,
) -> crate::error::Result<std::ops::ControlFlow<SimulationContentExecution, ContentPartialState>> {
    let SimulatedStreamInput {
        profile,
        manifest,
        command,
        generation,
        retained,
        fault,
        ..
    } = input;
    if profile.class != ContentAdapterClass::DeterministicSimulation {
        return Err(crate::error::MoltenError::invalid_harness(
            "simulation execution requires deterministic simulation profile",
        ));
    }
    let preflight = preflight_content_operation(profile, manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "simulation preflight denied: {:?}",
            preflight.issues
        )));
    }
    let state = begin_partial_state(profile, manifest, command, generation, retained).map_err(|issues| {
        crate::error::MoltenError::invalid_harness(format!("simulation partial state denied: {issues:?}"))
    })?;
    if fault == Some(SimulationFault::CapacityExceeded) {
        let state = classify_content_failure(profile, &state, ContentFailure::Overload).map_err(transition_error)?;
        return terminal_execution(profile, manifest, command, state).map(std::ops::ControlFlow::Break);
    }
    if let Some(SimulationFault::LatencyTicks(latency)) = fault {
        let completion_tick = command
            .submitted_tick
            .checked_add(latency)
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("simulation latency overflow"))?;
        if completion_tick > command.deadline_tick {
            let state = classify_content_failure(profile, &state, ContentFailure::Timeout).map_err(transition_error)?;
            return terminal_execution(profile, manifest, command, state).map(std::ops::ControlFlow::Break);
        }
    }
    Ok(std::ops::ControlFlow::Continue(state))
}

/// The terminal event after a cancellation, disconnect, or failed chunk, carrying no observed
/// bytes.
fn terminal_stream_event(
    input: SimulatedStreamInput<'_>,
    state: &ContentPartialState,
    chunk_ref: Option<String>,
    failure: Option<ContentFailure>,
) -> crate::error::Result<CanonicalContentArtifact<ContentEvent>> {
    canonical_content_event(
        input.profile,
        &content_event(EventInput {
            command: input.command,
            sequence: terminal_sequence(state)?,
            terminal: state.terminal,
            chunk_ref,
            observed_bytes: 0,
            failure,
            evidence_refs: &input.manifest.evidence_refs,
        }),
    )
}

/// The scripted chunk bytes and their observation, with the configured corruption or truncation
/// fault applied.
fn simulated_chunk_observation<'a>(
    input: SimulatedStreamInput<'a>,
    descriptor: &ContentChunkDescriptor,
    state: &ContentPartialState,
) -> crate::error::Result<(&'a Vec<u8>, ContentChunkObservation)> {
    let SimulatedStreamInput {
        manifest,
        command,
        chunks,
        fault,
        ..
    } = input;
    let bytes = chunks.get(&descriptor.chunk_ref).ok_or_else(|| {
        crate::error::MoltenError::invalid_harness(format!("simulation lacks chunk {}", descriptor.chunk_ref))
    })?;
    let manifest_chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|_| crate::error::MoltenError::invalid_harness("simulation chunk size does not fit usize"))?;
    let mut observed_ref = crate::chunk_store::hash_chunk(bytes, manifest_chunk_size);
    let mut observed_length = u64::try_from(bytes.len())
        .map_err(|_| crate::error::MoltenError::invalid_harness("simulation chunk length does not fit u64"))?;
    if fault == Some(SimulationFault::CorruptAt(descriptor.position)) {
        observed_ref = crate::preserves_rail::content_ref_from_bytes(b"deterministic-corruption");
    }
    if fault == Some(SimulationFault::TruncateAt(descriptor.position)) {
        observed_length = observed_length.saturating_sub(1);
    }
    let sequence = next_sequence(state)?;
    Ok((bytes, ContentChunkObservation {
        operation_ref: command.operation_ref.clone(),
        manifest_ref: manifest.manifest_ref.clone(),
        sequence,
        chunk_ref: descriptor.chunk_ref.clone(),
        position: descriptor.position,
        observed_content_ref: observed_ref,
        observed_length,
    }))
}

fn terminal_execution(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    state: ContentPartialState,
) -> crate::error::Result<SimulationContentExecution> {
    let event = canonical_content_event(
        profile,
        &content_event(EventInput {
            command,
            sequence: terminal_sequence(&state)?,
            terminal: state.terminal,
            chunk_ref: None,
            observed_bytes: 0,
            failure: state.failure,
            evidence_refs: &manifest.evidence_refs,
        }),
    )?;
    Ok(SimulationContentExecution {
        state: canonical_partial_state(profile, manifest, &state)?,
        events: vec![event],
        verified_chunks: Vec::new(),
    })
}

fn failure_from_issues(issues: &[ContentIssue]) -> ContentFailure {
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

fn terminal_sequence(state: &ContentPartialState) -> crate::error::Result<u64> {
    state
        .last_sequence
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("terminal simulation state lacks event sequence"))
}

fn transition_error(issue: ContentIssue) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("simulation transition denied: {issue:?}"))
}

fn next_sequence(state: &ContentPartialState) -> crate::error::Result<u64> {
    state
        .last_sequence
        .map_or(Some(0), |sequence| sequence.checked_add(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("simulation event sequence overflow"))
}
