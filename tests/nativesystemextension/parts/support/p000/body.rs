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
pub const EFFECT_OUTPUT_BYTES: &[u8] = b"fixture-native-effect-output-v2";
pub const GENERATION: u64 = 1;
pub const MAX_VALUE_BYTES: u64 = 262_144;
const REQUEST_PAYLOAD: &[u8] = b"fixture-ingress-request-v2";
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

pub type TestResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Turns a failed fixture step into a test error that keeps its label and `Debug` form.
pub trait OrFail<T> {
    fn or_fail(self, label: &str) -> TestResult<T>;
}

impl<T, E: std::fmt::Debug> OrFail<T> for std::result::Result<T, E> {
    fn or_fail(self, label: &str) -> TestResult<T> {
        self.map_err(|error| format!("{label}: {error:?}").into())
    }
}

impl<T> OrFail<T> for Option<T> {
    fn or_fail(self, label: &str) -> TestResult<T> {
        self.ok_or_else(|| format!("{label}: value is absent").into())
    }
}

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
        let output_ref = molten::preserves_rail::content_ref_from_bytes(EFFECT_OUTPUT_BYTES);
        Ok(PortEffectOutput {
            output_schema_ref: EFFECT_OUTPUT_SCHEMA.to_string(),
            output_ref: output_ref.clone(),
            materialized_output: Some(NativeCallbackValue {
                value_ref: output_ref,
                bytes: EFFECT_OUTPUT_BYTES.to_vec(),
            }),
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
    pub journal: std::sync::Arc<std::sync::Mutex<Journal>>,
    pub values: SharedNativeCallbackValuePort,
}

impl Cohort {
    pub fn new() -> TestResult<Self> {
        let executable_path = std::path::PathBuf::from(env!("CARGO_BIN_EXE_molten-native-extension-fixture"));
        let executable_bytes = std::fs::read(&executable_path).or_fail("read native fixture executable")?;
        let executable_bytes_ref = molten::preserves_rail::content_ref_from_bytes(&executable_bytes);
        let execution_profile =
            canonical_admit_execution_profile(&execution_profile_descriptor()).or_fail("execution profile")?;
        let native_profile =
            admit_native_host_profile(&native_profile(&execution_profile)).or_fail("native host profile")?;
        let admitted = admitted_manifest()?;
        let executable = admit_native_executable(
            &native_profile,
            &executable_evidence(&admitted, &execution_profile, &executable_bytes_ref),
        )
        .or_fail("native executable evidence")?;
        let instance_id = native_identity_ref(&[
            "native-instance-v2",
            &admitted.manifest().extension_id,
            &admitted.manifest().service_id,
            admitted.manifest_ref(),
        ]);
        let template = execution_template(ExecutionTemplateInput {
            native_profile: &native_profile,
            executable: &executable,
            execution: &execution_profile,
            executable_path,
            instance_id,
            admitted: &admitted,
        });
        Ok(Self {
            native_profile,
            executable,
            admitted,
            execution_profile,
            template,
            journal: std::sync::Arc::new(std::sync::Mutex::new(Journal::empty())),
            values: shared_native_callback_value_port(InMemoryNativeCallbackValuePort::empty()),
        })
    }

    pub fn replace_program(
        &mut self,
        path: std::path::PathBuf,
        arguments: Vec<String>,
        timeout_ms: u64,
        stdout_max_bytes: u64,
    ) {
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

    fn admission(&self) -> AdmissionSet {
        AdmissionSet {
            profile: self.native_profile.clone(),
            executable: self.executable.clone(),
            admitted: self.admitted.clone(),
        }
    }

    pub fn install(&self) -> TestResult<Service> {
        let port = LiveExecutionAdapter::new(self.execution_profile.clone(), Publisher::default())
            .or_fail("live execution adapter")?;
        NativeSystemExtensionService::install(
            self.admission(),
            port,
            self.journal.clone(),
            self.values.clone(),
            self.template.clone(),
        )
        .or_fail("install native service")
    }

    pub fn recovered(&self, instance: NativeInstanceRecord) -> TestResult<Service> {
        let instance = std::sync::Arc::new(std::sync::Mutex::new(instance));
        let port = LiveExecutionAdapter::new(self.execution_profile.clone(), Publisher::default())
            .or_fail("live execution adapter")?;
        let executor = NativeProcessSystemExtensionExecutor::new(
            port,
            self.journal.clone(),
            self.values.clone(),
            instance.clone(),
            self.template.clone(),
        )
        .or_fail("recovered native executor")?;
        NativeSystemExtensionService::from_recovered(self.admission(), executor, self.journal.clone(), instance)
            .or_fail("recover native service")
    }
}

pub fn ingress(generation: u64, manifest_ref: &str) -> TestResult<NativeIngressEnvelope> {
    let payload = REQUEST_PAYLOAD.to_vec();
    let accounted_bytes = u64::try_from(payload.len()).or_fail("fixture ingress length")?;
    Ok(NativeIngressEnvelope {
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
        payload: NativeCallbackValue {
            value_ref: molten::preserves_rail::content_ref_from_bytes(&payload),
            bytes: payload,
        },
        accounted_bytes,
    })
}
