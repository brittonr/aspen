use molten_core::content_store_adapter::*;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedChunkPayload {
    pub chunk_ref: String,
    pub position: usize,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalContentPutExecution {
    pub manifest: ContentManifestDescriptor,
    pub event: CanonicalContentArtifact<ContentEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalContentExecution {
    pub manifest: ContentManifestDescriptor,
    pub state: CanonicalContentArtifact<ContentPartialState>,
    pub events: Vec<CanonicalContentArtifact<ContentEvent>>,
    pub verified_chunks: Vec<VerifiedChunkPayload>,
    pub backend_hint_ref: String,
}

pub fn manifest_descriptor(manifest: &crate::chunk_store::ChunkManifest) -> ContentManifestDescriptor {
    ContentManifestDescriptor {
        manifest_ref: manifest.manifest_ref.clone(),
        total_length: manifest.total_len,
        chunker: manifest.chunker.clone(),
        chunk_size: manifest.chunk_size,
        metadata_ref: manifest.metadata_ref.clone(),
        policy_refs: sorted_refs(manifest.policy_refs.clone()),
        evidence_refs: sorted_refs(manifest.evidence_refs.clone()),
        chunks: manifest
            .chunks
            .iter()
            .enumerate()
            .map(|(position, chunk)| ContentChunkDescriptor {
                chunk_ref: chunk.chunk_ref.clone(),
                length: chunk.length,
                position,
                transform: chunk.transforms.ordering.clone(),
            })
            .collect(),
    }
}

pub struct StreamPutInput<'a> {
    pub profile: &'a ContentAdapterProfile,
    pub root: &'a crate::chunk_store::CapabilityChunkRoot,
    pub command: &'a ContentCommand,
    pub expected_manifest: &'a ContentManifestDescriptor,
    pub object_kind: &'a str,
    pub bytes: &'a [u8],
}

// r[impl molten.content_store_adapter.port_contract]
// r[impl molten.content_store_adapter.streaming_bounds]
// r[impl molten.content_store_adapter.verify_before_available]
pub fn execute_local_stream_put(input: StreamPutInput<'_>) -> crate::error::Result<LocalContentPutExecution> {
    let StreamPutInput {
        profile,
        root,
        command,
        expected_manifest,
        object_kind,
        bytes,
    } = input;
    if command.operation != ContentOperation::Put && command.operation != ContentOperation::Import {
        return Err(crate::error::MoltenError::invalid_harness("local stream put requires put or import operation"));
    }
    let preflight = preflight_content_operation(profile, expected_manifest, command, 0, 0);
    require_accepted("local content put", &preflight)?;
    verify_source_bytes(expected_manifest, bytes)?;
    let put = crate::chunk_store::put_bytes_with_root(root, object_kind, bytes, expected_manifest.chunk_size)?;
    if put.manifest_ref != expected_manifest.manifest_ref {
        return Err(crate::error::MoltenError::invalid_harness(
            "local put result does not match expected canonical manifest identity",
        ));
    }
    let stored = crate::chunk_store::read_manifest_with_root(root, &put.manifest_ref)?;
    let manifest = manifest_descriptor(&stored);
    if &manifest != expected_manifest {
        return Err(crate::error::MoltenError::invalid_harness(
            "local put readback differs from expected canonical manifest",
        ));
    }
    let event = content_event(EventInput {
        command,
        sequence: 0,
        terminal: ContentTerminal::Durable,
        chunk_ref: None,
        observed_bytes: manifest.total_length,
        failure: None,
        evidence_refs: &manifest.evidence_refs,
    });
    Ok(LocalContentPutExecution {
        manifest,
        event: canonical_content_event(profile, &event)?,
    })
}

// r[impl molten.content_store_adapter.port_contract]
// r[impl molten.content_store_adapter.streaming_bounds]
// r[impl molten.content_store_adapter.verify_before_available]
pub fn execute_local_stream_get(
    profile: &ContentAdapterProfile,
    root: &crate::chunk_store::CapabilityChunkRoot,
    command: &ContentCommand,
    generation: u64,
    retained: Option<&ContentPartialState>,
) -> crate::error::Result<LocalContentExecution> {
    if command.operation != ContentOperation::Get && command.operation != ContentOperation::Export {
        return Err(crate::error::MoltenError::invalid_harness("local stream get requires get or export operation"));
    }
    let source_manifest = crate::chunk_store::read_manifest_with_root(root, &command.manifest_ref)?;
    let manifest = manifest_descriptor(&source_manifest);
    let preflight = preflight_content_operation(profile, &manifest, command, 0, 0);
    require_accepted("local content get", &preflight)?;
    let mut state = begin_partial_state(profile, &manifest, command, generation, retained)
        .map_err(|issues| validation_error("local partial state", &issues))?;
    let resume_position = state.verified_chunk_refs.len();
    let mut verified_chunks = Vec::with_capacity(source_manifest.chunks.len().saturating_sub(resume_position));
    let mut events = Vec::with_capacity(verified_chunks.capacity());
    for (position, chunk) in source_manifest.chunks.iter().enumerate().skip(resume_position) {
        let manifest_chunk_size = usize::try_from(source_manifest.chunk_size).map_err(|_| {
            crate::error::MoltenError::invalid_harness("content manifest chunk size does not fit usize")
        })?;
        let bytes = crate::chunk_store::read_verified_chunk(root, chunk, manifest_chunk_size)?;
        let observed_content_ref = crate::chunk_store::hash_chunk(&bytes, manifest_chunk_size);
        let sequence = state
            .last_sequence
            .map_or(Some(0), |value| value.checked_add(1))
            .ok_or_else(|| crate::error::MoltenError::invalid_harness("local content event sequence overflow"))?;
        let observation = ContentChunkObservation {
            operation_ref: command.operation_ref.clone(),
            manifest_ref: manifest.manifest_ref.clone(),
            sequence,
            chunk_ref: chunk.chunk_ref.clone(),
            position,
            observed_content_ref,
            observed_length: u64::try_from(bytes.len())
                .map_err(|_| crate::error::MoltenError::invalid_harness("content chunk length does not fit u64"))?,
        };
        state = apply_chunk_observation(profile, &manifest, &state, &observation)
            .map_err(|issues| validation_error("local content chunk", &issues))?;
        let event = content_event(EventInput {
            command,
            sequence,
            terminal: state.terminal,
            chunk_ref: Some(chunk.chunk_ref.clone()),
            observed_bytes: observation.observed_length,
            failure: None,
            evidence_refs: &source_manifest.evidence_refs,
        });
        events.push(canonical_content_event(profile, &event)?);
        verified_chunks.push(VerifiedChunkPayload {
            chunk_ref: chunk.chunk_ref.clone(),
            position,
            bytes,
        });
    }
    if !content_is_available(&manifest, &state) {
        return Err(crate::error::MoltenError::invalid_harness(
            "local content get ended without complete verification",
        ));
    }
    let state = if profile.capabilities.contains(&ContentCapability::DurableCompletion) {
        mark_content_durable(profile, &state).map_err(|issue| validation_error("local durability", &[issue]))?
    } else {
        state
    };
    let state = canonical_partial_state(profile, &manifest, &state)?;
    Ok(LocalContentExecution {
        manifest,
        state,
        events,
        verified_chunks,
        backend_hint_ref: backend_hint_ref(profile.class, &command.manifest_ref),
    })
}

pub fn execute_local_verified_range(
    profile: &ContentAdapterProfile,
    root: &crate::chunk_store::CapabilityChunkRoot,
    command: &ContentCommand,
) -> crate::error::Result<Vec<u8>> {
    if command.operation != ContentOperation::RangeRead {
        return Err(crate::error::MoltenError::invalid_harness("local range adapter requires range-read operation"));
    }
    let source_manifest = crate::chunk_store::read_manifest_with_root(root, &command.manifest_ref)?;
    let manifest = manifest_descriptor(&source_manifest);
    let preflight = preflight_content_operation(profile, &manifest, command, 0, 0);
    require_accepted("local content range", &preflight)?;
    let range = command
        .range
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("local range command lacks range"))?;
    let read = crate::chunk_store::range_read_with_root(root, &manifest.manifest_ref, range.offset, range.length)?;
    if u64::try_from(read.bytes.len()).ok() != Some(range.length) {
        return Err(crate::error::MoltenError::invalid_harness("verified range adapter returned unexpected length"));
    }
    Ok(read.bytes)
}

pub(crate) struct EventInput<'a> {
    pub(crate) command: &'a ContentCommand,
    pub(crate) sequence: u64,
    pub(crate) terminal: ContentTerminal,
    pub(crate) chunk_ref: Option<String>,
    pub(crate) observed_bytes: u64,
    pub(crate) failure: Option<ContentFailure>,
    pub(crate) evidence_refs: &'a [String],
}

pub(crate) fn content_event(input: EventInput<'_>) -> ContentEvent {
    ContentEvent {
        schema: CONTENT_EVENT_SCHEMA.to_string(),
        operation_ref: input.command.operation_ref.clone(),
        manifest_ref: input.command.manifest_ref.clone(),
        sequence: input.sequence,
        terminal: input.terminal,
        chunk_ref: input.chunk_ref,
        observed_bytes: input.observed_bytes,
        failure: input.failure,
        evidence_refs: sorted_refs(input.evidence_refs.to_vec()),
        non_claims: REQUIRED_CONTENT_NON_CLAIMS.to_vec(),
    }
}

pub(crate) fn backend_hint_ref(class: ContentAdapterClass, backend_label: &str) -> String {
    let hint = format!("{}\0{backend_label}", class.as_str());
    crate::preserves_rail::content_ref_from_bytes(hint.as_bytes())
}

fn verify_source_bytes(manifest: &ContentManifestDescriptor, bytes: &[u8]) -> crate::error::Result<()> {
    if u64::try_from(bytes.len()).ok() != Some(manifest.total_length) {
        return Err(crate::error::MoltenError::invalid_harness("put source length does not match expected manifest"));
    }
    let chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|_| crate::error::MoltenError::invalid_harness("put chunk size does not fit usize"))?;
    for (descriptor, chunk_bytes) in manifest.chunks.iter().zip(bytes.chunks(chunk_size)) {
        if u64::try_from(chunk_bytes.len()).ok() != Some(descriptor.length)
            || crate::chunk_store::hash_chunk(chunk_bytes, chunk_size) != descriptor.chunk_ref
        {
            return Err(crate::error::MoltenError::invalid_harness(
                "put source chunk does not match expected manifest",
            ));
        }
    }
    Ok(())
}

fn require_accepted(label: &str, preflight: &ContentPreflight) -> crate::error::Result<()> {
    if preflight.terminal == ContentTerminal::Accepted && preflight.issues.is_empty() {
        Ok(())
    } else {
        Err(validation_error(label, &preflight.issues))
    }
}

fn validation_error(label: &str, issues: &[ContentIssue]) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} denied: {issues:?}"))
}

fn sorted_refs(mut refs: Vec<String>) -> Vec<String> {
    refs.sort();
    refs.dedup();
    refs
}
