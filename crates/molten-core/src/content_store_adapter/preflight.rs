use super::*;

// r[impl molten.content_store_adapter.port_contract]
// r[impl molten.content_store_adapter.streaming_bounds]
pub fn preflight_content_operation(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    active_operations: u64,
    queued_bytes: u64,
) -> ContentPreflight {
    let mut issues = validate_content_profile(profile);
    if manifest.chunks.len() > profile.bounds.max_chunk_count {
        issues.push(ContentIssue::ChunkCountExceeded);
        let terminal = if command.cancelled || command.operation == ContentOperation::Cancel {
            issues.push(ContentIssue::Cancelled);
            ContentTerminal::Cancelled
        } else {
            ContentTerminal::Denied
        };
        return ContentPreflight {
            terminal,
            required_chunk_refs: Vec::new(),
            issues,
        };
    }
    issues.extend(validate_manifest_descriptor(manifest));
    validate_command_shape(command, &mut issues);
    validate_binding(profile, manifest, command, &mut issues);
    validate_resources(
        profile,
        manifest,
        command,
        ResourceUsage {
            active_operations,
            queued_bytes,
        },
        &mut issues,
    );
    let required_chunk_refs = required_chunks(profile, manifest, command, &mut issues);
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
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    range: ContentRange,
) -> Result<Vec<String>, ContentIssue> {
    let mut required = Vec::new();
    visit_required_chunks(profile, manifest, range, |chunk| required.push(chunk.chunk_ref.clone()))?;
    Ok(required)
}

pub fn required_chunk_count_for_range(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    range: ContentRange,
) -> Result<u64, ContentIssue> {
    let mut count = 0_u64;
    visit_required_chunks(profile, manifest, range, |_| count += 1)?;
    Ok(count)
}

fn visit_required_chunks(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    range: ContentRange,
    mut visit: impl FnMut(&ContentChunkDescriptor),
) -> Result<(), ContentIssue> {
    if manifest.chunks.len() > profile.bounds.max_chunk_count {
        return Err(ContentIssue::ChunkCountExceeded);
    }
    if range.length == 0 {
        return Err(ContentIssue::RangeExceeded);
    }
    let end_bytes = range.offset.checked_add(range.length).ok_or(ContentIssue::ArithmeticOverflow)?;
    if end_bytes > manifest.total_length {
        return Err(ContentIssue::RangeExceeded);
    }
    if u64::try_from(manifest.chunks.len()).map_or(true, |count| count > manifest.total_length) {
        return Err(ContentIssue::ChunkCountExceeded);
    }
    let mut offset_bytes = 0_u64;
    for chunk in &manifest.chunks {
        if chunk.length == 0 {
            return Err(ContentIssue::ZeroBound("content-chunk-length"));
        }
        let chunk_end_bytes = offset_bytes.checked_add(chunk.length).ok_or(ContentIssue::ArithmeticOverflow)?;
        if range.offset < chunk_end_bytes && offset_bytes < end_bytes {
            visit(chunk);
        }
        offset_bytes = chunk_end_bytes;
    }
    Ok(())
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

struct ResourceUsage {
    active_operations: u64,
    queued_bytes: u64,
}

fn validate_resources(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    usage: ResourceUsage,
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
    if usage.active_operations >= profile.bounds.max_concurrent_operations {
        issues.push(ContentIssue::ConcurrencyExceeded);
    }
    match usage.queued_bytes.checked_add(command.expected_bytes) {
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
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    issues: &mut Vec<ContentIssue>,
) -> Vec<String> {
    match command.range {
        Some(range) => match required_chunks_for_range(profile, manifest, range) {
            Ok(required) => required,
            Err(issue) => {
                issues.push(issue);
                Vec::new()
            }
        },
        None => manifest.chunks.iter().map(|chunk| chunk.chunk_ref.clone()).collect(),
    }
}
