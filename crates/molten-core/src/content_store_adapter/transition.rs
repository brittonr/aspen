use super::*;

// r[impl molten.content_store_adapter.partial_state]
pub fn begin_partial_state(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    generation: u64,
    retained: Option<&ContentPartialState>,
) -> Result<ContentPartialState, Vec<ContentIssue>> {
    let mut issues = Vec::new();
    if generation == 0 {
        issues.push(ContentIssue::ZeroBound("content-operation-generation"));
    }
    let verified = retained
        .map(|state| {
            let retained_issues = validate_partial_state(profile, manifest, state);
            if state.generation != generation || state.operation_ref != command.operation_ref {
                issues.push(ContentIssue::PartialStateMismatch);
            }
            issues.extend(retained_issues);
            state.verified_chunk_refs.clone()
        })
        .unwrap_or_default();
    if !issues.is_empty() {
        return Err(issues);
    }
    let missing_chunk_refs = manifest.chunks.iter().skip(verified.len()).map(|chunk| chunk.chunk_ref.clone()).collect();
    let verified_bytes = manifest
        .chunks
        .iter()
        .take(verified.len())
        .try_fold(0_u64, |total, chunk| total.checked_add(chunk.length).ok_or(ContentIssue::ArithmeticOverflow))
        .map_err(|issue| vec![issue])?;
    Ok(ContentPartialState {
        schema: CONTENT_PARTIAL_STATE_SCHEMA.to_string(),
        operation_ref: command.operation_ref.clone(),
        manifest_ref: manifest.manifest_ref.clone(),
        profile_ref: profile.profile_ref.clone(),
        generation,
        terminal: if verified.is_empty() {
            ContentTerminal::Accepted
        } else {
            ContentTerminal::Streaming
        },
        verified_chunk_refs: verified,
        missing_chunk_refs,
        verified_bytes,
        event_count: retained.map_or(0, |state| state.event_count),
        last_sequence: retained.and_then(|state| state.last_sequence),
        failure: None,
    })
}

// r[impl molten.content_store_adapter.verify_before_available]
// r[impl molten.content_store_adapter.partial_state]
pub fn apply_chunk_observation(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
    observation: &ContentChunkObservation,
) -> Result<ContentPartialState, Vec<ContentIssue>> {
    let mut issues = validate_partial_state(profile, manifest, state);
    if state.terminal.is_terminal() {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    if observation.operation_ref != state.operation_ref || observation.manifest_ref != manifest.manifest_ref {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    let expected_sequence = match state.last_sequence {
        Some(sequence) => match sequence.checked_add(1) {
            Some(next) => next,
            None => {
                issues.push(ContentIssue::ArithmeticOverflow);
                sequence
            }
        },
        None => 0,
    };
    if observation.sequence != expected_sequence {
        issues.push(ContentIssue::ReorderedChunk(observation.chunk_ref.clone()));
    }
    let Some(expected_ref) = state.missing_chunk_refs.first() else {
        issues.push(ContentIssue::UnexpectedChunk(observation.chunk_ref.clone()));
        return Err(issues);
    };
    if &observation.chunk_ref != expected_ref {
        issues.push(ContentIssue::ReorderedChunk(observation.chunk_ref.clone()));
    }
    let Some(descriptor) = manifest.chunks.get(observation.position) else {
        issues.push(ContentIssue::UnexpectedChunk(observation.chunk_ref.clone()));
        return Err(issues);
    };
    if descriptor.position != observation.position || descriptor.chunk_ref != observation.chunk_ref {
        issues.push(ContentIssue::ReorderedChunk(observation.chunk_ref.clone()));
    }
    if descriptor.length != observation.observed_length {
        issues.push(ContentIssue::TruncatedChunk(observation.chunk_ref.clone()));
    }
    if descriptor.chunk_ref != observation.observed_content_ref {
        issues.push(ContentIssue::CorruptChunk(observation.chunk_ref.clone()));
    }
    if state.event_count >= profile.bounds.max_events {
        issues.push(ContentIssue::EventLimitExceeded);
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let verified_bytes = state
        .verified_bytes
        .checked_add(observation.observed_length)
        .ok_or_else(|| vec![ContentIssue::ArithmeticOverflow])?;
    if verified_bytes > profile.bounds.max_total_bytes || verified_bytes > manifest.total_length {
        return Err(vec![ContentIssue::TotalBytesExceeded]);
    }
    let mut next = state.clone();
    next.verified_chunk_refs.push(observation.chunk_ref.clone());
    next.missing_chunk_refs.remove(0);
    next.verified_bytes = verified_bytes;
    next.event_count += 1;
    next.last_sequence = Some(observation.sequence);
    next.terminal = if next.missing_chunk_refs.is_empty() {
        ContentTerminal::Verified
    } else {
        ContentTerminal::Streaming
    };
    Ok(next)
}

pub fn classify_content_failure(
    profile: &ContentAdapterProfile,
    state: &ContentPartialState,
    failure: ContentFailure,
) -> Result<ContentPartialState, ContentIssue> {
    let terminal = match failure {
        ContentFailure::Overload => ContentTerminal::Retryable,
        ContentFailure::TransportDisconnected if !state.verified_chunk_refs.is_empty() => ContentTerminal::Uncertain,
        ContentFailure::TransportDisconnected => ContentTerminal::Retryable,
        ContentFailure::Timeout => ContentTerminal::Uncertain,
        _ => ContentTerminal::Failed,
    };
    record_terminal_event(profile, state, terminal, Some(failure))
}

pub fn cancel_content_operation(
    profile: &ContentAdapterProfile,
    state: &ContentPartialState,
) -> Result<ContentPartialState, ContentIssue> {
    record_terminal_event(profile, state, ContentTerminal::Cancelled, None)
}

pub fn mark_content_durable(
    profile: &ContentAdapterProfile,
    state: &ContentPartialState,
) -> Result<ContentPartialState, ContentIssue> {
    if state.terminal != ContentTerminal::Verified {
        return Err(ContentIssue::PartialStateMismatch);
    }
    if !profile.capabilities.contains(&ContentCapability::DurableCompletion) {
        return Err(ContentIssue::UnsupportedCapability);
    }
    let mut next = state.clone();
    next.terminal = ContentTerminal::Durable;
    Ok(next)
}

pub fn content_is_available(manifest: &ContentManifestDescriptor, state: &ContentPartialState) -> bool {
    matches!(state.terminal, ContentTerminal::Verified | ContentTerminal::Durable)
        && state.manifest_ref == manifest.manifest_ref
        && state.verified_bytes == manifest.total_length
        && state.missing_chunk_refs.is_empty()
        && state.verified_chunk_refs.len() == manifest.chunks.len()
}

// r[impl molten.content_store_adapter.retention_boundary]
pub fn admit_content_read(authority: Option<&ContentProtectionAuthority>) -> ContentAuthorityDecision {
    match authority {
        Some(authority)
            if authority.schema == CONTENT_PROTECTION_AUTHORITY_SCHEMA
                && authority.read_authority_ref.as_deref().is_some_and(crate::fabric::valid_blake3_ref) =>
        {
            ContentAuthorityDecision::Admit
        }
        _ => ContentAuthorityDecision::Deny,
    }
}

// r[impl molten.content_store_adapter.retention_boundary]
pub fn admit_content_deletion(authority: Option<&ContentProtectionAuthority>) -> ContentAuthorityDecision {
    match authority {
        Some(authority)
            if authority.schema == CONTENT_PROTECTION_AUTHORITY_SCHEMA
                && crate::fabric::valid_blake3_ref(&authority.retention_policy_ref)
                && authority.canonical_pin_ref.is_none()
                && authority.deletion_gate_ref.as_deref().is_some_and(crate::fabric::valid_blake3_ref) =>
        {
            ContentAuthorityDecision::Admit
        }
        _ => ContentAuthorityDecision::Deny,
    }
}

fn record_terminal_event(
    profile: &ContentAdapterProfile,
    state: &ContentPartialState,
    terminal: ContentTerminal,
    failure: Option<ContentFailure>,
) -> Result<ContentPartialState, ContentIssue> {
    if state.event_count >= profile.bounds.max_events {
        return Err(ContentIssue::EventLimitExceeded);
    }
    let sequence = match state.last_sequence {
        Some(sequence) => sequence.checked_add(1).ok_or(ContentIssue::ArithmeticOverflow)?,
        None => 0,
    };
    let mut next = state.clone();
    next.terminal = terminal;
    next.failure = failure;
    next.event_count += 1;
    next.last_sequence = Some(sequence);
    Ok(next)
}

pub const fn backend_protection_effect_grants_authority() -> ContentAuthorityDecision {
    ContentAuthorityDecision::Deny
}
