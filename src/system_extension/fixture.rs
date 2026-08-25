use preserves::IOValue;

use super::CallbackKind;
use super::CallbackOutcome;
use super::CanonicalEffectCompletion;
use super::CanonicalOperatorStatus;
use super::EffectTarget;
use super::ExecutableConformanceInput;
use super::ExecutionProfile;
use super::FabricEffectPort;
use super::HealthState;
use super::HostDispatchResult;
use super::HostEvidence;
use super::LifecyclePhase;
use super::OverloadPolicy;
use super::PortEffectOutput;
use super::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS;
use super::ResourceEnvelope;
use super::SYSTEM_EXTENSION_MANIFEST_SCHEMA;
use super::SystemExtensionExecutor;
use super::SystemExtensionHost;
use super::SystemExtensionManifestInput;
use super::TypedEffectRequest;
use super::canonical_admit_system_extension_manifest;
use super::validate_executable_conformance;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric::DeterminismClass;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierRequest;
use crate::fabric::FABRIC_PORT_DESCRIPTOR_SCHEMA;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortClass;
use crate::fabric::FabricPortDescriptor;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricPortRequirement;
use crate::fabric::FabricResource;
use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;
use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
use crate::fabric::ReplayClass;
use crate::fabric::canonical_extension_tier_admission;

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const HASH_E: &str = "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const EXTENSION_ID: &str = "molten.fixture.system.echo";
const SERVICE_ID: &str = "molten.fixture.system.echo.service";
const STATE_SCHEMA: &str = "molten.fixture.system.echo.state.v1";
const UPGRADED_STATE_SCHEMA: &str = "molten.fixture.system.echo.state.v2";
const PORT_ID: &str = "molten.fabric.transport.session";
const PORT_VERSION: &str = "v1";
const PORT_PROFILE: &str = "fixture-transport-v1";
const PORT_OPERATION: &str = "send-envelope";
const INPUT_SCHEMA: &str = "molten.fixture.transport.input.v1";
const OUTPUT_SCHEMA: &str = "molten.fixture.transport.output.v1";
const INITIAL_GENERATION: u64 = 1;
const MAX_CONCURRENT_CALLBACKS: u64 = 2;
const MAX_QUEUED_EVENTS: u64 = 2;
const MAX_INFLIGHT_BYTES: u64 = 4_096;
const MAX_OPEN_STREAMS: u64 = 2;
const MAX_TIMERS: u64 = 2;
const MAX_EFFECT_REQUESTS: u64 = 4;
const CALLBACK_DEADLINE_TICKS: u64 = 16;
const SHUTDOWN_GRACE_TICKS: u64 = 32;
const MAX_RESTART_ATTEMPTS: u64 = 1;
const REQUEST_BYTES: u64 = 64;
const START_TICK: u64 = 10;
const HEALTH_TICK: u64 = 15;
const REQUEST_TICK: u64 = 20;
const CHECKPOINT_TICK: u64 = 30;
const UPGRADE_TICK: u64 = 40;
const ROLLBACK_TICK: u64 = 50;
const POST_ROLLBACK_REQUEST_TICK: u64 = 60;
const FAILURE_TICK: u64 = 70;
const RECOVERY_TICK: u64 = 80;
const POST_RECOVERY_TICK: u64 = 90;
const DRAIN_TICK: u64 = 100;
const SHUTDOWN_TICK: u64 = 110;
const FAILING_REQUEST_NUMBER: u64 = 2;
const WASM_FIXTURE_FUEL: u64 = 10_000;
const WASM_PROBE_SOURCE: &str = r#"
(module
  (func (export "invoke") (param i64) (result i64)
    local.get 0))
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableSystemExtensionFixtureRun {
    pub profile: ExecutionProfile,
    pub manifest_ref: String,
    pub manifest_value: IOValue,
    pub evidence: Vec<HostEvidence>,
    pub conformance: ExecutableConformanceInput,
    pub first_request_effects: Vec<TypedEffectRequest>,
    pub first_effect_completions: Vec<CanonicalEffectCompletion>,
    pub upgraded_status: CanonicalOperatorStatus,
    pub rolled_back_status: CanonicalOperatorStatus,
    pub recovered_status: CanonicalOperatorStatus,
    pub final_status: CanonicalOperatorStatus,
}

struct WasmProbe {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
}

impl WasmProbe {
    fn new() -> Result<Self> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|error| {
            MoltenError::invalid_harness(format!("sandboxed fixture engine initialization failed: {error}"))
        })?;
        let module = wasmtime::Module::new(&engine, WASM_PROBE_SOURCE).map_err(|error| {
            MoltenError::invalid_harness(format!("sandboxed fixture module compilation failed: {error}"))
        })?;
        Ok(Self { engine, module })
    }

    fn invoke(&self, sequence: u64) -> std::result::Result<(), String> {
        let sequence = i64::try_from(sequence).map_err(|error| format!("callback sequence out of range: {error}"))?;
        let mut store = wasmtime::Store::new(&self.engine, ());
        store
            .set_fuel(WASM_FIXTURE_FUEL)
            .map_err(|error| format!("sandboxed fixture fuel setup failed: {error}"))?;
        let instance = wasmtime::Instance::new(&mut store, &self.module, &[])
            .map_err(|error| format!("sandboxed fixture instantiation failed: {error}"))?;
        let invoke = instance
            .get_typed_func::<i64, i64>(&mut store, "invoke")
            .map_err(|error| format!("sandboxed fixture export lookup failed: {error}"))?;
        let observed = invoke
            .call(&mut store, sequence)
            .map_err(|error| format!("sandboxed fixture callback trapped: {error}"))?;
        if observed != sequence {
            return Err("sandboxed fixture callback result mismatch".to_string());
        }
        Ok(())
    }
}

#[derive(Default)]
struct FixtureTransportPort {
    routed: u64,
}

impl FabricEffectPort for FixtureTransportPort {
    fn route(
        &mut self,
        binding: &crate::fabric::CanonicalFabricPortBinding,
        effect: &TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<PortEffectOutput> {
        if binding.binding.key.port_id != PORT_ID
            || binding.binding.key.version != PORT_VERSION
            || effect.operation != PORT_OPERATION
        {
            return Err(crate::fabric::FabricPortError::malformed("fixture received an unexpected fabric binding"));
        }
        self.routed = self
            .routed
            .checked_add(1)
            .ok_or_else(|| crate::fabric::FabricPortError::malformed("fixture route counter overflow"))?;
        Ok(PortEffectOutput {
            output_schema_ref: OUTPUT_SCHEMA.to_string(),
            output_ref: HASH_C.to_string(),
        })
    }
}

struct EchoExecutor {
    profile: ExecutionProfile,
    request_count: u64,
    wasm_probe: Option<WasmProbe>,
}

impl EchoExecutor {
    fn new(profile: ExecutionProfile) -> Result<Self> {
        let wasm_probe = if profile == ExecutionProfile::SandboxedComponent {
            Some(WasmProbe::new()?)
        } else {
            None
        };
        Ok(Self {
            profile,
            request_count: 0,
            wasm_probe,
        })
    }
}

impl SystemExtensionExecutor for EchoExecutor {
    fn execution_profile(&self) -> ExecutionProfile {
        self.profile
    }

    fn invoke(&mut self, invocation: &super::CallbackInvocation) -> std::result::Result<CallbackOutcome, String> {
        if let Some(wasm_probe) = &self.wasm_probe {
            wasm_probe.invoke(invocation.sequence)?;
        }
        if invocation.callback == CallbackKind::Request {
            self.request_count =
                self.request_count.checked_add(1).ok_or_else(|| "fixture request counter overflow".to_string())?;
            if self.request_count == FAILING_REQUEST_NUMBER {
                return Err("fixture retryable callback failure".to_string());
            }
        }
        let effects = if invocation.callback == CallbackKind::Request {
            vec![TypedEffectRequest {
                target: EffectTarget::FabricPort(FabricPortKey {
                    port_id: PORT_ID.to_string(),
                    version: PORT_VERSION.to_string(),
                }),
                operation: PORT_OPERATION.to_string(),
                input_schema_ref: INPUT_SCHEMA.to_string(),
                output_schema_ref: OUTPUT_SCHEMA.to_string(),
                request_ref: HASH_D.to_string(),
                generation: invocation.generation,
                accounted_bytes: REQUEST_BYTES,
            }]
        } else {
            Vec::new()
        };
        let checkpoint_ref = if invocation.callback == CallbackKind::Checkpoint {
            Some(HASH_E.to_string())
        } else {
            None
        };
        Ok(CallbackOutcome {
            output_refs: vec![HASH_A.to_string()],
            effects,
            state_ref: Some(HASH_B.to_string()),
            checkpoint_ref,
            health: HealthState::Healthy,
        })
    }
}

// r[impl molten.system_extension.callbacks]
// r[impl molten.system_extension.final_validation]
pub fn run_executable_system_extension_fixture(
    profile: ExecutionProfile,
) -> Result<ExecutableSystemExtensionFixtureRun> {
    if profile == ExecutionProfile::NativeProcess {
        return Err(MoltenError::invalid_harness(
            "the deterministic fixture admits in-process-native or sandboxed-component profiles only",
        ));
    }
    let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
        tier: ExtensionTier::SystemExtension,
        requested_authorities: vec![
            FabricAuthority::Transport,
            FabricAuthority::Resources,
            FabricAuthority::Supervision,
            FabricAuthority::Evidence,
        ],
        admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
    })?;
    let descriptors = [port_descriptor()];
    let admitted =
        canonical_admit_system_extension_manifest(&manifest_input(profile), &descriptors, &tier, &[profile])?;
    let mut upgrade_input = manifest_input(profile);
    upgrade_input.implementation_ref = HASH_B.to_string();
    upgrade_input.state_schema = UPGRADED_STATE_SCHEMA.to_string();
    upgrade_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
    let upgrade_manifest = canonical_admit_system_extension_manifest(&upgrade_input, &descriptors, &tier, &[profile])?;
    let mut rollback_input = manifest_input(profile);
    rollback_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
    let rollback_manifest =
        canonical_admit_system_extension_manifest(&rollback_input, &descriptors, &tier, &[profile])?;
    let manifest_ref = admitted.manifest_ref().to_string();
    let manifest_value = admitted.value().clone();
    let mut host = SystemExtensionHost::new(admitted, EchoExecutor::new(profile)?)?;

    host.activate(START_TICK)?;
    host.health(HEALTH_TICK)?.require_executed("health")?;
    let (first_request_receipt, first_request_effects) =
        match host.dispatch_request(HASH_C, REQUEST_BYTES, REQUEST_TICK)? {
            HostDispatchResult::Executed {
                receipt,
                approved_effects,
                ..
            } => (receipt, approved_effects),
            other => {
                return Err(MoltenError::invalid_harness(format!("fixture first request did not execute: {other:?}")));
            }
        };
    let mut transport_port = FixtureTransportPort::default();
    let first_effect_completions = host.route_approved_effects(&first_request_receipt, &mut transport_port)?;
    host.checkpoint(CHECKPOINT_TICK)?;
    let checkpoint_ref = host
        .state()
        .checkpoint_ref
        .clone()
        .ok_or_else(|| MoltenError::invalid_harness("fixture checkpoint ref missing after checkpoint"))?;
    let upgraded_status =
        host.upgrade(upgrade_manifest, EchoExecutor::new(profile)?, &checkpoint_ref, UPGRADE_TICK)?.status;
    let rolled_back_status = host
        .rollback(rollback_manifest, EchoExecutor::new(profile)?, &checkpoint_ref, ROLLBACK_TICK)?
        .status;
    host.dispatch_request(HASH_C, REQUEST_BYTES, POST_ROLLBACK_REQUEST_TICK)?
        .require_executed("post-rollback request")?;
    match host.dispatch_request(HASH_C, REQUEST_BYTES, FAILURE_TICK)? {
        HostDispatchResult::Failed { .. } => {}
        other => {
            return Err(MoltenError::invalid_harness(format!("fixture retryable request did not fail: {other:?}")));
        }
    }
    if host.state().phase != LifecyclePhase::Failed {
        return Err(MoltenError::invalid_harness("fixture retryable failure did not enter failed phase"));
    }
    host.restart(RECOVERY_TICK)?;
    let recovered_status = host.operator_status()?;
    if recovered_status.status.phase != LifecyclePhase::Running {
        return Err(MoltenError::invalid_harness("fixture recovery did not return to running phase"));
    }
    host.dispatch_request(HASH_C, REQUEST_BYTES, POST_RECOVERY_TICK)?
        .require_executed("post-recovery request")?;
    host.drain(DRAIN_TICK)?;
    host.shutdown(SHUTDOWN_TICK)?;
    let final_status = host.operator_status()?;

    let required_callbacks = vec![
        CallbackKind::Initialize,
        CallbackKind::Start,
        CallbackKind::Request,
        CallbackKind::Health,
        CallbackKind::Checkpoint,
        CallbackKind::Recover,
        CallbackKind::Drain,
        CallbackKind::Shutdown,
    ];
    let conformance = host.executable_conformance_input(required_callbacks);
    let issues = validate_executable_conformance(&conformance);
    if !issues.is_empty() {
        return Err(MoltenError::invalid_harness(format!("executable fixture conformance denied: {issues:?}")));
    }

    Ok(ExecutableSystemExtensionFixtureRun {
        profile,
        manifest_ref,
        manifest_value,
        evidence: host.evidence().to_vec(),
        conformance,
        first_request_effects,
        first_effect_completions,
        upgraded_status,
        rolled_back_status,
        recovered_status,
        final_status,
    })
}

fn manifest_input(profile: ExecutionProfile) -> SystemExtensionManifestInput {
    SystemExtensionManifestInput {
        schema: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        extension_id: EXTENSION_ID.to_string(),
        service_id: SERVICE_ID.to_string(),
        implementation_ref: HASH_A.to_string(),
        callback_groups: vec![
            "initialize".to_string(),
            "start".to_string(),
            "request".to_string(),
            "health".to_string(),
            "checkpoint".to_string(),
            "recover".to_string(),
            "drain".to_string(),
            "shutdown".to_string(),
        ],
        required_ports: vec![port_requirement()],
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
        execution_profile: profile,
        state_schema: STATE_SCHEMA.to_string(),
        compatible_state_schemas: vec![STATE_SCHEMA.to_string()],
        evidence_profile_ref: HASH_E.to_string(),
        initial_generation: INITIAL_GENERATION,
        non_claims: REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS.to_vec(),
    }
}

fn port_descriptor() -> FabricPortDescriptor {
    FabricPortDescriptor {
        schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: FabricPortClass::Transport,
        operation_classes: vec![PORT_OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        authority_requirements: vec![FabricAuthority::Transport],
        resource_requirements: vec![FabricResource::Concurrency, FabricResource::NetworkBytes],
        determinism: DeterminismClass::ExternalEffect,
        replay: ReplayClass::RecordedEffectRequired,
        implementation_profile: PORT_PROFILE.to_string(),
        conformance_refs: vec![HASH_A.to_string()],
        non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
        enabled: true,
    }
}

fn port_requirement() -> FabricPortRequirement {
    FabricPortRequirement {
        port_id: PORT_ID.to_string(),
        version: PORT_VERSION.to_string(),
        class: FabricPortClass::Transport,
        operation_classes: vec![PORT_OPERATION.to_string()],
        input_schema_refs: vec![INPUT_SCHEMA.to_string()],
        output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
        allowed_authorities: vec![FabricAuthority::Transport],
        available_resources: vec![FabricResource::Concurrency, FabricResource::NetworkBytes],
        expected_determinism: DeterminismClass::ExternalEffect,
        expected_replay: ReplayClass::RecordedEffectRequired,
        expected_profile: PORT_PROFILE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // r[verify molten.system_extension.lifecycle]
    #[test]
    fn stale_checkpoint_denies_upgrade_before_generation_or_evidence_changes() {
        let profile = ExecutionProfile::InProcessNative;
        let tier = canonical_extension_tier_admission(&ExtensionTierRequest {
            tier: ExtensionTier::SystemExtension,
            requested_authorities: vec![
                FabricAuthority::Transport,
                FabricAuthority::Resources,
                FabricAuthority::Supervision,
                FabricAuthority::Evidence,
            ],
            admission_evidence: REQUIRED_SYSTEM_EXTENSION_EVIDENCE.to_vec(),
        })
        .expect("tier admission");
        let descriptors = [port_descriptor()];
        let admitted =
            canonical_admit_system_extension_manifest(&manifest_input(profile), &descriptors, &tier, &[profile])
                .expect("initial manifest");
        let mut upgrade_input = manifest_input(profile);
        upgrade_input.state_schema = UPGRADED_STATE_SCHEMA.to_string();
        upgrade_input.compatible_state_schemas = vec![STATE_SCHEMA.to_string(), UPGRADED_STATE_SCHEMA.to_string()];
        let upgrade = canonical_admit_system_extension_manifest(&upgrade_input, &descriptors, &tier, &[profile])
            .expect("upgrade manifest");
        let mut host =
            SystemExtensionHost::new(admitted, EchoExecutor::new(profile).expect("executor")).expect("extension host");
        host.activate(START_TICK).expect("activation");
        host.checkpoint(CHECKPOINT_TICK).expect("checkpoint");
        let before_state = host.state().clone();
        let before_evidence_count = host.evidence().len();

        let error = host
            .upgrade(upgrade, EchoExecutor::new(profile).expect("upgrade executor"), HASH_A, UPGRADE_TICK)
            .expect_err("stale checkpoint must deny upgrade");

        assert!(error.to_string().contains("checkpoint does not match"));
        assert_eq!(host.state(), &before_state);
        assert_eq!(host.evidence().len(), before_evidence_count);
    }
}
