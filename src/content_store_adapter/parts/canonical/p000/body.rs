use molten_core::content_store_adapter::*;
use molten_core::fabric::*;

const PROFILE_RECORD: &str = "content-store-adapter-profile-v1";
const COMMAND_RECORD: &str = "content-store-adapter-command-v1";
const EVENT_RECORD: &str = "content-store-adapter-event-v1";
const PARTIAL_RECORD: &str = "content-store-adapter-partial-state-v1";
const STATUS_RECORD: &str = "content-store-adapter-status-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalContentArtifact<T> {
    pub artifact: T,
    pub artifact_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.content_store_adapter.port_contract]
pub fn canonical_content_profile(
    profile: &ContentAdapterProfile,
) -> crate::error::Result<CanonicalContentArtifact<ContentAdapterProfile>> {
    require_valid("content adapter profile", &validate_content_profile(profile))?;
    canonical_artifact(profile.clone(), profile_value(profile))
}

pub fn canonical_content_command(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    command: &ContentCommand,
    active_operations: u64,
    queued_bytes: u64,
) -> crate::error::Result<CanonicalContentArtifact<ContentCommand>> {
    let active_operations = crate::bounded::usize_from_u64(active_operations, "content active operations")?;
    let preflight = preflight_content_operation(profile, manifest, command, active_operations, queued_bytes);
    require_valid("content command", &preflight.issues)?;
    canonical_artifact(command.clone(), command_value(command))
}

pub fn canonical_content_event(
    profile: &ContentAdapterProfile,
    event: &ContentEvent,
) -> crate::error::Result<CanonicalContentArtifact<ContentEvent>> {
    require_valid("content event", &validate_content_event(profile, event))?;
    canonical_artifact(event.clone(), event_value(event))
}

pub fn canonical_partial_state(
    profile: &ContentAdapterProfile,
    manifest: &ContentManifestDescriptor,
    state: &ContentPartialState,
) -> crate::error::Result<CanonicalContentArtifact<ContentPartialState>> {
    require_valid("content partial state", &validate_partial_state(profile, manifest, state))?;
    canonical_artifact(state.clone(), partial_value(state))
}

pub fn canonical_content_status(
    profile: &ContentAdapterProfile,
    status: &ContentAdapterStatus,
) -> crate::error::Result<CanonicalContentArtifact<ContentAdapterStatus>> {
    require_valid("content adapter status", &validate_adapter_status(profile, status))?;
    canonical_artifact(status.clone(), status_value(status))
}

pub fn content_store_port_descriptors(profile_ref: &str) -> Vec<FabricPortDescriptor> {
    vec![
        content_exchange_port_descriptor(profile_ref),
        content_store_port_descriptor(profile_ref),
    ]
}

fn content_exchange_port_descriptor(profile_ref: &str) -> FabricPortDescriptor {
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
    }
}

fn content_store_port_descriptor(profile_ref: &str) -> FabricPortDescriptor {
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
    }
}

fn profile_value(profile: &ContentAdapterProfile) -> preserves::IOValue {
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

fn command_value(command: &ContentCommand) -> preserves::IOValue {
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

fn event_value(event: &ContentEvent) -> preserves::IOValue {
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

fn partial_value(state: &ContentPartialState) -> preserves::IOValue {
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

fn status_value(status: &ContentAdapterStatus) -> preserves::IOValue {
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

fn bounds_value(bounds: &ContentResourceBounds) -> preserves::IOValue {
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
