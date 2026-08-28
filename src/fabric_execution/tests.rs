use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::fabric::DeterminismClass;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortRequirement;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::ReplayClass;
use crate::fabric::build_fabric_port_registry;
use crate::fabric::resolve_fabric_port_binding;

mod live;
mod simulation;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const HASH_F: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const GENERATION: u64 = 1;
const TIMEOUT_MS: u64 = 1_000;
const SHORT_TIMEOUT_MS: u64 = 50;
const POLL_INTERVAL_MS: u64 = 5;
const TEARDOWN_TIMEOUT_MS: u64 = 500;
const STREAM_BYTES: u64 = 4_096;
const SMALL_STREAM_BYTES: u64 = 4;
const MEMORY_BYTES: u64 = STREAM_BYTES * 2;
const STORAGE_BYTES: u64 = 16_384;
const ARGUMENT_COUNT: usize = 8;
const ARGUMENT_BYTES: usize = 256;
const ENVIRONMENT_COUNT: usize = 8;
const ENVIRONMENT_NAME_BYTES: usize = 64;
const ENVIRONMENT_VALUE_BYTES: usize = 256;
const CONCURRENCY_UNITS: u64 = 1;
const QUEUE_UNITS: u64 = 1;
const DEADLINE_TICKS: u64 = 2_000;
const SUCCESS_EXIT_CODE: i32 = 0;
const REJECTED_EXIT_CODE: i32 = 7;
const EXPECTED_STDOUT: &[u8] = b"bounded:input";
const INPUT_BYTES: &[u8] = b"input\n";
const FLOOD_OUTPUT: &str = "overflow";
const NON_TERMINATING_SCRIPT: &str = "while :; do :; done";

#[derive(Debug, Clone, Default)]
struct MemoryPublisher {
    fail: bool,
    published: Vec<(String, String, Vec<u8>)>,
}

impl ExecutionOutputPublisher for MemoryPublisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &RetainedExecutionStream,
    ) -> Result<PublishedExecutionStream, ExecutionOutputPublicationError> {
        if self.fail {
            return Err(ExecutionOutputPublicationError::Unavailable);
        }
        let content_ref = crate::preserves_rail::content_ref_from_bytes(&stream.retained_bytes);
        let receipt_material = format!("{operation_ref}\0{}\0{content_ref}", stream.role);
        let publication_receipt_ref = crate::preserves_rail::content_ref_from_bytes(receipt_material.as_bytes());
        self.published.push((operation_ref.to_string(), stream.role.clone(), stream.retained_bytes.clone()));
        Ok(PublishedExecutionStream {
            content_ref,
            publication_receipt_ref,
        })
    }
}

fn descriptor(kind: ExecutionProfileKind) -> ExecutionProfileDescriptor {
    ExecutionProfileDescriptor {
        schema: EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: match kind {
            ExecutionProfileKind::LiveBoundedProcess => "bounded-process-live-v1",
            ExecutionProfileKind::DeterministicSimulation => "bounded-process-simulation-v1",
        }
        .to_string(),
        profile_ref: HASH_A.to_string(),
        kind,
        platform: ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: TIMEOUT_MS,
        max_stdin_bytes: STREAM_BYTES,
        max_stdout_bytes: STREAM_BYTES,
        max_stderr_bytes: STREAM_BYTES,
        max_poll_interval_ms: POLL_INTERVAL_MS,
        max_teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
        max_arguments: ARGUMENT_COUNT,
        max_argument_bytes: ARGUMENT_BYTES,
        max_environment_entries: ENVIRONMENT_COUNT,
        max_environment_name_bytes: ENVIRONMENT_NAME_BYTES,
        max_environment_value_bytes: ENVIRONMENT_VALUE_BYTES,
        max_concurrency_units: CONCURRENCY_UNITS,
        max_queue_units: QUEUE_UNITS,
        component_repository: BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: BOUNDED_EXEC_REVISION.to_string(),
        component_license: BOUNDED_EXEC_LICENSE.to_string(),
        component_package: BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![HASH_B.to_string()],
        fabric_non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn request(arguments: Vec<String>) -> ExecutionRequest {
    ExecutionRequest {
        schema: EXECUTION_REQUEST_SCHEMA.to_string(),
        operation_ref: HASH_B.to_string(),
        idempotency_ref: HASH_C.to_string(),
        extension_id: "fixture-extension".to_string(),
        service_id: "fixture-service".to_string(),
        callback_ref: HASH_D.to_string(),
        effect_ref: HASH_E.to_string(),
        generation: GENERATION,
        profile_ref: HASH_A.to_string(),
        executable_artifact_ref: HASH_B.to_string(),
        executable_identity_ref: HASH_C.to_string(),
        arguments,
        environment: vec![EnvironmentEntry {
            name: "FIXTURE".to_string(),
            value: "bounded".to_string(),
            value_class: EnvironmentValueClass::Public,
        }],
        environment_mode: ExecutionEnvironmentMode::Clear,
        invocation_mode: ExecutionInvocationMode::Direct,
        executable_resolution: ExecutableResolutionMode::ExactArtifact,
        workspace_ref: HASH_D.to_string(),
        workspace_mode: WorkspaceMode::CapabilityRoot,
        stdin_ref: Some(HASH_E.to_string()),
        limits: ExecutionRequestLimits {
            timeout_ms: TIMEOUT_MS,
            stdin_max_bytes: STREAM_BYTES,
            stdout_max_bytes: STREAM_BYTES,
            stderr_max_bytes: STREAM_BYTES,
            poll_interval_ms: POLL_INTERVAL_MS,
            teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
            concurrency_units: CONCURRENCY_UNITS,
            queue_units: QUEUE_UNITS,
        },
        termination_scope: ExecutionTerminationScope::ProcessGroup,
        accepted_exit_codes: vec![SUCCESS_EXIT_CODE],
        reject_stdout_truncation: true,
        reject_stderr_truncation: true,
        authority_ref: HASH_E.to_string(),
        resource_grant_ref: HASH_F.to_string(),
    }
}

fn authority(request: &ExecutionRequest) -> ExecutionAuthorityFacts {
    ExecutionAuthorityFacts {
        authority_ref: request.authority_ref.clone(),
        executable_authority_ref: HASH_A.to_string(),
        provenance_ref: HASH_B.to_string(),
        effect_admission_ref: HASH_C.to_string(),
        workspace_authority_ref: HASH_D.to_string(),
        process_authority_ref: HASH_E.to_string(),
        resource_grant_ref: request.resource_grant_ref.clone(),
        policy_ref: HASH_F.to_string(),
        executable_artifact_ref: request.executable_artifact_ref.clone(),
        executable_identity_ref: request.executable_identity_ref.clone(),
        workspace_ref: request.workspace_ref.clone(),
        operation_ref: request.operation_ref.clone(),
        extension_id: request.extension_id.clone(),
        service_id: request.service_id.clone(),
        generation: request.generation,
        profile_ref: request.profile_ref.clone(),
    }
}

const fn resources() -> ExecutionResourceGrant {
    ExecutionResourceGrant {
        memory_bytes: MEMORY_BYTES,
        storage_bytes: STORAGE_BYTES,
        diagnostic_bytes: MEMORY_BYTES,
        logical_deadline_ticks: DEADLINE_TICKS,
        concurrency_units: CONCURRENCY_UNITS,
        queue_units: QUEUE_UNITS,
    }
}

fn canonical_request(
    kind: ExecutionProfileKind,
    arguments: Vec<String>,
) -> (CanonicalExecutionProfile, CanonicalExecutionRequest) {
    canonicalize_request(kind, request(arguments))
}

fn canonicalize_request(
    kind: ExecutionProfileKind,
    request: ExecutionRequest,
) -> (CanonicalExecutionProfile, CanonicalExecutionRequest) {
    let profile = canonical_admit_execution_profile(&descriptor(kind)).expect("canonical profile");
    let authority = authority(&request);
    let request = canonical_admit_execution_request(&profile, &request, &authority, resources(), GENERATION)
        .expect("canonical request");
    (profile, request)
}

fn resolved(stdin_bytes: Option<Vec<u8>>) -> ResolvedExecutionContext {
    ResolvedExecutionContext {
        executable_path: PathBuf::from("/bin/sh"),
        executable_artifact_ref: HASH_B.to_string(),
        executable_identity_ref: HASH_C.to_string(),
        workspace_path: PathBuf::from("/"),
        workspace_ref: HASH_D.to_string(),
        stdin_ref: stdin_bytes.as_ref().map(|_| HASH_E.to_string()),
        stdin_bytes,
    }
}

fn script_arguments(script: &str) -> Vec<String> {
    vec!["-c".to_string(), script.to_string()]
}

fn scripted_process(lifecycle: ExecutionLifecycleState, bytes: &[u8]) -> ExecutionProcessObservation {
    let byte_count = u64::try_from(bytes.len()).expect("fixture output byte count");
    ExecutionProcessObservation {
        lifecycle,
        start_observed: lifecycle != ExecutionLifecycleState::FailedBeforeStart,
        terminal_observed: !matches!(
            lifecycle,
            ExecutionLifecycleState::Unknown | ExecutionLifecycleState::TeardownIncomplete
        ),
        teardown_observed: !matches!(
            lifecycle,
            ExecutionLifecycleState::Unknown | ExecutionLifecycleState::TeardownIncomplete
        ),
        exit_code: Some(SUCCESS_EXIT_CODE),
        signal: None,
        disposition: ExecutionObservedDisposition::ExitPolicyAccepted,
        stdout: RetainedExecutionStream {
            role: "stdout-retained-prefix".to_string(),
            retained_bytes: bytes.to_vec(),
            observed_bytes: byte_count,
            retained_byte_count: byte_count,
            truncated: false,
        },
        stderr: RetainedExecutionStream {
            role: "stderr-retained-prefix".to_string(),
            retained_bytes: Vec::new(),
            observed_bytes: 0,
            retained_byte_count: 0,
            truncated: false,
        },
    }
}

// r[verify molten.fabric_execution.component_pin]
// r[verify molten.fabric_execution.port_contract]
#[test]
fn source_cohort_and_exact_port_binding_are_canonical() {
    let source =
        canonical_bounded_exec_source_cohort(ExecutionPlatform::UnixProcessGroup).expect("bounded exec source cohort");
    assert_eq!(source.revision, BOUNDED_EXEC_REVISION);
    assert!(source.source_ref.starts_with("blake3:"));

    let profile =
        canonical_admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess)).expect("live profile");
    let descriptor = fabric_execution_port_descriptor(&profile);
    let registry = build_fabric_port_registry(std::slice::from_ref(&descriptor)).expect("execution registry");
    let requirement = FabricPortRequirement {
        port_id: EXECUTION_PORT_ID.to_string(),
        version: EXECUTION_PORT_VERSION.to_string(),
        class: FabricPortClass::Execution,
        operation_classes: descriptor.operation_classes.clone(),
        input_schema_refs: descriptor.input_schema_refs.clone(),
        output_schema_refs: descriptor.output_schema_refs.clone(),
        allowed_authorities: vec![
            FabricAuthority::Execution,
            FabricAuthority::Resources,
            FabricAuthority::Evidence,
        ],
        available_resources: vec![
            FabricResource::Memory,
            FabricResource::StorageBytes,
            FabricResource::ExecutionMillis,
            FabricResource::InputBytes,
            FabricResource::OutputBytes,
            FabricResource::Concurrency,
            FabricResource::QueueDepth,
            FabricResource::LogicalTime,
            FabricResource::Diagnostics,
        ],
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: "bounded-process-live-v1".to_string(),
    };
    let binding = resolve_fabric_port_binding(&registry, &requirement).expect("exact execution binding");
    assert_eq!(binding.class, FabricPortClass::Execution);
    assert_eq!(binding.implementation_profile, "bounded-process-live-v1");
}
