use preserves::IOValue;
use wasmparser::ExternalKind;
use wasmparser::Parser;
use wasmparser::Payload;
use wasmtime::Caller;
use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::Linker;
use wasmtime::Memory;
use wasmtime::Module;
use wasmtime::Store;
use wasmtime::StoreLimits;
use wasmtime::StoreLimitsBuilder;
use wasmtime::format_err;

use super::core::AdmissionRequest;
use super::core::CoreStep;
use super::schema::ActorExecutorConfig;
use super::schema::ActorKind;
use super::schema::HarnessSuite;
use super::schema::WasmAbiReceiptInput;
use super::schema::WasmExecutionReceiptInput;
use super::schema::wasm_execution_receipt_value;
use super::schema::wasm_executor_export_name;
use super::schema::wasm_module_bytes;
use super::schema::wasm_module_ref;
use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::canonical_bytes;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::parse_canonical_bytes;

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
    pub suite: &'a HarnessSuite,
    pub step: &'a CoreStep,
    pub sequence: u64,
    pub step_ref: &'a str,
    pub actor_input: &'a IOValue,
    pub hostcall_decision: &'a IOValue,
}

pub fn execute_wasm_actor_step(input: &WasmActorStepInput<'_>) -> Result<Option<IOValue>> {
    let suite = input.suite;
    let step = input.step;
    let sequence = input.sequence;
    let step_ref = input.step_ref;
    let actor_input = input.actor_input;
    let hostcall_decision = input.hostcall_decision;
    let actor_id = step.primary_actor();
    let Some(actor) = suite.actors.iter().find(|actor| actor.id == actor_id) else {
        return Err(MoltenError::invalid_harness(format!("actor {actor_id} missing from executor registry")));
    };
    if actor.kind != ActorKind::Wasm {
        return Ok(None);
    }
    let Some(ActorExecutorConfig::Wasm(config)) = actor.executor.as_ref() else {
        return Err(MoltenError::invalid_harness(format!(
            "wasm actor {actor_id} missing Wasm executor preflight fixture"
        )));
    };

    let operation = AdmissionRequest::from_step(step).action.as_str().to_string();
    let export = wasm_executor_export_name(&operation);
    let bytes = wasm_module_bytes(config)?;
    let has_preserves_abi = module_exports(&bytes)?
        .iter()
        .any(|(name, kind)| name == "molten_alloc" && *kind == ExternalKind::Func);

    let mut engine_config = Config::new();
    engine_config.consume_fuel(true);
    let engine = Engine::new(&engine_config).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor engine creation failed for actor {actor_id}: {error}"))
    })?;
    let module = Module::from_binary(&engine, &bytes).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor module for actor {actor_id} failed Wasmtime compilation; core-module execution is required and components remain fail-closed: {error}"
        ))
    })?;

    let mut linker = Linker::new(&engine);
    let hostcall_decision_bytes = canonical_bytes(hostcall_decision)?;
    if hostcall_decision_bytes.len() > WASM_ABI_MAX_HOSTCALL_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor hostcall decision bytes for actor {actor_id} exceed molten.wasm.abi.v1 limit"
        )));
    }
    for hostcall in &config.allowed_hostcalls {
        let captured_hostcall = hostcall.clone();
        if has_preserves_abi {
            let response_bytes = hostcall_decision_bytes.clone();
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
                        "Wasm executor hostcall linker setup failed for actor {actor_id}: {error}"
                    ))
                })?;
        } else {
            linker
                .func_wrap("molten:hostcall", hostcall, move |mut caller: Caller<'_, WasmExecutionState>| {
                    caller.data_mut().hostcalls.push(captured_hostcall.clone());
                })
                .map_err(|error| {
                    MoltenError::invalid_harness(format!(
                        "Wasm executor hostcall linker setup failed for actor {actor_id}: {error}"
                    ))
                })?;
        }
    }

    let limits = StoreLimitsBuilder::new()
        .memory_size(WASM_MEMORY_LIMIT_BYTES)
        .table_elements(WASM_TABLE_ELEMENT_LIMIT)
        .instances(1)
        .memories(1)
        .tables(1)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(&engine, WasmExecutionState {
        hostcalls: Vec::new(),
        limits,
    });
    store.limiter(|state| &mut state.limits);
    store.set_fuel(WASM_FUEL_LIMIT).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor fuel setup failed for actor {actor_id}: {error}"))
    })?;

    let instance = linker.instantiate(&mut store, &module).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor instantiation failed for actor {actor_id}; only declared molten:hostcall imports are linked and WASI is unavailable: {error}"
        ))
    })?;

    let abi_receipt = if has_preserves_abi {
        Some(execute_preserves_abi(actor_id, &instance, &mut store, &export, actor_input)?)
    } else {
        let func = instance.get_typed_func::<(), ()>(&mut store, &export).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "Wasm executor actor {actor_id} missing required export {export} for hostcall operation {operation}: {error}"
            ))
        })?;
        func.call(&mut store, ()).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "Wasm executor actor {actor_id} export {export} trapped while requesting hostcall {operation}: {error}"
            ))
        })?;
        None
    };

    let fuel_remaining = store.get_fuel().map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor fuel readback failed for actor {actor_id}: {error}"))
    })?;
    let hostcalls = store.data().hostcalls.clone();
    if hostcalls != vec![operation.clone()] {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} requested hostcalls {:?}, expected exactly {:?} for step {sequence} ({step_ref})",
            hostcalls,
            vec![operation.clone()]
        )));
    }

    let module_ref = wasm_module_ref(config)?;
    Ok(Some(wasm_execution_receipt_value(WasmExecutionReceiptInput {
        actor_id,
        module_ref: &module_ref,
        export: &export,
        operation: &operation,
        hostcalls: &hostcalls,
        fuel_limit: WASM_FUEL_LIMIT,
        fuel_remaining,
        memory_limit_bytes: WASM_MEMORY_LIMIT_BYTES as u64,
        abi: abi_receipt,
    })))
}

fn execute_preserves_abi(
    actor_id: &str,
    instance: &wasmtime::Instance,
    store: &mut Store<WasmExecutionState>,
    export: &str,
    actor_input: &IOValue,
) -> Result<WasmAbiReceiptInput> {
    let memory = instance.get_memory(&mut *store, "memory").ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} uses molten.wasm.abi.v1 but does not export memory"
        ))
    })?;
    let alloc = instance.get_typed_func::<i32, i32>(&mut *store, "molten_alloc").map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} missing molten.wasm.abi.v1 allocator export molten_alloc: {error}"
        ))
    })?;
    let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut *store, "molten_dealloc").map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} missing molten.wasm.abi.v1 deallocator export molten_dealloc: {error}"
        ))
    })?;
    let func = instance
        .get_typed_func::<(i32, i32), i64>(&mut *store, export)
        .map_err(|error| {
            MoltenError::invalid_harness(format!(
                "Wasm executor actor {actor_id} export {export} must use molten.wasm.abi.v1 signature (i32,i32)->i64: {error}"
            ))
        })?;

    let input_bytes = canonical_bytes(actor_input)?;
    if input_bytes.len() > WASM_ABI_MAX_INPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} input bytes exceed molten.wasm.abi.v1 limit"
        )));
    }
    let input_len = i32::try_from(input_bytes.len()).map_err(|_| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} input length does not fit i32"))
    })?;
    let input_ptr = alloc.call(&mut *store, input_len).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} allocator trapped: {error}"))
    })?;
    write_memory_checked(actor_id, &memory, &mut *store, input_ptr, &input_bytes)?;

    let descriptor = func.call(&mut *store, (input_ptr, input_len)).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} export {export} trapped under molten.wasm.abi.v1: {error}"
        ))
    })?;
    dealloc.call(&mut *store, (input_ptr, input_len)).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} deallocator trapped: {error}"))
    })?;

    let (output_ptr, output_len) = decode_descriptor(actor_id, descriptor)?;
    if output_len > WASM_ABI_MAX_OUTPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} output bytes exceed molten.wasm.abi.v1 limit"
        )));
    }
    let output_bytes = read_memory_checked(actor_id, &memory, &mut *store, output_ptr, output_len)?;
    let output_value = parse_canonical_bytes(&output_bytes).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} returned invalid canonical Preserves output bytes: {error}"
        ))
    })?;
    let output_ref = canonical_hash(&output_value)?;
    let output_len_i32 = i32::try_from(output_len).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} output length cannot be passed to deallocator: {error}"
        ))
    })?;
    dealloc.call(&mut *store, (output_ptr, output_len_i32)).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} output deallocator trapped: {error}"))
    })?;
    let input_ref = canonical_hash(actor_input)?;
    Ok(WasmAbiReceiptInput {
        input_ref,
        output_ref,
        output_bytes: output_len as u64,
    })
}

fn validate_hostcall_preserves_bytes(
    caller: &mut Caller<'_, WasmExecutionState>,
    ptr: i32,
    len: i32,
) -> wasmtime::Result<()> {
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| format_err!("molten.wasm.abi.v1 hostcall missing exported memory"))?;
    let bytes = read_memory_raw(&memory, &mut *caller, ptr, len).map_err(|error| format_err!("{error}"))?;
    if bytes.len() > WASM_ABI_MAX_HOSTCALL_BYTES {
        return Err(format_err!("molten.wasm.abi.v1 hostcall request exceeds byte limit"));
    }
    parse_canonical_bytes(&bytes)
        .map_err(|error| format_err!("invalid canonical Preserves hostcall bytes: {error}"))?;
    Ok(())
}

fn write_hostcall_response_bytes(
    caller: &mut Caller<'_, WasmExecutionState>,
    response_bytes: &[u8],
) -> wasmtime::Result<()> {
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| format_err!("molten.wasm.abi.v1 hostcall missing exported memory"))?;
    memory
        .write(caller, WASM_ABI_HOSTCALL_RESPONSE_PTR, response_bytes)
        .map_err(|_| format_err!("molten.wasm.abi.v1 hostcall response pointer out of guest memory bounds"))
}

fn module_exports(bytes: &[u8]) -> Result<Vec<(String, ExternalKind)>> {
    let mut exports = Vec::with_capacity(WASM_MAX_EXPORTS);
    for payload in Parser::new(0).parse_all(bytes) {
        if let Payload::ExportSection(section) =
            payload.map_err(|error| MoltenError::invalid_harness(format!("wasm export parse failed: {error}")))?
        {
            for export in section {
                let export = export
                    .map_err(|error| MoltenError::invalid_harness(format!("wasm export parse failed: {error}")))?;
                if exports.len() >= WASM_MAX_EXPORTS {
                    return Err(MoltenError::invalid_harness(format!(
                        "wasm module declares more than {WASM_MAX_EXPORTS} exports"
                    )));
                }
                exports.push((export.name.to_string(), export.kind));
            }
        }
    }
    Ok(exports)
}

fn decode_descriptor(actor_id: &str, descriptor: i64) -> Result<(i32, usize)> {
    let raw = u64::from_ne_bytes(descriptor.to_ne_bytes());
    let ptr_u64 = raw >> 32;
    let len_u64 = raw & 0xffff_ffff;
    let ptr_u32 = u32::try_from(ptr_u64).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} ABI pointer out of range: {error}"))
    })?;
    let len_u32 = u32::try_from(len_u64).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} ABI length out of range: {error}"))
    })?;
    let ptr = i32::try_from(ptr_u32).map_err(|_| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} returned negative ABI pointer"))
    })?;
    let len = usize::try_from(len_u32).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} ABI length unsupported: {error}"))
    })?;
    Ok((ptr, len))
}

fn write_memory_checked(
    actor_id: &str,
    memory: &Memory,
    store: &mut Store<WasmExecutionState>,
    ptr: i32,
    bytes: &[u8],
) -> Result<()> {
    let ptr = usize::try_from(ptr).map_err(|_| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} returned negative ABI pointer"))
    })?;
    memory.write(store, ptr, bytes).map_err(|_| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} ABI pointer/length is out of guest memory bounds"
        ))
    })
}

fn read_memory_checked(
    actor_id: &str,
    memory: &Memory,
    store: &mut Store<WasmExecutionState>,
    ptr: i32,
    len: usize,
) -> Result<Vec<u8>> {
    let ptr = usize::try_from(ptr).map_err(|_| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} returned negative ABI pointer"))
    })?;
    let mut bytes = vec![0; len];
    memory.read(store, ptr, &mut bytes).map_err(|_| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} ABI pointer/length is out of guest memory bounds"
        ))
    })?;
    Ok(bytes)
}

fn read_memory_raw<T>(memory: &Memory, store: T, ptr: i32, len: i32) -> std::result::Result<Vec<u8>, String>
where T: wasmtime::AsContext {
    let ptr = usize::try_from(ptr).map_err(|_| "negative ABI pointer".to_string())?;
    let len = usize::try_from(len).map_err(|_| "negative ABI length".to_string())?;
    let mut bytes = vec![0; len];
    memory
        .read(store, ptr, &mut bytes)
        .map_err(|_| "ABI pointer/length is out of guest memory bounds".to_string())?;
    Ok(bytes)
}
