use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::*;

const REF_HEX_CHARS: usize = 16;
const REF_REPETITIONS: usize = 4;
const GENERATION_ONE: u64 = 1;
const SUBMITTED_TICK: u64 = 10;
const DEADLINE_TICK: u64 = 20;
const MAX_TOTAL_BYTES: u64 = 1_024;
const MAX_CHUNK_COUNT: usize = 16;
const MAX_CHUNK_BYTES: u64 = 256;
const MAX_RANGE_BYTES: u64 = 512;
const MAX_CONCURRENT_OPERATIONS: usize = 4;
const MAX_QUEUED_BYTES: u64 = 2_048;
const MAX_MEMORY_BYTES: u64 = 1_024;
const MAX_DEADLINE_TICKS: u64 = 20;
const MAX_RETRIES: u32 = 2;
const MAX_EVENTS: usize = 32;
const MAX_STATUS_ENTRIES: usize = 16;
const CHUNK_LENGTH: u64 = 4;
const TOTAL_LENGTH: u64 = CHUNK_LENGTH * 2;
const TRUNCATED_CHUNK_LENGTH: u64 = CHUNK_LENGTH - 1;
const QUEUED_AT_LIMIT: u64 = MAX_QUEUED_BYTES - TOTAL_LENGTH;

fn test_ref(label: &str) -> String {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let chunk = format!("{:0REF_HEX_CHARS$x}", hasher.finish());
    format!("blake3:{}", chunk.repeat(REF_REPETITIONS))
}

fn profile() -> ContentAdapterProfile {
    ContentAdapterProfile {
        schema: CONTENT_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "local-bounded-v1".to_string(),
        profile_ref: test_ref("profile"),
        class: ContentAdapterClass::CapabilityLocal,
        capabilities: vec![
            ContentCapability::StreamingPut,
            ContentCapability::StreamingGet,
            ContentCapability::VerifiedRange,
            ContentCapability::Availability,
            ContentCapability::Import,
            ContentCapability::Export,
            ContentCapability::Protection,
            ContentCapability::DurableCompletion,
        ],
        bounds: ContentResourceBounds {
            max_total_bytes: MAX_TOTAL_BYTES,
            max_chunk_count: MAX_CHUNK_COUNT,
            max_chunk_bytes: MAX_CHUNK_BYTES,
            max_range_bytes: MAX_RANGE_BYTES,
            max_concurrent_operations: MAX_CONCURRENT_OPERATIONS,
            max_queued_bytes: MAX_QUEUED_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_deadline_ticks: MAX_DEADLINE_TICKS,
            max_retries: MAX_RETRIES,
            max_events: MAX_EVENTS,
            max_status_entries: MAX_STATUS_ENTRIES,
        },
        supported_transforms: vec!["identity".to_string()],
        evidence_refs: vec![test_ref("profile-evidence")],
        non_claims: REQUIRED_CONTENT_NON_CLAIMS.to_vec(),
    }
}

fn manifest() -> ContentManifestDescriptor {
    ContentManifestDescriptor {
        manifest_ref: test_ref("manifest"),
        total_length: TOTAL_LENGTH,
        chunker: "fixed-v1".to_string(),
        chunk_size: CHUNK_LENGTH,
        metadata_ref: test_ref("metadata"),
        policy_refs: vec![test_ref("manifest-policy")],
        evidence_refs: vec![test_ref("manifest-evidence")],
        chunks: vec![
            ContentChunkDescriptor {
                chunk_ref: test_ref("chunk-a"),
                length: CHUNK_LENGTH,
                position: 0,
                transform: "identity".to_string(),
            },
            ContentChunkDescriptor {
                chunk_ref: test_ref("chunk-b"),
                length: CHUNK_LENGTH,
                position: 1,
                transform: "identity".to_string(),
            },
        ],
    }
}

fn command(operation: ContentOperation) -> ContentCommand {
    ContentCommand {
        schema: CONTENT_COMMAND_SCHEMA.to_string(),
        operation_ref: test_ref("operation"),
        adapter_ref: profile().profile_ref,
        operation,
        manifest_ref: manifest().manifest_ref,
        range: (operation == ContentOperation::RangeRead).then_some(ContentRange {
            offset: CHUNK_LENGTH,
            length: CHUNK_LENGTH,
        }),
        expected_bytes: TOTAL_LENGTH,
        expected_chunks: manifest().chunks.len(),
        submitted_tick: SUBMITTED_TICK,
        deadline_tick: DEADLINE_TICK,
        retry_count: 0,
        cancelled: false,
        policy_refs: vec![test_ref("operation-policy")],
    }
}

// r[verify molten.content_store_adapter.port_contract]
// r[verify molten.content_store_adapter.streaming_bounds]
#[test]
fn preflight_is_bounded_capability_checked_and_range_specific() {
    let profile = profile();
    let manifest = manifest();
    let range = preflight_content_operation(&profile, &manifest, &command(ContentOperation::RangeRead), 0, 0);
    assert_eq!(range.terminal, ContentTerminal::Accepted);
    assert_eq!(range.required_chunk_refs, vec![manifest.chunks[1].chunk_ref.clone()]);

    let mut unsupported = profile.clone();
    unsupported.capabilities.retain(|capability| *capability != ContentCapability::VerifiedRange);
    let denied = preflight_content_operation(&unsupported, &manifest, &command(ContentOperation::RangeRead), 0, 0);
    assert_eq!(denied.terminal, ContentTerminal::Denied);
    assert!(denied.issues.contains(&ContentIssue::UnsupportedCapability));

    let overloaded =
        preflight_content_operation(&profile, &manifest, &command(ContentOperation::Get), MAX_CONCURRENT_OPERATIONS, 0);
    assert!(overloaded.issues.contains(&ContentIssue::ConcurrencyExceeded));
}

// r[verify molten.content_store_adapter.streaming_bounds]
// r[verify molten.content_store_adapter.verify_before_available]
#[test]
fn unsupported_transform_overbound_queue_reordering_and_truncation_deny_before_availability() {
    let profile = profile();
    let mut unsupported_manifest = manifest();
    unsupported_manifest.chunks[0].transform = "zstd-placeholder".to_string();
    let unsupported =
        preflight_content_operation(&profile, &unsupported_manifest, &command(ContentOperation::Get), 0, 0);
    assert!(unsupported.issues.contains(&ContentIssue::UnsupportedTransform("zstd-placeholder".to_string())));

    let exact_queue =
        preflight_content_operation(&profile, &manifest(), &command(ContentOperation::Get), 0, QUEUED_AT_LIMIT);
    assert_eq!(exact_queue.terminal, ContentTerminal::Accepted);
    let over_queue =
        preflight_content_operation(&profile, &manifest(), &command(ContentOperation::Get), 0, QUEUED_AT_LIMIT + 1);
    assert!(over_queue.issues.contains(&ContentIssue::QueueExceeded));

    let manifest = manifest();
    let command = command(ContentOperation::Get);
    let state = begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, None).expect("state");
    let reordered = ContentChunkObservation {
        operation_ref: command.operation_ref.clone(),
        manifest_ref: manifest.manifest_ref.clone(),
        sequence: 0,
        chunk_ref: manifest.chunks[1].chunk_ref.clone(),
        position: 1,
        observed_content_ref: manifest.chunks[1].chunk_ref.clone(),
        observed_length: CHUNK_LENGTH,
    };
    assert!(
        apply_chunk_observation(&profile, &manifest, &state, &reordered)
            .expect_err("reordered chunk denied")
            .iter()
            .any(|issue| matches!(issue, ContentIssue::ReorderedChunk(_)))
    );
    let truncated = ContentChunkObservation {
        chunk_ref: manifest.chunks[0].chunk_ref.clone(),
        position: 0,
        observed_content_ref: manifest.chunks[0].chunk_ref.clone(),
        observed_length: TRUNCATED_CHUNK_LENGTH,
        ..reordered
    };
    assert!(
        apply_chunk_observation(&profile, &manifest, &state, &truncated)
            .expect_err("truncated chunk denied")
            .contains(&ContentIssue::TruncatedChunk(manifest.chunks[0].chunk_ref.clone()))
    );
    assert!(!content_is_available(&manifest, &state));
}

// r[verify molten.content_store_adapter.verify_before_available]
// r[verify molten.content_store_adapter.partial_state]
#[test]
fn verification_advances_in_order_and_corruption_never_becomes_available() {
    let profile = profile();
    let manifest = manifest();
    let command = command(ContentOperation::Get);
    let state = begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, None).expect("initial state");
    assert!(!content_is_available(&manifest, &state));
    let first = ContentChunkObservation {
        operation_ref: command.operation_ref.clone(),
        manifest_ref: manifest.manifest_ref.clone(),
        sequence: 0,
        chunk_ref: manifest.chunks[0].chunk_ref.clone(),
        position: 0,
        observed_content_ref: manifest.chunks[0].chunk_ref.clone(),
        observed_length: CHUNK_LENGTH,
    };
    let state = apply_chunk_observation(&profile, &manifest, &state, &first).expect("first chunk");
    assert_eq!(state.terminal, ContentTerminal::Streaming);
    let mut corrupt = first;
    corrupt.sequence = 1;
    corrupt.chunk_ref = manifest.chunks[1].chunk_ref.clone();
    corrupt.position = 1;
    corrupt.observed_content_ref = test_ref("corrupt");
    assert!(
        apply_chunk_observation(&profile, &manifest, &state, &corrupt)
            .expect_err("corruption denied")
            .contains(&ContentIssue::CorruptChunk(manifest.chunks[1].chunk_ref.clone()))
    );
    assert!(!content_is_available(&manifest, &state));
}

// r[verify molten.content_store_adapter.partial_state]
#[test]
fn verified_partial_state_resumes_and_disconnect_is_uncertain() {
    let profile = profile();
    let manifest = manifest();
    let command = command(ContentOperation::Get);
    let state = begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, None).expect("initial state");
    let first = ContentChunkObservation {
        operation_ref: command.operation_ref.clone(),
        manifest_ref: manifest.manifest_ref.clone(),
        sequence: 0,
        chunk_ref: manifest.chunks[0].chunk_ref.clone(),
        position: 0,
        observed_content_ref: manifest.chunks[0].chunk_ref.clone(),
        observed_length: CHUNK_LENGTH,
    };
    let partial = apply_chunk_observation(&profile, &manifest, &state, &first).expect("partial");
    let resumed = begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, Some(&partial)).expect("resume");
    assert_eq!(resumed.missing_chunk_refs, vec![manifest.chunks[1].chunk_ref.clone()]);
    assert_eq!(
        classify_content_failure(&profile, &resumed, ContentFailure::TransportDisconnected)
            .expect("disconnect outcome")
            .terminal,
        ContentTerminal::Uncertain
    );

    let mut stale = resumed;
    stale.generation += 1;
    assert!(begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, Some(&stale)).is_err());
}

// r[verify molten.content_store_adapter.identity_boundary]
#[test]
fn repeated_chunk_refs_preserve_order_without_changing_manifest_identity() {
    let profile = profile();
    let mut manifest = manifest();
    manifest.chunks[1].chunk_ref = manifest.chunks[0].chunk_ref.clone();
    assert!(validate_manifest_descriptor(&manifest).is_empty());
    let command = command(ContentOperation::Get);
    let mut command = ContentCommand {
        manifest_ref: manifest.manifest_ref.clone(),
        ..command
    };
    command.expected_chunks = manifest.chunks.len();
    let state = begin_partial_state(&profile, &manifest, &command, GENERATION_ONE, None).expect("repeated state");
    assert_eq!(state.missing_chunk_refs.len(), manifest.chunks.len());
}

// r[verify molten.content_store_adapter.retention_boundary]
#[test]
fn protection_handles_pins_and_unprotect_never_grant_read_or_delete_authority() {
    assert_eq!(backend_protection_effect_grants_authority(), ContentAuthorityDecision::Deny);
    assert_eq!(admit_content_read(None), ContentAuthorityDecision::Deny);
    assert_eq!(admit_content_deletion(None), ContentAuthorityDecision::Deny);
    let authority = ContentProtectionAuthority {
        schema: CONTENT_PROTECTION_AUTHORITY_SCHEMA.to_string(),
        operation_ref: test_ref("authority-operation"),
        manifest_ref: manifest().manifest_ref,
        retention_policy_ref: test_ref("retention-policy"),
        canonical_pin_ref: Some(test_ref("pin")),
        read_authority_ref: None,
        deletion_gate_ref: Some(test_ref("deletion-gate")),
    };
    assert_eq!(admit_content_read(Some(&authority)), ContentAuthorityDecision::Deny);
    assert_eq!(admit_content_deletion(Some(&authority)), ContentAuthorityDecision::Deny);
}
