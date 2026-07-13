use super::*;

// r[impl molten.content_store_adapter.port_contract]
// r[impl molten.content_store_adapter.streaming_bounds]
pub fn preflight_content_operation(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    active_operations: usize,
    queued_bytes: u64,
) -> ContentPreflight {
    let mut issues = validate_content_profile(profile);
    issues.extend(validate_manifest_descriptor(manifest));
    validate_command_shape(command, &mut issues);
    validate_binding(profile, manifest, command, &mut issues);
    validate_resources(profile, manifest, command, active_operations, queued_bytes, &mut issues);
    let required_chunk_refs = required_chunks(manifest, command, &mut issues);
    let terminal = if command.cancelled || command.operation == ContentOperation::Cancel {
        issues.push(ContentIssue::Cancelled);
        ContentTerminal::Cancelled
    } else if issues.is_empty() {
        ContentTerminal::Accepted
    } else {
        ContentTerminal::Denied
    };
    ContentPreflight {
        terminal,
        required_chunk_refs,
        issues,
    }
}

pub fn required_chunks_for_range(
    manifest: &ContentManifestDescriptor,
    range: ContentRange,
) -> Result<Vec<String>, ContentIssue> {
    if range.length == 0 {
        return Err(ContentIssue::RangeExceeded);
    }
    let end = range.offset.checked_add(range.length).ok_or(ContentIssue::ArithmeticOverflow)?;
    if end > manifest.total_length {
        return Err(ContentIssue::RangeExceeded);
    }
    let mut offset = 0_u64;
    let mut required = Vec::new();
    for chunk in &manifest.chunks {
        let chunk_end = offset.checked_add(chunk.length).ok_or(ContentIssue::ArithmeticOverflow)?;
        if ranges_overlap(range.offset, end, offset, chunk_end) {
            required.push(chunk.chunk_ref.clone());
        }
        offset = chunk_end;
    }
    Ok(required)
}

fn validate_binding(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    issues: &mut Vec<ContentIssue>,
) {
    if command.adapter_ref != profile.profile_ref {
        issues.push(ContentIssue::AdapterMismatch);
    }
    if command.manifest_ref != manifest.manifest_ref {
        issues.push(ContentIssue::ManifestMismatch);
    }
    if let Some(capability) = command.operation.required_capability()
        && !profile.capabilities.contains(&capability)
    {
        issues.push(ContentIssue::UnsupportedCapability);
    }
    for chunk in &manifest.chunks {
        if !profile.supported_transforms.contains(&chunk.transform) {
            issues.push(ContentIssue::UnsupportedTransform(chunk.transform.clone()));
        }
    }
}

fn validate_resources(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    active_operations: usize,
    queued_bytes: u64,
    issues: &mut Vec<ContentIssue>,
) {
    if manifest.total_length > profile.bounds.max_total_bytes || command.expected_bytes > profile.bounds.max_total_bytes
    {
        issues.push(ContentIssue::TotalBytesExceeded);
    }
    if manifest.chunks.len() > profile.bounds.max_chunk_count
        || command.expected_chunks > profile.bounds.max_chunk_count
    {
        issues.push(ContentIssue::ChunkCountExceeded);
    }
    if manifest.chunks.iter().any(|chunk| chunk.length > profile.bounds.max_chunk_bytes) {
        issues.push(ContentIssue::ChunkBytesExceeded);
    }
    if active_operations >= profile.bounds.max_concurrent_operations {
        issues.push(ContentIssue::ConcurrencyExceeded);
    }
    match queued_bytes.checked_add(command.expected_bytes) {
        Some(total) if total > profile.bounds.max_queued_bytes => issues.push(ContentIssue::QueueExceeded),
        Some(_) => {}
        None => issues.push(ContentIssue::ArithmeticOverflow),
    }
    if command.expected_bytes > profile.bounds.max_memory_bytes {
        issues.push(ContentIssue::MemoryExceeded);
    }
    match command.submitted_tick.checked_add(profile.bounds.max_deadline_ticks) {
        Some(maximum) if command.deadline_tick > maximum => issues.push(ContentIssue::DeadlineExceeded),
        Some(_) => {}
        None => issues.push(ContentIssue::ArithmeticOverflow),
    }
    if command.retry_count > profile.bounds.max_retries {
        issues.push(ContentIssue::RetryExceeded);
    }
    if let Some(range) = command.range
        && range.length > profile.bounds.max_range_bytes
    {
        issues.push(ContentIssue::RangeExceeded);
    }
}

fn required_chunks(
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    issues: &mut Vec<ContentIssue>,
) -> Vec<String> {
    match command.range {
        Some(range) => match required_chunks_for_range(manifest, range) {
            Ok(required) => required,
            Err(issue) => {
                issues.push(issue);
                Vec::new()
            }
        },
        None => manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect(),
    }
}

const fn ranges_overlap(left_start: u64, left_end: u64, right_start: u64, right_end: u64) -> bool {
    left_start < right_end && right_start < left_end
}
