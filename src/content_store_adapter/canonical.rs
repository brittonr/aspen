use molten_core::content_store_adapter::*;
use molten_core::fabric::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;

const PROFILE_RECORD: &str = "content-store-adapter-profile-v1";
const COMMAND_RECORD: &str = "content-store-adapter-command-v1";
const EVENT_RECORD: &str = "content-store-adapter-event-v1";
const PARTIAL_RECORD: &str = "content-store-adapter-partial-state-v1";
const STATUS_RECORD: &str = "content-store-adapter-status-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalContentArtifact<T> {
    pub artifact: T,
    pub artifact_ref: String,
    pub value: IOValue,
}

// r[impl molten.content_store_adapter.port_contract]
pub fn canonical_content_profile(
    profile: &ContentAdapterProfile,
) -> Result<CanonicalContentArtifact<ContentAdapterProfile>> {
    require_valid("content adapter profile", &validate_content_profile(profile))?;
    canonical_artifact(profile.clone(), profile_value(profile))
}

pub fn canonical_content_command(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    active_operations: usize,
    queued_bytes: u64,
) -> Result<CanonicalContentArtifact<ContentCommand>> {
    let preflight = preflight_content_operation(profile, manifest, command, active_operations, queued_bytes);
    require_valid("content command", &preflight.issues)?;
    canonical_artifact(command.clone(), command_value(command))
}

pub fn canonical_content_event(
    profile: &ContentAdapterProfile,
    event: &ContentEvent,
) -> Result<CanonicalContentArtifact<ContentEvent>> {
    require_valid("content event", &validate_content_event(profile, event))?;
    canonical_artifact(event.clone(), event_value(event))
}

pub fn canonical_partial_state(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
) -> Result<CanonicalContentArtifact<ContentPartialState>> {
    require_valid("content partial state", &validate_partial_state(profile, manifest, state))?;
    canonical_artifact(state.clone(), partial_value(state))
}

pub fn canonical_content_status(
    profile: &ContentAdapterProfile,
    status: &ContentAdapterStatus,
) -> Result<CanonicalContentArtifact<ContentAdapterStatus>> {
    require_valid("content adapter status", &validate_adapter_status(profile, status))?;
    canonical_artifact(status.clone(), status_value(status))
}

pub fn content_store_port_descriptors(profile_ref: &str) -> Vec<FabricPortDescriptor> {
    vec![
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: "content-exchange".to_string(),
            version: "v1".to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![
                "cancel".to_string(),
                "export".to_string(),
                "import".to_string(),
                "stream-get".to_string(),
            ],
            input_schema_refs: vec![CONTENT_COMMAND_SCHEMA.to_string()],
            output_schema_refs: vec![
                CONTENT_EVENT_SCHEMA.to_string(),
                CONTENT_PARTIAL_STATE_SCHEMA.to_string(),
            ],
            authority_requirements: vec![
                FabricAuthority::Transport,
                FabricAuthority::Policy,
                FabricAuthority::Resources,
                FabricAuthority::Evidence,
            ],
            resource_requirements: vec![
                FabricResource::Memory,
                FabricResource::NetworkBytes,
                FabricResource::Concurrency,
                FabricResource::QueueDepth,
                FabricResource::LogicalTime,
                FabricResource::Diagnostics,
            ],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: "bounded-verified-content-exchange".to_string(),
            conformance_refs: vec![profile_ref.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        },
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: "content-store".to_string(),
            version: "v1".to_string(),
            class: FabricPortClass::DurableState,
            operation_classes: vec![
                "availability".to_string(),
                "protect".to_string(),
                "range-read".to_string(),
                "stream-get".to_string(),
                "stream-put".to_string(),
                "unprotect".to_string(),
            ],
            input_schema_refs: vec![CONTENT_COMMAND_SCHEMA.to_string()],
            output_schema_refs: vec![
                CONTENT_EVENT_SCHEMA.to_string(),
                CONTENT_PARTIAL_STATE_SCHEMA.to_string(),
                CONTENT_STATUS_SCHEMA.to_string(),
            ],
            authority_requirements: vec![
                FabricAuthority::DurableState,
                FabricAuthority::Policy,
                FabricAuthority::Resources,
                FabricAuthority::Evidence,
            ],
            resource_requirements: vec![
                FabricResource::Memory,
                FabricResource::StorageBytes,
                FabricResource::Concurrency,
                FabricResource::QueueDepth,
                FabricResource::LogicalTime,
                FabricResource::Diagnostics,
            ],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: "capability-rooted-verified-content-store".to_string(),
            conformance_refs: vec![profile_ref.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        },
    ]
}

fn profile_value(profile: &ContentAdapterProfile) -> IOValue {
    record(PROFILE_RECORD, vec![
        string(CONTENT_ADAPTER_PROFILE_SCHEMA),
        field("profile-id", string(&profile.profile_id)),
        field("declared-profile-ref", string(&profile.profile_ref)),
        field("class", string(profile.class.as_str())),
        field("capabilities", strings(profile.capabilities.iter().map(|capability| capability.as_str()))),
        field("bounds", bounds_value(&profile.bounds)),
        field("supported-transforms", strings(profile.supported_transforms.iter().map(String::as_str))),
        field("evidence-refs", strings(profile.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&profile.non_claims)),
        checks(&[
            "canonical-identity-primitive-owned",
            "bounded-operations",
            "backend-hints-are-not-authority",
        ]),
    ])
}

fn command_value(command: &ContentCommand) -> IOValue {
    record(COMMAND_RECORD, vec![
        string(CONTENT_COMMAND_SCHEMA),
        field("operation-ref", string(&command.operation_ref)),
        field("adapter-ref", string(&command.adapter_ref)),
        field("operation", string(command.operation.as_str())),
        field("manifest-ref", string(&command.manifest_ref)),
        field("range", range_value(command.range)),
        field("expected-bytes", u64_value(command.expected_bytes)),
        field("expected-chunks", usize_value(command.expected_chunks)),
        field("submitted-tick", u64_value(command.submitted_tick)),
        field("deadline-tick", u64_value(command.deadline_tick)),
        field("retry-count", u64_value(u64::from(command.retry_count))),
        field("cancelled", bool_value(command.cancelled)),
        field("policy-refs", strings(command.policy_refs.iter().map(String::as_str))),
        checks(&["preflight-before-io", "canonical-ids-only"]),
    ])
}

fn event_value(event: &ContentEvent) -> IOValue {
    record(EVENT_RECORD, vec![
        string(CONTENT_EVENT_SCHEMA),
        field("operation-ref", string(&event.operation_ref)),
        field("manifest-ref", string(&event.manifest_ref)),
        field("sequence", u64_value(event.sequence)),
        field("terminal", string(event.terminal.as_str())),
        field("chunk-ref", optional_string(event.chunk_ref.as_deref())),
        field("observed-bytes", u64_value(event.observed_bytes)),
        field("failure", optional_failure(event.failure)),
        field("evidence-refs", strings(event.evidence_refs.iter().map(String::as_str))),
        field("non-claims", non_claims_value(&event.non_claims)),
        checks(&["verification-before-availability", "terminal-outcome-explicit"]),
    ])
}

fn partial_value(state: &ContentPartialState) -> IOValue {
    record(PARTIAL_RECORD, vec![
        string(CONTENT_PARTIAL_STATE_SCHEMA),
        field("operation-ref", string(&state.operation_ref)),
        field("manifest-ref", string(&state.manifest_ref)),
        field("profile-ref", string(&state.profile_ref)),
        field("generation", u64_value(state.generation)),
        field("terminal", string(state.terminal.as_str())),
        field("verified-chunk-refs", strings(state.verified_chunk_refs.iter().map(String::as_str))),
        field("missing-chunk-refs", strings(state.missing_chunk_refs.iter().map(String::as_str))),
        field("verified-bytes", u64_value(state.verified_bytes)),
        field("event-count", usize_value(state.event_count)),
        field("last-sequence", optional_u64(state.last_sequence)),
        field("failure", optional_failure(state.failure)),
        checks(&["partial-state-bounded", "resume-revalidates-identity"]),
    ])
}

fn status_value(status: &ContentAdapterStatus) -> IOValue {
    record(STATUS_RECORD, vec![
        string(CONTENT_STATUS_SCHEMA),
        field("profile-ref", string(&status.profile_ref)),
        field("class", string(status.class.as_str())),
        field("generation", u64_value(status.generation)),
        field("active-operations", usize_value(status.active_operations)),
        field("queued-bytes", u64_value(status.queued_bytes)),
        field(
            "terminal-counts",
            sequence(
                status
                    .terminal_counts
                    .iter()
                    .map(|(terminal, count)| {
                        record("terminal-count", vec![string(terminal.as_str()), u64_value(*count)])
                    })
                    .collect(),
            ),
        ),
        field("backend-hint-ref", optional_string(status.backend_hint_ref.as_deref())),
        field("issues", issues_value(&status.issues)),
        field("non-claims", non_claims_value(&status.non_claims)),
        checks(&["backend-hints-redacted", "status-does-not-grant-authority"]),
    ])
}

fn bounds_value(bounds: &ContentResourceBounds) -> IOValue {
    record("content-resource-bounds", vec![
        field("max-total-bytes", u64_value(bounds.max_total_bytes)),
        field("max-chunk-count", usize_value(bounds.max_chunk_count)),
        field("max-chunk-bytes", u64_value(bounds.max_chunk_bytes)),
        field("max-range-bytes", u64_value(bounds.max_range_bytes)),
        field("max-concurrent-operations", usize_value(bounds.max_concurrent_operations)),
        field("max-queued-bytes", u64_value(bounds.max_queued_bytes)),
        field("max-memory-bytes", u64_value(bounds.max_memory_bytes)),
        field("max-deadline-ticks", u64_value(bounds.max_deadline_ticks)),
        field("max-retries", u64_value(u64::from(bounds.max_retries))),
        field("max-events", usize_value(bounds.max_events)),
        field("max-status-entries", usize_value(bounds.max_status_entries)),
    ])
}

fn issue_code(issue: &ContentIssue) -> &'static str {
    match issue {
        ContentIssue::SchemaMismatch(_) => "schema-mismatch",
        ContentIssue::EmptyField(_) => "empty-field",
        ContentIssue::MalformedToken(_) => "malformed-token",
        ContentIssue::MalformedRef(_) => "malformed-ref",
        ContentIssue::ZeroBound(_) => "zero-bound",
        ContentIssue::DuplicateValue(_) => "duplicate-value",
        ContentIssue::MissingNonClaim(_) => "missing-non-claim",
        ContentIssue::ProfileMismatch => "profile-mismatch",
        ContentIssue::AdapterMismatch => "adapter-mismatch",
        ContentIssue::ManifestMismatch => "manifest-mismatch",
        ContentIssue::UnsupportedCapability => "unsupported-capability",
        ContentIssue::UnsupportedTransform(_) => "unsupported-transform",
        ContentIssue::TotalBytesExceeded => "total-bytes-exceeded",
        ContentIssue::ChunkCountExceeded => "chunk-count-exceeded",
        ContentIssue::ChunkBytesExceeded => "chunk-bytes-exceeded",
        ContentIssue::RangeExceeded => "range-exceeded",
        ContentIssue::ConcurrencyExceeded => "concurrency-exceeded",
        ContentIssue::QueueExceeded => "queue-exceeded",
        ContentIssue::MemoryExceeded => "memory-exceeded",
        ContentIssue::DeadlineExceeded => "deadline-exceeded",
        ContentIssue::RetryExceeded => "retry-exceeded",
        ContentIssue::EventLimitExceeded => "event-limit-exceeded",
        ContentIssue::ArithmeticOverflow => "arithmetic-overflow",
        ContentIssue::Cancelled => "cancelled",
        ContentIssue::PartialStateMismatch => "partial-state-mismatch",
        ContentIssue::UnexpectedChunk(_) => "unexpected-chunk",
        ContentIssue::ReorderedChunk(_) => "reordered-chunk",
        ContentIssue::CorruptChunk(_) => "corrupt-chunk",
        ContentIssue::TruncatedChunk(_) => "truncated-chunk",
        ContentIssue::DuplicateChunk(_) => "duplicate-chunk",
        ContentIssue::BackendHintCannotReplaceIdentity => "backend-hint-cannot-replace-identity",
        ContentIssue::ProtectionCannotGrantAuthority => "protection-cannot-grant-authority",
    }
}

fn canonical_artifact<T>(artifact: T, value: IOValue) -> Result<CanonicalContentArtifact<T>> {
    let artifact_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalContentArtifact {
        artifact,
        artifact_ref,
        value,
    })
}

fn require_valid(label: &str, issues: &[ContentIssue]) -> Result<()> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} denied: {issues:?}")))
    }
}

fn range_value(range: Option<ContentRange>) -> IOValue {
    match range {
        Some(range) => record("some", vec![record("content-range", vec![
            field("offset", u64_value(range.offset)),
            field("length", u64_value(range.length)),
        ])]),
        None => record("none", Vec::new()),
    }
}

fn optional_failure(failure: Option<ContentFailure>) -> IOValue {
    match failure {
        Some(failure) => record("some", vec![string(failure.as_str())]),
        None => record("none", Vec::new()),
    }
}

fn optional_string(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn optional_u64(value: Option<u64>) -> IOValue {
    match value {
        Some(value) => record("some", vec![u64_value(value)]),
        None => record("none", Vec::new()),
    }
}

fn non_claims_value(values: &[ContentNonClaim]) -> IOValue {
    strings(values.iter().map(|value| value.as_str()))
}

fn issues_value(values: &[ContentIssue]) -> IOValue {
    strings(values.iter().map(issue_code))
}

fn checks(names: &[&str]) -> IOValue {
    field(
        "checks",
        sequence(names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect()),
    )
}

fn strings<'a>(values: impl Iterator<Item = &'a str>) -> IOValue {
    sequence(values.map(string).collect())
}

fn usize_value(value: usize) -> IOValue {
    match u64::try_from(value) {
        Ok(value) => u64_value(value),
        Err(_) => record("usize-overflow", Vec::new()),
    }
}

fn bool_value(value: bool) -> IOValue {
    crate::preserves_rail::bool_value(value)
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value.as_ref())
}

fn u64_value(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}
