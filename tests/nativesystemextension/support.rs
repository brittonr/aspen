#![allow(
    tigerstyle::excessive_file_length,
    reason = "the integration cohort keeps all exact profile, manifest, authority, and executable fixtures together"
)]

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use molten::fabric::*;
use molten::fabric_execution::*;
use molten::system_extension::*;

pub const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
pub const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
pub const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
pub const HASH_F: &str = "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
pub const EFFECT_PORT_ID: &str = "molten.fixture.native.effect";
pub const EFFECT_PORT_VERSION: &str = "v1";
pub const EFFECT_OPERATION: &str = "fixture-effect";
pub const EFFECT_INPUT_SCHEMA: &str = "molten.fixture.native.effect-input.v1";
pub const EFFECT_OUTPUT_SCHEMA: &str = "molten.fixture.native.effect-output.v1";
pub const GENERATION: u64 = 1;
pub const REQUEST_BYTES: u64 = 128;
const CALLBACK_LIMIT: u64 = 1_048_576;
const DIAGNOSTIC_LIMIT: u64 = 1_048_576;
const TIMEOUT_MS: u64 = 5_000;
const POLL_INTERVAL_MS: u64 = 5;
const TEARDOWN_TIMEOUT_MS: u64 = 1_000;
const MAX_ARGUMENTS: usize = 16;
const MAX_ARGUMENT_BYTES: usize = 4_096;
const MAX_ENVIRONMENT: usize = 16;
const MAX_ENVIRONMENT_NAME: usize = 128;
const MAX_ENVIRONMENT_VALUE: usize = 4_096;
const MAX_INSTANCES: usize = 4;
const MAX_OPERATIONS: usize = 64;
const MAX_BINDINGS: usize = 16;
const MAX_POLICIES: usize = 16;
const RESOURCE_UNITS: u64 = 1;
const QUEUE_UNITS: u64 = 64;
const LOGICAL_DEADLINE: u64 = 10_000;
const CALLBACK_DEADLINE_TICKS: u64 = 100;
const SHUTDOWN_GRACE_TICKS: u64 = 100;
const MAX_RESTART_ATTEMPTS: u64 = 2;
const MAX_CONCURRENT_CALLBACKS: u64 = 1;
const MAX_QUEUED_EVENTS: u64 = 8;
const MAX_INFLIGHT_BYTES: u64 = CALLBACK_LIMIT;
const MAX_OPEN_STREAMS: u64 = 1;
const MAX_TIMERS: u64 = 1;
const MAX_EFFECT_REQUESTS: u64 = 8;
const SUCCESS_EXIT_CODE: i32 = 0;

#[derive(Debug, Clone, Default)]
pub struct Publisher {
    pub published: Vec<String>,
}

impl ExecutionOutputPublisher for Publisher {
    fn publish(
        &mut self,
        operation_ref: &str,
        stream: &RetainedExecutionStream,
    ) -> Result<PublishedExecutionStream, ExecutionOutputPublicationError> {
        let content_ref = molten::preserves_rail::content_ref_from_bytes(&stream.retained_bytes);
        let receipt_ref = molten::preserves_rail::content_ref_from_bytes(
            format!("{operation_ref}\0{}\0{content_ref}", stream.role).as_bytes(),
        );
        self.published.push(content_ref.clone());
        Ok(PublishedExecutionStream {
            content_ref,
            publication_receipt_ref: receipt_ref,
        })
    }
}

#[derive(Debug, Default)]
pub struct EffectPort {
    pub routed: u64,
}

impl FabricEffectPort for EffectPort {
    fn route(
        &mut self,
        binding: &CanonicalFabricPortBinding,
        effect: &TypedEffectRequest,
    ) -> FabricPortResult<PortEffectOutput> {
        if binding.binding.key.port_id != EFFECT_PORT_ID
            || binding.binding.key.version != EFFECT_PORT_VERSION
            || effect.operation != EFFECT_OPERATION
        {
            return Err(FabricPortError::malformed("fixture effect binding mismatch"));
        }
        self.routed = self
            .routed
            .checked_add(1)
            .ok_or_else(|| FabricPortError::malformed("fixture effect count overflow"))?;
        Ok(PortEffectOutput {
            output_schema_ref: EFFECT_OUTPUT_SCHEMA.to_string(),
            output_ref: HASH_E.to_string(),
        })
    }
}

pub type Port = LiveExecutionAdapter<Publisher>;
pub type Journal = InMemoryNativeHostJournal;
pub type Service = NativeSystemExtensionService<Port, Journal>;

#[derive(Clone)]
pub struct Cohort {
    pub native_profile: AdmittedNativeHostProfile,
    pub executable: AdmittedNativeExecutable,
    pub admitted: CanonicalAdmittedSystemExtensionManifest,
    pub execution_profile: CanonicalExecutionProfile,
    pub template: NativeExecutionTemplate,
    pub journal: Arc<Mutex<Journal>>,
}

impl Cohort {
    pub fn new() -> Self {
        let executable_path = PathBuf::from(env!("CARGO_BIN_EXE_molten-native-extension-fixture"));
        let executable_bytes = std::fs::read(&executable_path).expect("read native fixture executable");
        let executable_bytes_ref = molten::preserves_rail::content_ref_from_bytes(&executable_bytes);
        let execution_profile =
            canonical_admit_execution_profile(&execution_profile_descriptor()).expect("execution profile");
        let native_profile =
            admit_native_host_profile(&native_profile(&execution_profile)).expect("native host profile");
        let admitted = admitted_manifest();
        let executable = admit_native_executable(
            &native_profile,
            &executable_evidence(&admitted, &execution_profile, &executable_bytes_ref),
        )
        .expect("native executable evidence");
        let instance_id = native_identity_ref(&[
            "native-instance-v1",
            &admitted.manifest().extension_id,
            &admitted.manifest().service_id,
            admitted.manifest_ref(),
        ]);
        let template = execution_template(
            &native_profile,
            &executable,
            &execution_profile,
            executable_path,
            instance_id,
            &admitted,
        );
        Self {
            native_profile,
            executable,
            admitted,
            execution_profile,
            template,
            journal: Arc::new(Mutex::new(Journal::default())),
        }
    }

    pub fn replace_program(&mut self, path: PathBuf, arguments: Vec<String>, timeout_ms: u64, stdout_max_bytes: u64) {
        let executable_bytes_ref = std::fs::read(&path).map_or_else(
            |_| molten::preserves_rail::content_ref_from_bytes(b"missing-native-fixture"),
            |bytes| molten::preserves_rail::content_ref_from_bytes(&bytes),
        );
        self.executable.executable.executable_bytes_ref = executable_bytes_ref.clone();
        self.template.executable = self.executable.clone();
        self.template.request.executable_identity_ref = executable_bytes_ref.clone();
        self.template.request.arguments = arguments;
        self.template.request.limits.timeout_ms = timeout_ms;
        self.template.request.limits.stdout_max_bytes = stdout_max_bytes;
        self.template.authority.executable_identity_ref = executable_bytes_ref.clone();
        self.template.resolved.executable_path = path;
        self.template.resolved.executable_identity_ref = executable_bytes_ref;
    }

    pub fn install(&self) -> Service {
        let port = LiveExecutionAdapter::new(self.execution_profile.clone(), Publisher::default())
            .expect("live execution adapter");
        NativeSystemExtensionService::install(
            self.native_profile.clone(),
            self.executable.clone(),
            self.admitted.clone(),
            port,
            self.journal.clone(),
            self.template.clone(),
        )
        .expect("install native service")
    }

    pub fn recovered(&self, instance: NativeInstanceRecord) -> Service {
        let instance = Arc::new(Mutex::new(instance));
        let port = LiveExecutionAdapter::new(self.execution_profile.clone(), Publisher::default())
            .expect("live execution adapter");
        let executor = NativeProcessSystemExtensionExecutor::new(
            port,
            self.journal.clone(),
            instance.clone(),
            self.template.clone(),
        )
        .expect("recovered native executor");
        NativeSystemExtensionService::from_recovered(
            self.native_profile.clone(),
            self.executable.clone(),
            self.admitted.clone(),
            executor,
            self.journal.clone(),
            instance,
        )
        .expect("recover native service")
    }
}

pub fn ingress(generation: u64, manifest_ref: &str) -> NativeIngressEnvelope {
    NativeIngressEnvelope {
        schema: NATIVE_INGRESS_SCHEMA.to_string(),
        request_ref: HASH_A.to_string(),
        endpoint_ref: HASH_B.to_string(),
        peer_ref: HASH_C.to_string(),
        service_id: "molten.fixture.native.service".to_string(),
        manifest_ref: manifest_ref.to_string(),
        generation,
        authority_ref: HASH_D.to_string(),
        policy_ref: HASH_E.to_string(),
        resource_ref: HASH_F.to_string(),
        transport_profile_ref: HASH_C.to_string(),
        alpn: NATIVE_ALPN.to_string(),
        framing: NATIVE_FRAMING.to_string(),
        payload_ref: HASH_D.to_string(),
        accounted_bytes: REQUEST_BYTES,
    }
}

fn execution_profile_descriptor() -> ExecutionProfileDescriptor {
    ExecutionProfileDescriptor {
        schema: EXECUTION_PROFILE_SCHEMA.to_string(),
        profile_id: "native-callback-live-v1".to_string(),
        profile_ref: HASH_B.to_string(),
        kind: ExecutionProfileKind::LiveBoundedProcess,
        platform: ExecutionPlatform::UnixProcessGroup,
        supported_termination_scopes: vec![ExecutionTerminationScope::ProcessGroup],
        max_timeout_ms: TIMEOUT_MS,
        max_stdin_bytes: CALLBACK_LIMIT,
        max_stdout_bytes: CALLBACK_LIMIT,
        max_stderr_bytes: DIAGNOSTIC_LIMIT,
        max_poll_interval_ms: POLL_INTERVAL_MS,
        max_teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
        max_arguments: MAX_ARGUMENTS,
        max_argument_bytes: MAX_ARGUMENT_BYTES,
        max_environment_entries: MAX_ENVIRONMENT,
        max_environment_name_bytes: MAX_ENVIRONMENT_NAME,
        max_environment_value_bytes: MAX_ENVIRONMENT_VALUE,
        max_concurrency_units: RESOURCE_UNITS,
        max_queue_units: QUEUE_UNITS,
        component_repository: BOUNDED_EXEC_REPOSITORY.to_string(),
        component_revision: BOUNDED_EXEC_REVISION.to_string(),
        component_license: BOUNDED_EXEC_LICENSE.to_string(),
        component_package: BOUNDED_EXEC_PACKAGE.to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        fabric_non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        non_claims: REQUIRED_EXECUTION_NON_CLAIMS.to_vec(),
    }
}

fn native_profile(execution: &CanonicalExecutionProfile) -> NativeHostProfile {
    NativeHostProfile {
        schema: NATIVE_HOST_PROFILE_SCHEMA.to_string(),
        profile_id: "native-host-local-pilot-v1".to_string(),
        profile_ref: HASH_A.to_string(),
        execution_profile_ref: execution.profile_ref.clone(),
        transport_profile_ref: HASH_C.to_string(),
        alpn: NATIVE_ALPN.to_string(),
        framing: NATIVE_FRAMING.to_string(),
        max_callback_input_bytes: CALLBACK_LIMIT,
        max_callback_output_bytes: CALLBACK_LIMIT,
        max_diagnostic_bytes: DIAGNOSTIC_LIMIT,
        max_instances: MAX_INSTANCES,
        max_unresolved_operations: MAX_OPERATIONS,
        max_port_bindings: MAX_BINDINGS,
        max_policy_refs: MAX_POLICIES,
        is_local_live_pilot: true,
        non_claims: REQUIRED_NATIVE_HOST_NON_CLAIMS.to_vec(),
    }
}

fn executable_evidence(
    admitted: &CanonicalAdmittedSystemExtensionManifest,
    execution: &CanonicalExecutionProfile,
    executable_bytes_ref: &str,
) -> NativeExecutableEvidence {
    NativeExecutableEvidence {
        schema: NATIVE_EXECUTABLE_EVIDENCE_SCHEMA.to_string(),
        executable_ref: HASH_E.to_string(),
        executable_bytes_ref: executable_bytes_ref.to_string(),
        artifact_kind_ref: HASH_A.to_string(),
        target_ref: HASH_B.to_string(),
        dependency_closure_ref: HASH_C.to_string(),
        materialization_ref: HASH_D.to_string(),
        provenance_ref: HASH_E.to_string(),
        source_gate_ref: HASH_F.to_string(),
        policy_ref: HASH_A.to_string(),
        authority_ref: HASH_B.to_string(),
        resource_ref: HASH_C.to_string(),
        execution_profile_ref: execution.profile_ref.clone(),
        manifest_ref: admitted.manifest_ref().to_string(),
        state_schema_ref: HASH_D.to_string(),
        port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
    }
}

fn execution_template(
    native_profile: &AdmittedNativeHostProfile,
    executable: &AdmittedNativeExecutable,
    execution: &CanonicalExecutionProfile,
    executable_path: PathBuf,
    instance_id: String,
    admitted: &CanonicalAdmittedSystemExtensionManifest,
) -> NativeExecutionTemplate {
    NativeExecutionTemplate {
        host_profile: native_profile.clone(),
        executable: executable.clone(),
        request: ExecutionRequest {
            schema: EXECUTION_REQUEST_SCHEMA.to_string(),
            operation_ref: HASH_A.to_string(),
            idempotency_ref: HASH_B.to_string(),
            extension_id: admitted.manifest().extension_id.clone(),
            service_id: admitted.manifest().service_id.clone(),
            callback_ref: HASH_C.to_string(),
            effect_ref: HASH_D.to_string(),
            generation: GENERATION,
            profile_ref: execution.profile.descriptor.profile_ref.clone(),
            executable_artifact_ref: executable.executable.executable_ref.clone(),
            executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
            arguments: Vec::new(),
            environment: Vec::new(),
            environment_mode: ExecutionEnvironmentMode::Clear,
            invocation_mode: ExecutionInvocationMode::Direct,
            executable_resolution: ExecutableResolutionMode::ExactArtifact,
            workspace_ref: HASH_D.to_string(),
            workspace_mode: WorkspaceMode::CapabilityRoot,
            stdin_ref: Some(HASH_E.to_string()),
            limits: ExecutionRequestLimits {
                timeout_ms: TIMEOUT_MS,
                stdin_max_bytes: CALLBACK_LIMIT,
                stdout_max_bytes: CALLBACK_LIMIT,
                stderr_max_bytes: DIAGNOSTIC_LIMIT,
                poll_interval_ms: POLL_INTERVAL_MS,
                teardown_timeout_ms: TEARDOWN_TIMEOUT_MS,
                concurrency_units: RESOURCE_UNITS,
                queue_units: RESOURCE_UNITS,
            },
            termination_scope: ExecutionTerminationScope::ProcessGroup,
            accepted_exit_codes: vec![SUCCESS_EXIT_CODE],
            reject_stdout_truncation: true,
            reject_stderr_truncation: true,
            authority_ref: executable.executable.authority_ref.clone(),
            resource_grant_ref: executable.executable.resource_ref.clone(),
        },
        authority: ExecutionAuthorityFacts {
            authority_ref: executable.executable.authority_ref.clone(),
            executable_authority_ref: HASH_A.to_string(),
            provenance_ref: executable.executable.provenance_ref.clone(),
            effect_admission_ref: HASH_B.to_string(),
            workspace_authority_ref: HASH_C.to_string(),
            process_authority_ref: HASH_D.to_string(),
            resource_grant_ref: executable.executable.resource_ref.clone(),
            policy_ref: executable.executable.policy_ref.clone(),
            executable_artifact_ref: executable.executable.executable_ref.clone(),
            executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
            workspace_ref: HASH_D.to_string(),
            operation_ref: HASH_A.to_string(),
            extension_id: admitted.manifest().extension_id.clone(),
            service_id: admitted.manifest().service_id.clone(),
            generation: GENERATION,
            profile_ref: execution.profile.descriptor.profile_ref.clone(),
        },
        resources: ExecutionResourceGrant {
            memory_bytes: CALLBACK_LIMIT + DIAGNOSTIC_LIMIT,
            storage_bytes: CALLBACK_LIMIT,
            diagnostic_bytes: CALLBACK_LIMIT + DIAGNOSTIC_LIMIT,
            logical_deadline_ticks: LOGICAL_DEADLINE,
            concurrency_units: RESOURCE_UNITS,
            queue_units: QUEUE_UNITS,
        },
        resolved: ResolvedExecutionContext {
            executable_path,
            executable_artifact_ref: executable.executable.executable_ref.clone(),
            executable_identity_ref: executable.executable.executable_bytes_ref.clone(),
            workspace_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            workspace_ref: HASH_D.to_string(),
            stdin_ref: None,
            stdin_bytes: None,
        },
        context: NativeCallbackContext {
            manifest_ref: admitted.manifest_ref().to_string(),
            executable_ref: executable.executable.executable_ref.clone(),
            instance_id,
            extension_id: admitted.manifest().extension_id.clone(),
            service_id: admitted.manifest().service_id.clone(),
            state_ref: None,
            policy_refs: admitted.manifest().policy_refs.clone(),
            resource_ref: executable.executable.resource_ref.clone(),
            port_binding_refs: admitted.all_binding_refs().map(str::to_string).collect(),
        },
    }
}

fn admitted_manifest() -> CanonicalAdmittedSystemExtensionManifest {
    let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::Execution,
            FabricAuthority::Resources,
            FabricAuthority::Supervision,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })
    .expect("native extension tier");
    canonical_admit_system_extension_manifest(
        &SystemExtensionManifestInput {
            schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
            extension_id: "molten.fixture.native".to_string(),
            service_id: "molten.fixture.native.service".to_string(),
            implementation_ref: HASH_E.to_string(),
            callback_groups: vec![
                "initialize".to_string(),
                "start".to_string(),
                "request".to_string(),
                "message".to_string(),
                "health".to_string(),
                "checkpoint".to_string(),
                "recover".to_string(),
                "drain".to_string(),
                "shutdown".to_string(),
            ],
            required_ports: vec![effect_requirement()],
            optional_ports: Vec::new(),
            capability_refs: vec![HASH_B.to_string()],
            policy_refs: vec![HASH_C.to_string()],
            provenance_refs: vec![HASH_D.to_string()],
            resources: ResourceEnvelope {
                max_concurrent_callbacks: MAX_CONCURRENT_CALLBACKS,
                max_queued_events: MAX_QUEUED_EVENTS,
                max_inflight_bytes: MAX_INFLIGHT_BYTES,
                max_open_streams: MAX_OPEN_STREAMS,
                max_timers: MAX_TIMERS,
                max_effect_requests: MAX_EFFECT_REQUESTS,
                callback_deadline_ticks: CALLBACK_DEADLINE_TICKS,
                shutdown_grace_ticks: SHUTDOWN_GRACE_TICKS,
                max_restart_attempts: MAX_RESTART_ATTEMPTS,
                overload_policy: OverloadPolicy::UpstreamBackpressure,
            },
            execution_profile: ExecutionProfile::NativeProcess,
            state_schema: "molten.fixture.native.state.v1".to_string(),
            compatible_state_schemas: vec!["molten.fixture.native.state.v1".to_string()],
            evidence_profile_ref: HASH_E.to_string(),
            initial_generation: GENERATION,
            non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
        },
        &[effect_descriptor()],
        &tier,
        &[ExecutionProfile::NativeProcess],
    )
    .expect("native manifest")
}

fn effect_descriptor() -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: EFFECT_PORT_ID.to_string(),
        version: EFFECT_PORT_VERSION.to_string(),
        class: FabricPortClass::Evidence,
        operation_classes: vec![EFFECT_OPERATION.to_string()],
        input_schema_refs: vec![EFFECT_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EFFECT_OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![FabricAuthority::Evidence],
        resource_requirements: vec![FabricResource::Diagnostics],
        determinism: DeterminismClass::ExternalEffect,
        replay: ReplayClass::RecordedEffectRequired,
        implementation_profile: "native-fixture-effect-v1".to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn effect_requirement() -> FabricPortRequirement {
    FabricPortRequirement {
        port_id: EFFECT_PORT_ID.to_string(),
        version: EFFECT_PORT_VERSION.to_string(),
        class: FabricPortClass::Evidence,
        operation_classes: vec![EFFECT_OPERATION.to_string()],
        input_schema_refs: vec![EFFECT_INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![EFFECT_OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![FabricAuthority::Evidence],
        available_resources: vec![FabricResource::Diagnostics],
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: "native-fixture-effect-v1".to_string(),
    }
}
