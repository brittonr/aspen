// r[impl molten.fabric_execution.validation]
use super::*;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;

mod recoverytests;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const HASH_F: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const GENERATION: u64 = 1;
const STALE_GENERATION: u64 = 2;
const TIMEOUT_MS: u64 = 1_000;
const POLL_INTERVAL_MS: u64 = 10;
const TEARDOWN_TIMEOUT_MS: u64 = 500;
const STREAM_BYTES: u64 = 4_096;
const MEMORY_BYTES: u64 = STREAM_BYTES * 2;
const STORAGE_BYTES: u64 = 16_384;
const ARGUMENT_COUNT: usize = 8;
const ARGUMENT_BYTES: usize = 128;
const ENVIRONMENT_COUNT: usize = 8;
const ENVIRONMENT_NAME_BYTES: usize = 64;
const ENVIRONMENT_VALUE_BYTES: usize = 256;
const CONCURRENCY_UNITS: u64 = 1;
const QUEUE_UNITS: u64 = 1;
const DEADLINE_TICKS: u64 = 2_000;
const SUCCESS_EXIT_CODE: i32 = 0;

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
        supported_termination_scopes: vec![
            ExecutionTerminationScope::DirectChild,
            ExecutionTerminationScope::ProcessGroup,
        ],
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

fn request() -> ExecutionRequest {
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
        arguments: vec!["fixture-argument".to_string()],
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

fn authority() -> ExecutionAuthorityFacts {
    let request = request();
    ExecutionAuthorityFacts {
        authority_ref: request.authority_ref,
        executable_authority_ref: HASH_A.to_string(),
        provenance_ref: HASH_B.to_string(),
        effect_admission_ref: HASH_C.to_string(),
        workspace_authority_ref: HASH_D.to_string(),
        process_authority_ref: HASH_E.to_string(),
        resource_grant_ref: request.resource_grant_ref,
        policy_ref: HASH_F.to_string(),
        executable_artifact_ref: request.executable_artifact_ref,
        executable_identity_ref: request.executable_identity_ref,
        workspace_ref: request.workspace_ref,
        operation_ref: request.operation_ref,
        extension_id: request.extension_id,
        service_id: request.service_id,
        generation: request.generation,
        profile_ref: request.profile_ref,
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

// r[verify molten.fabric_execution.component_pin]
// r[verify molten.fabric_execution.nonclaims]
// r[verify molten.fabric_execution.simulation]
#[test]
fn reviewed_component_profile_is_admitted_and_mutable_source_is_denied() {
    let admitted = admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess))
        .expect("reviewed execution profile");
    let simulation = admit_execution_profile(&descriptor(ExecutionProfileKind::DeterministicSimulation))
        .expect("reviewed simulation profile");
    assert_eq!(admitted.descriptor.component_revision, BOUNDED_EXEC_REVISION);
    assert_eq!(simulation.descriptor.kind, ExecutionProfileKind::DeterministicSimulation);
    assert_eq!(admitted.descriptor.non_claims, REQUIRED_EXECUTION_NON_CLAIMS);

    let mut mutable = descriptor(ExecutionProfileKind::LiveBoundedProcess);
    mutable.component_repository = "../bounded-exec".to_string();
    let issues = admit_execution_profile(&mutable).expect_err("mutable source must deny");
    assert!(issues.contains(&ExecutionAdmissionIssue::ComponentSourceMismatch));
}

// r[verify molten.fabric_execution.request]
// r[verify molten.fabric_execution.authority]
// r[verify molten.fabric_execution.output]
// r[verify molten.fabric_execution.port_contract]
// r[verify molten.fabric_execution.validation]
#[test]
fn complete_equal_requests_produce_equal_capability_rooted_plans() {
    let profile = admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess))
        .expect("reviewed execution profile");
    let first = admit_execution_request(&profile, &request(), &authority(), resources(), GENERATION)
        .expect("first admitted request");
    let second = admit_execution_request(&profile, &request(), &authority(), resources(), GENERATION)
        .expect("second admitted request");
    assert_eq!(first, second);
    assert_eq!(first.resolution.workspace_ref, HASH_D);
    assert_eq!(first.resolution.stdout_role, "stdout-retained-prefix");
}

// r[verify molten.fabric_execution.authority]
#[test]
fn executable_possession_without_complete_authority_denies() {
    let profile = admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess))
        .expect("reviewed execution profile");
    let mut incomplete = authority();
    incomplete.executable_authority_ref.clear();
    incomplete.provenance_ref.clear();
    incomplete.policy_ref.clear();
    let issues = admit_execution_request(&profile, &request(), &incomplete, resources(), GENERATION)
        .expect_err("possession alone must deny");
    assert!(
        issues.iter().any(|issue| matches!(
            issue,
            ExecutionAdmissionIssue::MissingAuthorityEvidence("executable-authority-ref")
        ))
    );
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ExecutionAdmissionIssue::MissingAuthorityEvidence("provenance-ref")))
    );
    assert!(
        issues
            .iter()
            .any(|issue| matches!(issue, ExecutionAdmissionIssue::MissingAuthorityEvidence("policy-ref")))
    );
}

// r[verify molten.fabric_execution.environment]
#[test]
fn inherited_environment_shell_path_search_implicit_workspace_and_secrets_deny() {
    let profile = admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess))
        .expect("reviewed execution profile");
    let mut invalid = request();
    invalid.environment_mode = ExecutionEnvironmentMode::InheritRequested;
    invalid.invocation_mode = ExecutionInvocationMode::ShellExpansion;
    invalid.executable_resolution = ExecutableResolutionMode::PathSearch;
    invalid.workspace_mode = WorkspaceMode::ImplicitCurrentDirectory;
    invalid.environment[0].value_class = EnvironmentValueClass::Secret;
    let issues = admit_execution_request(&profile, &invalid, &authority(), resources(), GENERATION)
        .expect_err("ambient process state must deny");
    assert!(issues.contains(&ExecutionAdmissionIssue::InheritedEnvironmentDenied));
    assert!(issues.contains(&ExecutionAdmissionIssue::ShellExpansionDenied));
    assert!(issues.contains(&ExecutionAdmissionIssue::PathSearchDenied));
    assert!(issues.contains(&ExecutionAdmissionIssue::ImplicitCurrentDirectoryDenied));
    assert!(issues.contains(&ExecutionAdmissionIssue::SecretEnvironmentDenied("FIXTURE".to_string())));
}

// r[verify molten.fabric_execution.request]
#[test]
fn zero_and_overbound_limits_deny_before_spawn() {
    let profile = admit_execution_profile(&descriptor(ExecutionProfileKind::LiveBoundedProcess))
        .expect("reviewed execution profile");
    let mut invalid = request();
    invalid.limits.timeout_ms = 0;
    invalid.limits.stdout_max_bytes = STREAM_BYTES + 1;
    let issues = admit_execution_request(&profile, &invalid, &authority(), resources(), GENERATION)
        .expect_err("invalid limits must deny");
    assert!(issues.contains(&ExecutionAdmissionIssue::ZeroBound("timeout-ms")));
    assert!(issues.contains(&ExecutionAdmissionIssue::BoundExceeded {
        field: "stdout-max-bytes",
        actual: STREAM_BYTES + 1,
        maximum: STREAM_BYTES,
    }));
}

// r[verify molten.fabric_execution.lifecycle]
#[test]
fn lifecycle_accepts_observed_terminals_and_rejects_invalid_transitions() {
    let queued = plan_execution_lifecycle_transition(ExecutionLifecycleState::Admitted, ExecutionLifecycleEvent::Queue)
        .expect("queue transition");
    let started =
        plan_execution_lifecycle_transition(queued.next, ExecutionLifecycleEvent::Start).expect("start transition");
    let exited =
        plan_execution_lifecycle_transition(started.next, ExecutionLifecycleEvent::Exit).expect("exit transition");
    assert_eq!(exited.next, ExecutionLifecycleState::Exited);
    assert_eq!(
        plan_execution_lifecycle_transition(ExecutionLifecycleState::Exited, ExecutionLifecycleEvent::Cancel),
        Err(ExecutionLifecycleIssue::InvalidTransition {
            state: ExecutionLifecycleState::Exited,
            event: ExecutionLifecycleEvent::Cancel,
        })
    );
}

// r[verify molten.fabric_execution.generation]
#[test]
fn stale_or_substituted_completion_denies() {
    let expected = request().identity();
    let mut observed = expected.clone();
    observed.generation = STALE_GENERATION;
    observed.executable_identity_ref = HASH_F.to_string();
    let issues = admit_execution_completion(&expected, &observed, ExecutionLifecycleState::Exited, GENERATION)
        .expect_err("stale substituted completion must deny");
    assert!(issues.contains(&ExecutionCompletionIssue::StaleGeneration {
        actual: STALE_GENERATION,
        active: GENERATION,
    }));
    assert!(issues.contains(&ExecutionCompletionIssue::IdentityMismatch("executable-identity-ref")));
}
