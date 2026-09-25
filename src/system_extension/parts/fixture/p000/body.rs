use super::FabricEffectPort;
use super::SystemExtensionExecutor;

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
    pub profile: super::ExecutionProfile,
    pub manifest_ref: String,
    pub manifest_value: preserves::IOValue,
    pub evidence: Vec<super::HostEvidence>,
    pub conformance: super::ExecutableConformanceInput,
    pub first_request_effects: Vec<super::TypedEffectRequest>,
    pub first_effect_completions: Vec<super::CanonicalEffectCompletion>,
    pub upgraded_status: super::CanonicalOperatorStatus,
    pub rolled_back_status: super::CanonicalOperatorStatus,
    pub recovered_status: super::CanonicalOperatorStatus,
    pub final_status: super::CanonicalOperatorStatus,
}

struct WasmProbe {
    engine: wasmtime::Engine,
    module: wasmtime::Module,
}

impl WasmProbe {
    fn new() -> crate::error::Result<Self> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!(
                "sandboxed fixture engine initialization failed: {error}"
            ))
        })?;
        let module = wasmtime::Module::new(&engine, WASM_PROBE_SOURCE).map_err(|error| {
            crate::error::MoltenError::invalid_harness(format!("sandboxed fixture module compilation failed: {error}"))
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
        effect: &super::TypedEffectRequest,
    ) -> crate::fabric::FabricPortResult<super::PortEffectOutput> {
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
        Ok(super::PortEffectOutput {
            output_schema_ref: OUTPUT_SCHEMA.to_string(),
            output_ref: HASH_C.to_string(),
            materialized_output: None,
        })
    }
}

struct EchoExecutor {
    profile: super::ExecutionProfile,
    request_count: u64,
    wasm_probe: Option<WasmProbe>,
}

impl EchoExecutor {
    fn new(profile: super::ExecutionProfile) -> crate::error::Result<Self> {
        let wasm_probe = if profile == super::ExecutionProfile::SandboxedComponent {
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
    fn execution_profile(&self) -> super::ExecutionProfile {
        self.profile
    }

    fn invoke(
        &mut self,
        invocation: &super::CallbackInvocation,
    ) -> std::result::Result<super::CallbackOutcome, String> {
        if let Some(wasm_probe) = &self.wasm_probe {
            wasm_probe.invoke(invocation.sequence)?;
        }
        if invocation.callback == super::CallbackKind::Request {
            self.request_count =
                self.request_count.checked_add(1).ok_or_else(|| "fixture request counter overflow".to_string())?;
            if self.request_count == FAILING_REQUEST_NUMBER {
                return Err("fixture retryable callback failure".to_string());
            }
        }
        let effects = if invocation.callback == super::CallbackKind::Request {
            vec![super::TypedEffectRequest {
                target: super::EffectTarget::FabricPort(crate::fabric::FabricPortKey {
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
        let checkpoint_ref = if invocation.callback == super::CallbackKind::Checkpoint {
            Some(HASH_E.to_string())
        } else {
            None
        };
        Ok(super::CallbackOutcome {
            output_refs: vec![HASH_A.to_string()],
            effects,
            state_ref: Some(HASH_B.to_string()),
            checkpoint_ref,
            health: super::HealthState::Healthy,
        })
    }
}

// r[impl molten.system_extension.callbacks]
// r[impl molten.system_extension.final_validation]
pub fn run_executable_system_extension_fixture(
    profile: super::ExecutionProfile,
) -> crate::error::Result<ExecutableSystemExtensionFixtureRun> {
    if profile == super::ExecutionProfile::NativeProcess {
        return Err(crate::error::MoltenError::invalid_harness(
            "the deterministic fixture admits in-process-native or sandboxed-component profiles only",
        ));
    }
    let [admitted, upgrade_manifest, rollback_manifest] = fixture_manifests(profile)?;
    let manifest_ref = admitted.manifest_ref().to_string();
    let manifest_value = admitted.value().clone();
    let mut host = super::SystemExtensionHost::new(admitted, EchoExecutor::new(profile)?)?;

    host.activate(START_TICK)?;
    host.health(HEALTH_TICK)?.require_executed("health")?;
    let (first_request_receipt, first_request_effects) =
        match host.dispatch_request(HASH_C, REQUEST_BYTES, REQUEST_TICK)? {
            super::HostDispatchResult::Executed {
                receipt,
                approved_effects,
                ..
            } => (receipt, approved_effects),
            other => {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "fixture first request did not execute: {other:?}"
                )));
            }
        };
    let mut transport_port = FixtureTransportPort::default();
    let first_effect_completions = host.route_approved_effects(&first_request_receipt, &mut transport_port)?;
    host.checkpoint(CHECKPOINT_TICK)?;
    let checkpoint_ref =
        host.state().checkpoint_ref.clone().ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("fixture checkpoint ref missing after checkpoint")
        })?;
    let upgraded_status =
        host.upgrade(upgrade_manifest, EchoExecutor::new(profile)?, &checkpoint_ref, UPGRADE_TICK)?.status;
    let rolled_back_status = host
        .rollback(rollback_manifest, EchoExecutor::new(profile)?, &checkpoint_ref, ROLLBACK_TICK)?
        .status;
    host.dispatch_request(HASH_C, REQUEST_BYTES, POST_ROLLBACK_REQUEST_TICK)?
        .require_executed("post-rollback request")?;
    let recovered_status = fail_and_recover(&mut host)?;
    host.drain(DRAIN_TICK)?;
    host.shutdown(SHUTDOWN_TICK)?;
    let final_status = host.operator_status()?;

    let conformance = validated_conformance(&host)?;

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
