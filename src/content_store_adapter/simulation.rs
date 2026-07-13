use std::collections::BTreeMap;

use molten_core::content_store_adapter::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

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

// r[impl molten.content_store_adapter.partial_state]
// r[impl molten.content_store_adapter.live_sim_conformance]
pub fn execute_simulated_stream(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    generation: u64,
    retained: Option<&ContentPartialState>,
    chunks: &BTreeMap<String, Vec<u8>>,
    fault: Option<SimulationFault>,
) -> Result<SimulationContentExecution> {
    if profile.class != ContentAdapterClass::DeterministicSimulation {
        return Err(MoltenError::invalid_harness("simulation execution requires deterministic simulation profile"));
    }
    let preflight = preflight_content_operation(profile, manifest, command, 0, 0);
    if preflight.terminal != ContentTerminal::Accepted || !preflight.issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("simulation preflight denied: {:?}", preflight.issues)));
    }
    let mut state = begin_partial_state(profile, manifest, command, generation, retained)
        .map_err(|issues| MoltenError::invalid_harness(format!("simulation partial state denied: {issues:?}")))?;
    if fault == Some(SimulationFault::CapacityExceeded) {
        state = classify_content_failure(profile, &state, ContentFailure::Overload).map_err(transition_error)?;
        return terminal_execution(profile, manifest, command, state);
    }
    if let Some(SimulationFault::LatencyTicks(latency)) = fault {
        let completion_tick = command
            .submitted_tick
            .checked_add(latency)
            .ok_or_else(|| MoltenError::invalid_harness("simulation latency overflow"))?;
        if completion_tick > command.deadline_tick {
            state = classify_content_failure(profile, &state, ContentFailure::Timeout).map_err(transition_error)?;
            return terminal_execution(profile, manifest, command, state);
        }
    }
    let mut events = Vec::new();
    let mut verified_chunks = Vec::new();
    let resume_position = state.verified_chunk_refs.len();
    for descriptor in manifest.chunks.iter().skip(resume_position) {
        if fault == Some(SimulationFault::CancelAt(descriptor.position)) {
            state = cancel_content_operation(profile, &state).map_err(transition_error)?;
            events.push(canonical_content_event(
                profile,
                &content_event(
                    command,
                    terminal_sequence(&state)?,
                    state.terminal,
                    None,
                    0,
                    None,
                    &manifest.evidence_refs,
                ),
            )?);
            break;
        }
        if fault == Some(SimulationFault::DisconnectAt(descriptor.position)) {
            state = classify_content_failure(profile, &state, ContentFailure::TransportDisconnected)
                .map_err(transition_error)?;
            events.push(canonical_content_event(
                profile,
                &content_event(
                    command,
                    terminal_sequence(&state)?,
                    state.terminal,
                    None,
                    0,
                    state.failure,
                    &manifest.evidence_refs,
                ),
            )?);
            break;
        }
        let bytes = chunks
            .get(&descriptor.chunk_ref)
            .ok_or_else(|| MoltenError::invalid_harness(format!("simulation lacks chunk {}", descriptor.chunk_ref)))?;
        let manifest_chunk_size = usize::try_from(manifest.chunk_size)
            .map_err(|_| MoltenError::invalid_harness("simulation chunk size does not fit usize"))?;
        let mut observed_ref = crate::chunk_store::hash_chunk(bytes, manifest_chunk_size);
        let mut observed_length = u64::try_from(bytes.len())
            .map_err(|_| MoltenError::invalid_harness("simulation chunk length does not fit u64"))?;
        if fault == Some(SimulationFault::CorruptAt(descriptor.position)) {
            observed_ref = crate::preserves_rail::content_ref_from_bytes(b"deterministic-corruption");
        }
        if fault == Some(SimulationFault::TruncateAt(descriptor.position)) {
            observed_length = observed_length.saturating_sub(1);
        }
        let sequence = next_sequence(&state)?;
        let observation = ContentChunkObservation {
            operation_ref: command.operation_ref.clone(),
            manifest_ref: manifest.manifest_ref.clone(),
            sequence,
            chunk_ref: descriptor.chunk_ref.clone(),
            position: descriptor.position,
            observed_content_ref: observed_ref,
            observed_length,
        };
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
                    &content_event(
                        command,
                        sequence,
                        state.terminal,
                        Some(descriptor.chunk_ref.clone()),
                        observed_length,
                        None,
                        &manifest.evidence_refs,
                    ),
                )?);
            }
            Err(issues) => {
                let failure = failure_from_issues(&issues);
                state = classify_content_failure(profile, &state, failure).map_err(transition_error)?;
                events.push(canonical_content_event(
                    profile,
                    &content_event(
                        command,
                        terminal_sequence(&state)?,
                        state.terminal,
                        Some(descriptor.chunk_ref.clone()),
                        0,
                        Some(failure),
                        &manifest.evidence_refs,
                    ),
                )?);
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

fn terminal_execution(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    state: ContentPartialState,
) -> Result<SimulationContentExecution> {
    let event = canonical_content_event(
        profile,
        &content_event(
            command,
            terminal_sequence(&state)?,
            state.terminal,
            None,
            0,
            state.failure,
            &manifest.evidence_refs,
        ),
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

fn terminal_sequence(state: &ContentPartialState) -> Result<u64> {
    state
        .last_sequence
        .ok_or_else(|| MoltenError::invalid_harness("terminal simulation state lacks event sequence"))
}

fn transition_error(issue: ContentIssue) -> MoltenError {
    MoltenError::invalid_harness(format!("simulation transition denied: {issue:?}"))
}

fn next_sequence(state: &ContentPartialState) -> Result<u64> {
    state
        .last_sequence
        .map_or(Some(0), |sequence| sequence.checked_add(1))
        .ok_or_else(|| MoltenError::invalid_harness("simulation event sequence overflow"))
}
