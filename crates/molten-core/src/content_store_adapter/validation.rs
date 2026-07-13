use super::*;
use crate::fabric::valid_blake3_ref;

// r[impl molten.content_store_adapter.port_contract]
// r[impl molten.content_store_adapter.streaming_bounds]
pub fn validate_content_profile(profile: &ContentAdapterProfile) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    if profile.schema != CONTENT_ADAPTER_PROFILE_SCHEMA {
        issues.push(ContentIssue::SchemaMismatch("content-adapter-profile"));
    }
    validate_token("content-profile-id", &profile.profile_id, &mut issues);
    validate_ref("content-profile-ref", &profile.profile_ref, &mut issues);
    validate_positive_bounds(&profile.bounds, &mut issues);
    validate_sorted_unique_capabilities(&profile.capabilities, &mut issues);
    validate_sorted_unique_tokens("supported-transforms", &profile.supported_transforms, &mut issues);
    validate_ref_list("content-profile-evidence-ref", &profile.evidence_refs, &mut issues);
    for required in REQUIRED_CONTENT_NON_CLAIMS {
        if !profile.non_claims.contains(&required) {
            issues.push(ContentIssue::MissingNonClaim(required.as_str()));
        }
    }
    if profile.non_claims.len() != REQUIRED_CONTENT_NON_CLAIMS.len() {
        issues.push(ContentIssue::DuplicateValue("content-profile-non-claims"));
    }
    issues
}

// r[impl molten.content_store_adapter.identity_boundary]
pub fn validate_manifest_descriptor(manifest: &ContentManifestDescriptor) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    validate_ref("content-manifest-ref", &manifest.manifest_ref, &mut issues);
    validate_token("content-chunker", &manifest.chunker, &mut issues);
    validate_ref("content-metadata-ref", &manifest.metadata_ref, &mut issues);
    if manifest.total_length == 0 {
        issues.push(ContentIssue::ZeroBound("content-total-length"));
    }
    if manifest.chunk_size == 0 {
        issues.push(ContentIssue::ZeroBound("content-chunk-size"));
    }
    if manifest.chunks.is_empty() {
        issues.push(ContentIssue::ZeroBound("content-chunk-count"));
    }
    validate_ref_list("content-policy-ref", &manifest.policy_refs, &mut issues);
    validate_ref_list("content-evidence-ref", &manifest.evidence_refs, &mut issues);
    let mut total = 0_u64;
    for (expected_position, chunk) in manifest.chunks.iter().enumerate() {
        validate_ref("content-chunk-ref", &chunk.chunk_ref, &mut issues);
        validate_token("content-chunk-transform", &chunk.transform, &mut issues);
        if chunk.length == 0 {
            issues.push(ContentIssue::ZeroBound("content-chunk-length"));
        }
        if chunk.position != expected_position {
            issues.push(ContentIssue::ReorderedChunk(chunk.chunk_ref.clone()));
        }
        match total.checked_add(chunk.length) {
            Some(next) => total = next,
            None => issues.push(ContentIssue::ArithmeticOverflow),
        }
    }
    if total != manifest.total_length {
        issues.push(ContentIssue::ManifestMismatch);
    }
    issues
}

pub fn validate_partial_state(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    if state.schema != CONTENT_PARTIAL_STATE_SCHEMA {
        issues.push(ContentIssue::SchemaMismatch("content-partial-state"));
    }
    for (field, reference) in [
        ("partial-operation-ref", state.operation_ref.as_str()),
        ("partial-manifest-ref", state.manifest_ref.as_str()),
        ("partial-profile-ref", state.profile_ref.as_str()),
    ] {
        validate_ref(field, reference, &mut issues);
    }
    if state.profile_ref != profile.profile_ref {
        issues.push(ContentIssue::ProfileMismatch);
    }
    if state.manifest_ref != manifest.manifest_ref {
        issues.push(ContentIssue::ManifestMismatch);
    }
    if state.generation == 0 {
        issues.push(ContentIssue::ZeroBound("partial-generation"));
    }
    if state.event_count > profile.bounds.max_events {
        issues.push(ContentIssue::EventLimitExceeded);
    }
    if state.verified_bytes > profile.bounds.max_total_bytes || state.verified_bytes > manifest.total_length {
        issues.push(ContentIssue::TotalBytesExceeded);
    }
    validate_partition(manifest, state, &mut issues);
    if matches!(state.terminal, ContentTerminal::Verified | ContentTerminal::Durable)
        && (!state.missing_chunk_refs.is_empty() || state.verified_bytes != manifest.total_length)
    {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    match (state.event_count, state.last_sequence) {
        (0, None) => {}
        (count, Some(sequence)) if u64::try_from(count).ok() == sequence.checked_add(1) => {}
        _ => issues.push(ContentIssue::PartialStateMismatch),
    }
    if state.failure.is_some()
        && !matches!(state.terminal, ContentTerminal::Retryable | ContentTerminal::Failed | ContentTerminal::Uncertain)
    {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    issues
}

pub fn validate_content_event(profile: &ContentAdapterProfile, event: &ContentEvent) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    if event.schema != CONTENT_EVENT_SCHEMA {
        issues.push(ContentIssue::SchemaMismatch("content-event"));
    }
    validate_ref("content-event-operation-ref", &event.operation_ref, &mut issues);
    validate_ref("content-event-manifest-ref", &event.manifest_ref, &mut issues);
    if let Some(chunk_ref) = event.chunk_ref.as_deref() {
        validate_ref("content-event-chunk-ref", chunk_ref, &mut issues);
    }
    if event.observed_bytes > profile.bounds.max_total_bytes {
        issues.push(ContentIssue::TotalBytesExceeded);
    }
    if usize::try_from(event.sequence).map_or(true, |sequence| sequence >= profile.bounds.max_events) {
        issues.push(ContentIssue::EventLimitExceeded);
    }
    if matches!(event.terminal, ContentTerminal::Verified | ContentTerminal::Durable) && event.failure.is_some() {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    if event.failure.is_some()
        && !matches!(event.terminal, ContentTerminal::Retryable | ContentTerminal::Failed | ContentTerminal::Uncertain)
    {
        issues.push(ContentIssue::PartialStateMismatch);
    }
    validate_ref_list("content-event-evidence-ref", &event.evidence_refs, &mut issues);
    for required in REQUIRED_CONTENT_NON_CLAIMS {
        if !event.non_claims.contains(&required) {
            issues.push(ContentIssue::MissingNonClaim(required.as_str()));
        }
    }
    issues
}

pub fn validate_adapter_status(profile: &ContentAdapterProfile, status: &ContentAdapterStatus) -> Vec<ContentIssue> {
    let mut issues = validate_content_profile(profile);
    if status.schema != CONTENT_STATUS_SCHEMA {
        issues.push(ContentIssue::SchemaMismatch("content-adapter-status"));
    }
    if status.profile_ref != profile.profile_ref {
        issues.push(ContentIssue::ProfileMismatch);
    }
    if status.class != profile.class {
        issues.push(ContentIssue::AdapterMismatch);
    }
    if status.generation == 0 {
        issues.push(ContentIssue::ZeroBound("content-status-generation"));
    }
    if status.active_operations > profile.bounds.max_concurrent_operations {
        issues.push(ContentIssue::ConcurrencyExceeded);
    }
    if status.queued_bytes > profile.bounds.max_queued_bytes {
        issues.push(ContentIssue::QueueExceeded);
    }
    if status.terminal_counts.len() > profile.bounds.max_status_entries
        || status.issues.len() > profile.bounds.max_status_entries
    {
        issues.push(ContentIssue::EventLimitExceeded);
    }
    if status.terminal_counts.windows(CONTENT_ADJACENT_PAIR_WIDTH).any(|pair| pair[0].0 >= pair[1].0) {
        issues.push(ContentIssue::DuplicateValue("content-status-terminal-count"));
    }
    if let Some(hint_ref) = status.backend_hint_ref.as_deref() {
        validate_ref("content-backend-hint-ref", hint_ref, &mut issues);
    }
    for required in REQUIRED_CONTENT_NON_CLAIMS {
        if !status.non_claims.contains(&required) {
            issues.push(ContentIssue::MissingNonClaim(required.as_str()));
        }
    }
    issues
}

pub(crate) fn validate_command_shape(command: &ContentCommand, issues: &mut Vec<ContentIssue>) {
    if command.schema != CONTENT_COMMAND_SCHEMA {
        issues.push(ContentIssue::SchemaMismatch("content-command"));
    }
    for (field, reference) in [
        ("content-operation-ref", command.operation_ref.as_str()),
        ("content-adapter-ref", command.adapter_ref.as_str()),
        ("content-command-manifest-ref", command.manifest_ref.as_str()),
    ] {
        validate_ref(field, reference, issues);
    }
    validate_ref_list("content-command-policy-ref", &command.policy_refs, issues);
    if command.expected_bytes == 0 {
        issues.push(ContentIssue::ZeroBound("content-command-expected-bytes"));
    }
    if command.expected_chunks == 0 {
        issues.push(ContentIssue::ZeroBound("content-command-expected-chunks"));
    }
    if command.deadline_tick < command.submitted_tick {
        issues.push(ContentIssue::DeadlineExceeded);
    }
    if command.operation == ContentOperation::RangeRead && command.range.is_none() {
        issues.push(ContentIssue::RangeExceeded);
    }
    if command.operation != ContentOperation::RangeRead && command.range.is_some() {
        issues.push(ContentIssue::RangeExceeded);
    }
}

fn validate_partition(
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
    issues: &mut Vec<ContentIssue>,
) {
    let manifest_refs = manifest.chunks.iter().map(|chunk| chunk.chunk_ref.as_str()).collect::<Vec<_>>();
    let state_refs = state
        .verified_chunk_refs
        .iter()
        .chain(&state.missing_chunk_refs)
        .map(String::as_str)
        .collect::<Vec<_>>();
    if state_refs != manifest_refs {
        issues.push(ContentIssue::PartialStateMismatch);
    }
}

fn validate_positive_bounds(bounds: &ContentResourceBounds, issues: &mut Vec<ContentIssue>) {
    for (name, value) in [
        ("max-total-bytes", bounds.max_total_bytes),
        ("max-chunk-bytes", bounds.max_chunk_bytes),
        ("max-range-bytes", bounds.max_range_bytes),
        ("max-queued-bytes", bounds.max_queued_bytes),
        ("max-memory-bytes", bounds.max_memory_bytes),
        ("max-deadline-ticks", bounds.max_deadline_ticks),
    ] {
        if value == 0 {
            issues.push(ContentIssue::ZeroBound(name));
        }
    }
    for (name, value) in [
        ("max-chunk-count", bounds.max_chunk_count),
        ("max-concurrent-operations", bounds.max_concurrent_operations),
        ("max-events", bounds.max_events),
        ("max-status-entries", bounds.max_status_entries),
    ] {
        if value == 0 {
            issues.push(ContentIssue::ZeroBound(name));
        }
    }
}

fn validate_sorted_unique_capabilities(values: &[ContentCapability], issues: &mut Vec<ContentIssue>) {
    if values.is_empty() || values.windows(CONTENT_ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ContentIssue::DuplicateValue("content-capabilities"));
    }
}

fn validate_sorted_unique_tokens(field: &'static str, values: &[String], issues: &mut Vec<ContentIssue>) {
    if values.is_empty() || values.windows(CONTENT_ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ContentIssue::DuplicateValue(field));
    }
    for value in values {
        validate_token(field, value, issues);
    }
}

pub(crate) fn validate_ref_list(field: &'static str, values: &[String], issues: &mut Vec<ContentIssue>) {
    if values.windows(CONTENT_ADJACENT_PAIR_WIDTH).any(|pair| pair[0] >= pair[1]) {
        issues.push(ContentIssue::DuplicateValue(field));
    }
    for value in values {
        validate_ref(field, value, issues);
    }
}

fn validate_ref(field: &'static str, value: &str, issues: &mut Vec<ContentIssue>) {
    if !valid_blake3_ref(value) {
        issues.push(ContentIssue::MalformedRef(field));
    }
}

fn validate_token(field: &'static str, value: &str, issues: &mut Vec<ContentIssue>) {
    if value.is_empty() {
        issues.push(ContentIssue::EmptyField(field));
        return;
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/'))
    {
        issues.push(ContentIssue::MalformedToken(field));
    }
}
