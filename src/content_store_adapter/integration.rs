use molten_core::content_store_adapter::*;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;

pub fn content_adapter_profile(
    profile_id: &str,
    profile_ref: String,
    class: ContentAdapterClass,
    bounds: ContentResourceBounds,
    evidence_refs: Vec<String>,
) -> Result<ContentAdapterProfile> {
    let capabilities = match class {
        ContentAdapterClass::CapabilityLocal | ContentAdapterClass::RedbIndexed => vec![
            ContentCapability::StreamingPut,
            ContentCapability::StreamingGet,
            ContentCapability::VerifiedRange,
            ContentCapability::Availability,
            ContentCapability::Import,
            ContentCapability::Export,
            ContentCapability::Protection,
            ContentCapability::DurableCompletion,
        ],
        ContentAdapterClass::IrohBlobs => vec![
            ContentCapability::StreamingGet,
            ContentCapability::Availability,
            ContentCapability::Import,
            ContentCapability::Export,
            ContentCapability::Protection,
        ],
        ContentAdapterClass::DeterministicSimulation => vec![
            ContentCapability::StreamingPut,
            ContentCapability::StreamingGet,
            ContentCapability::VerifiedRange,
            ContentCapability::Availability,
            ContentCapability::Import,
            ContentCapability::Export,
            ContentCapability::Protection,
            ContentCapability::DurableCompletion,
        ],
    };
    let mut evidence_refs = evidence_refs;
    evidence_refs.sort();
    evidence_refs.dedup();
    let profile = ContentAdapterProfile {
        schema: CONTENT_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: profile_id.to_string(),
        profile_ref,
        class,
        capabilities,
        bounds,
        supported_transforms: vec!["identity".to_string()],
        evidence_refs,
        non_claims: REQUIRED_CONTENT_NON_CLAIMS.to_vec(),
    };
    let issues = validate_content_profile(&profile);
    if issues.is_empty() {
        Ok(profile)
    } else {
        Err(MoltenError::invalid_harness(format!("content adapter profile denied: {issues:?}")))
    }
}

pub struct ContentCommandInput<'a> {
    pub operation_ref: String,
    pub operation: ContentOperation,
    pub manifest: &'a ContentManifestDescriptor,
    pub range: Option<ContentRange>,
    pub submitted_tick: u64,
    pub deadline_tick: u64,
    pub retry_count: u32,
    pub cancelled: bool,
    pub policy_refs: Vec<String>,
}

pub fn content_command(profile: &ContentAdapterProfile, input: ContentCommandInput<'_>) -> Result<ContentCommand> {
    let (expected_bytes, expected_chunks) = expected_shape(input.manifest, input.range)?;
    let mut policy_refs = input.policy_refs;
    policy_refs.sort();
    policy_refs.dedup();
    Ok(ContentCommand {
        schema: CONTENT_COMMAND_SCHEMA.to_string(),
        operation_ref: input.operation_ref,
        adapter_ref: profile.profile_ref.clone(),
        operation: input.operation,
        manifest_ref: input.manifest.manifest_ref.clone(),
        range: input.range,
        expected_bytes,
        expected_chunks,
        submitted_tick: input.submitted_tick,
        deadline_tick: input.deadline_tick,
        retry_count: input.retry_count,
        cancelled: input.cancelled,
        policy_refs,
    })
}

pub fn assemble_verified_content(
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
    chunks: &[VerifiedChunkPayload],
) -> Result<Vec<u8>> {
    if !content_is_available(manifest, state) {
        return Err(MoltenError::invalid_harness("content cannot be assembled before complete verification"));
    }
    if chunks.len() != manifest.chunks.len() {
        return Err(MoltenError::invalid_harness("verified payload count does not match manifest"));
    }
    let manifest_chunk_size = usize::try_from(manifest.chunk_size)
        .map_err(|_| MoltenError::invalid_harness("manifest chunk size does not fit usize"))?;
    let mut bytes = Vec::new();
    for (position, (descriptor, payload)) in manifest.chunks.iter().zip(chunks).enumerate() {
        if payload.position != position || payload.chunk_ref != descriptor.chunk_ref {
            return Err(MoltenError::invalid_harness("verified payload ordering does not match manifest"));
        }
        let payload_length = u64::try_from(payload.bytes.len())
            .map_err(|_| MoltenError::invalid_harness("verified payload length does not fit u64"))?;
        if payload_length != descriptor.length
            || crate::chunk_store::hash_chunk(&payload.bytes, manifest_chunk_size) != descriptor.chunk_ref
        {
            return Err(MoltenError::invalid_harness("verified payload no longer matches canonical chunk identity"));
        }
        bytes.extend_from_slice(&payload.bytes);
    }
    if u64::try_from(bytes.len()).ok() != Some(manifest.total_length) {
        return Err(MoltenError::invalid_harness("assembled content length does not match manifest"));
    }
    Ok(bytes)
}

pub fn backend_protection_status(
    profile: &ContentAdapterProfile,
    manifest_ref: &str,
    protected: bool,
) -> Result<CanonicalContentArtifact<ContentEvent>> {
    let terminal = ContentTerminal::Verified;
    let event = ContentEvent {
        schema: CONTENT_EVENT_SCHEMA.to_string(),
        operation_ref: crate::preserves_rail::content_ref_from_bytes(
            format!("protection\0{manifest_ref}\0{protected}").as_bytes(),
        ),
        manifest_ref: manifest_ref.to_string(),
        sequence: 0,
        terminal,
        chunk_ref: None,
        observed_bytes: 0,
        failure: None,
        evidence_refs: Vec::new(),
        non_claims: REQUIRED_CONTENT_NON_CLAIMS.to_vec(),
    };
    canonical_content_event(profile, &event)
}

fn expected_shape(manifest: &ContentManifestDescriptor, range: Option<ContentRange>) -> Result<(u64, usize)> {
    match range {
        Some(range) => {
            let refs = required_chunks_for_range(manifest, range)
                .map_err(|issue| MoltenError::invalid_harness(format!("content range denied: {issue:?}")))?;
            Ok((range.length, refs.len()))
        }
        None => Ok((manifest.total_length, manifest.chunks.len())),
    }
}
