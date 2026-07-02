type PreservesValue = preserves::IOValue;
type ExternalKind = wasmparser::ExternalKind;
type Parser = wasmparser::Parser;
type Payload<'a> = wasmparser::Payload<'a>;
type Caller<'a, T> = wasmtime::Caller<'a, T>;
type Config = wasmtime::Config;
type Engine = wasmtime::Engine;
type Linker<T> = wasmtime::Linker<T>;
type Memory = wasmtime::Memory;
type Module = wasmtime::Module;
type Store<T> = wasmtime::Store<T>;
type StoreLimits = wasmtime::StoreLimits;
type StoreLimitsBuilder = wasmtime::StoreLimitsBuilder;

type AdmissionRequest = super::core::AdmissionRequest;
type CoreStep = super::core::CoreStep;
type ActorConfig = super::schema::ActorExecutorConfig;
type ActorMode = super::schema::ActorKind;
type Suite = super::schema::Suite;
type AbiReceipt = super::schema::WasmAbiReceiptInput;
type ExecReceipt<'a> = super::schema::WasmExecutionReceiptInput<'a>;
type ModuleConfig = super::schema::WasmExecutorConfig;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

fn validate_bound_request(hostcall_request: &PreservesValue, operation: &str) -> Result<()> {
    super::schema::validate_hostcall_effect_binding_request(hostcall_request, operation)
}

fn execution_receipt_value(input: ExecReceipt<'_>) -> PreservesValue {
    super::schema::wasm_execution_receipt_value(input)
}

fn export_name(operation: &str) -> String {
    super::schema::wasm_executor_export_name(operation)
}

fn module_bytes(config: &ModuleConfig) -> Result<Vec<u8>> {
    super::schema::wasm_module_bytes(config)
}

fn module_ref(config: &ModuleConfig) -> Result<String> {
    super::schema::wasm_module_ref(config)
}

fn canonical_bytes(value: &PreservesValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &PreservesValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<PreservesValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
}

const WASM_FUEL_LIMIT: u64 = 10_000;
const WASM_MEMORY_LIMIT_BYTES: usize = 64 * 1024;
const WASM_TABLE_ELEMENT_LIMIT: usize = 1024;
const WASM_ABI_MAX_INPUT_BYTES: usize = 8 * 1024;
const WASM_ABI_MAX_OUTPUT_BYTES: usize = 8 * 1024;
const WASM_ABI_MAX_HOSTCALL_BYTES: usize = 8 * 1024;
const WASM_ABI_HOSTCALL_RESPONSE_PTR: usize = 8 * 1024;
const WASM_MAX_EXPORTS: usize = 1024;

const _: () = assert!(WASM_MAX_EXPORTS <= 16_384);

#[derive(Debug)]
struct WasmExecutionState {
    hostcalls: Vec<String>,
    limits: StoreLimits,
}

pub struct WasmActorStepInput<'a> {
    pub suite: &'a Suite,
    pub step: &'a CoreStep,
    pub sequence: u64,
    pub step_ref: &'a str,
    pub actor_input: &'a PreservesValue,
    pub hostcall_request: &'a PreservesValue,
    pub hostcall_decision: &'a PreservesValue,
}

pub fn execute_wasm_actor_step(input: &WasmActorStepInput<'_>) -> Result<Option<PreservesValue>> {
    let Some(prepared) = prepare(input)? else {
        return Ok(None);
    };
    let compiled = compile(prepared.actor_id, &prepared.bytes)?;
    let mut linker = Linker::new(&compiled.engine);
    let decision_bytes = canonical_bytes(input.hostcall_decision)?;
    link_imports(&mut linker, LinkInput {
        actor_id: prepared.actor_id,
        allowed: &prepared.allowed,
        has_preserves_abi: prepared.has_preserves_abi,
        decision_bytes: &decision_bytes,
    })?;
    let mut store = new_store(&compiled.engine, prepared.actor_id)?;
    let instance = linker.instantiate(&mut store, &compiled.module).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor instantiation failed for actor {}; only declared molten:hostcall imports are linked and WASI is unavailable: {error}",
            prepared.actor_id
        ))
    })?;

    let abi_receipt = if prepared.has_preserves_abi {
        Some(execute_preserves_abi(
            prepared.actor_id,
            &instance,
            &mut store,
            &prepared.export,
            input.actor_input,
        )?)
    } else {
        let func = instance.get_typed_func::<(), ()>(&mut store, &prepared.export).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "Wasm executor actor {} missing required export {} for hostcall operation {}: {error}",
                prepared.actor_id, prepared.export, prepared.operation
            ))
        })?;
        func.call(&mut store, ()).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "Wasm executor actor {} export {} trapped while requesting hostcall {}: {error}",
                prepared.actor_id, prepared.export, prepared.operation
            ))
        })?;
        None
    };

    let fuel_remaining = store.get_fuel().map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor fuel readback failed for actor {}: {error}",
            prepared.actor_id
        ))
    })?;
    let hostcalls = store.data().hostcalls.clone();
    require_single_call(&hostcalls, &prepared, input.sequence, input.step_ref)?;

    Ok(Some(execution_receipt_value(ExecReceipt {
        actor_id: prepared.actor_id,
        module_ref: &prepared.module_ref,
        export: &prepared.export,
        operation: &prepared.operation,
        hostcalls: &hostcalls,
        fuel_limit: WASM_FUEL_LIMIT,
        fuel_remaining,
        memory_limit_bytes: WASM_MEMORY_LIMIT_BYTES as u64,
        abi: abi_receipt,
    })))
}

struct Prepared<'a> {
    actor_id: &'a str,
    operation: String,
    export: String,
    bytes: Vec<u8>,
    allowed: Vec<String>,
    has_preserves_abi: bool,
    module_ref: String,
}

struct Compiled {
    engine: Engine,
    module: Module,
}

struct LinkInput<'a> {
    actor_id: &'a str,
    allowed: &'a [String],
    has_preserves_abi: bool,
    decision_bytes: &'a [u8],
}

fn prepare<'a>(input: &WasmActorStepInput<'a>) -> Result<Option<Prepared<'a>>> {
    let actor_id = input.step.primary_actor();
    let Some(actor) = input.suite.actors.iter().find(|actor| actor.id == actor_id) else {
        return Err(MoltenError::invalid_harness(format!("actor {actor_id} missing from executor registry")));
    };
    if actor.kind != ActorMode::Wasm {
        return Ok(None);
    }
    let Some(ActorConfig::Wasm(config)) = actor.executor.as_ref() else {
        return Err(MoltenError::invalid_harness(format!(
            "wasm actor {actor_id} missing Wasm executor preflight fixture"
        )));
    };

    let operation = AdmissionRequest::from_step(input.step).action.as_str().to_string();
    validate_bound_request(input.hostcall_request, &operation)?;
    let export = export_name(&operation);
    let bytes = module_bytes(config)?;
    let has_preserves_abi = module_exports(&bytes)?
        .iter()
        .any(|(name, kind)| name == "molten_alloc" && *kind == ExternalKind::Func);
    let module_ref = module_ref(config)?;

    Ok(Some(Prepared {
        actor_id,
        operation,
        export,
        bytes,
        allowed: config.allowed_hostcalls.clone(),
        has_preserves_abi,
        module_ref,
    }))
}

fn compile(actor_id: &str, bytes: &[u8]) -> Result<Compiled> {
    let mut engine_config = Config::new();
    engine_config.consume_fuel(true);
    let engine = Engine::new(&engine_config).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor engine creation failed for actor {actor_id}: {error}"))
    })?;
    let module = Module::from_binary(&engine, bytes).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor module for actor {actor_id} failed Wasmtime compilation; core-module execution is required and components remain fail-closed: {error}"
        ))
    })?;
    Ok(Compiled { engine, module })
}

fn link_imports(linker: &mut Linker<WasmExecutionState>, input: LinkInput<'_>) -> Result<()> {
    if input.decision_bytes.len() > WASM_ABI_MAX_HOSTCALL_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor hostcall decision bytes for actor {} exceed molten.wasm.abi.v1 limit",
            input.actor_id
        )));
    }
    for hostcall in input.allowed {
        let captured_hostcall = hostcall.clone();
        if input.has_preserves_abi {
            let response_bytes = input.decision_bytes.to_vec();
            linker
                .func_wrap(
                    "molten:hostcall",
                    hostcall,
                    move |mut caller: Caller<'_, WasmExecutionState>, ptr: i32, len: i32| -> wasmtime::Result<i64> {
                        validate_hostcall_preserves_bytes(&mut caller, ptr, len)?;
                        write_hostcall_response_bytes(&mut caller, &response_bytes)?;
                        caller.data_mut().hostcalls.push(captured_hostcall.clone());
                        Ok(((WASM_ABI_HOSTCALL_RESPONSE_PTR as u64) << 32 | response_bytes.len() as u64) as i64)
                    },
                )
                .map_err(|error| {
                    MoltenError::invalid_harness(format!(
                        "Wasm executor hostcall linker setup failed for actor {}: {error}",
                        input.actor_id
                    ))
                })?;
        } else {
            linker
                .func_wrap("molten:hostcall", hostcall, move |mut caller: Caller<'_, WasmExecutionState>| {
                    caller.data_mut().hostcalls.push(captured_hostcall.clone());
                })
                .map_err(|error| {
                    MoltenError::invalid_harness(format!(
                        "Wasm executor hostcall linker setup failed for actor {}: {error}",
                        input.actor_id
                    ))
                })?;
        }
    }
    Ok(())
}

fn new_store(engine: &Engine, actor_id: &str) -> Result<Store<WasmExecutionState>> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(WASM_MEMORY_LIMIT_BYTES)
        .table_elements(WASM_TABLE_ELEMENT_LIMIT)
        .instances(1)
        .memories(1)
        .tables(1)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(engine, WasmExecutionState {
        hostcalls: Vec::new(),
        limits,
    });
    store.limiter(|state| &mut state.limits);
    store.set_fuel(WASM_FUEL_LIMIT).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor fuel setup failed for actor {actor_id}: {error}"))
    })?;
    Ok(store)
}

fn require_single_call(hostcalls: &[String], prepared: &Prepared<'_>, sequence: u64, step_ref: &str) -> Result<()> {
    let expected = std::slice::from_ref(&prepared.operation);
    if hostcalls != expected {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {} requested hostcalls {:?}, expected exactly {:?} for step {sequence} ({step_ref})",
            prepared.actor_id, hostcalls, expected
        )));
    }
    Ok(())
}
