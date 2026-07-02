
fn execute_preserves_abi(
    actor_id: &str,
    instance: &wasmtime::Instance,
    store: &mut Store<WasmExecutionState>,
    export: &str,
    actor_input: &PreservesValue,
) -> Result<AbiReceipt> {
    let parts = parts(actor_id, instance, store, export)?;
    let written = write_input(InputWrite {
        actor_id,
        memory: &parts.memory,
        store,
        alloc: &parts.alloc,
        actor_input,
    })?;
    let descriptor = parts.func.call(&mut *store, (written.ptr, written.len)).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {actor_id} export {export} trapped under molten.wasm.abi.v1: {error}"
        ))
    })?;
    parts.dealloc.call(&mut *store, (written.ptr, written.len)).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {actor_id} deallocator trapped: {error}"))
    })?;
    let output = read_output(OutputRead {
        actor_id,
        memory: &parts.memory,
        store,
        dealloc: &parts.dealloc,
        descriptor,
    })?;
    let input_ref = canonical_hash(actor_input)?;
    Ok(AbiReceipt {
        input_ref,
        output_ref: output.output_ref,
        output_bytes: output.bytes,
    })
}

struct Parts {
    memory: Memory,
    alloc: wasmtime::TypedFunc<i32, i32>,
    dealloc: wasmtime::TypedFunc<(i32, i32), ()>,
    func: wasmtime::TypedFunc<(i32, i32), i64>,
}

struct Written {
    ptr: i32,
    len: i32,
}

struct Output {
    output_ref: String,
    bytes: u64,
}

struct InputWrite<'a, 'b> {
    actor_id: &'a str,
    memory: &'a Memory,
    store: &'b mut Store<WasmExecutionState>,
    alloc: &'a wasmtime::TypedFunc<i32, i32>,
    actor_input: &'a PreservesValue,
}

struct OutputRead<'a, 'b> {
    actor_id: &'a str,
    memory: &'a Memory,
    store: &'b mut Store<WasmExecutionState>,
    dealloc: &'a wasmtime::TypedFunc<(i32, i32), ()>,
    descriptor: i64,
}

fn parts(
    actor_id: &str,
    instance: &wasmtime::Instance,
    store: &mut Store<WasmExecutionState>,
    export: &str,
) -> Result<Parts> {
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
    Ok(Parts {
        memory,
        alloc,
        dealloc,
        func,
    })
}

fn write_input(input: InputWrite<'_, '_>) -> Result<Written> {
    let input_bytes = canonical_bytes(input.actor_input)?;
    if input_bytes.len() > WASM_ABI_MAX_INPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {} input bytes exceed molten.wasm.abi.v1 limit",
            input.actor_id
        )));
    }
    let input_len = i32::try_from(input_bytes.len()).map_err(|_| {
        MoltenError::invalid_harness(format!("Wasm executor actor {} input length does not fit i32", input.actor_id))
    })?;
    let input_ptr = input.alloc.call(&mut *input.store, input_len).map_err(|error| {
        MoltenError::invalid_harness(format!("Wasm executor actor {} allocator trapped: {error}", input.actor_id))
    })?;
    write_memory_checked(input.actor_id, input.memory, &mut *input.store, input_ptr, &input_bytes)?;
    Ok(Written {
        ptr: input_ptr,
        len: input_len,
    })
}

fn read_output(input: OutputRead<'_, '_>) -> Result<Output> {
    let (output_ptr, output_len) = decode_descriptor(input.actor_id, input.descriptor)?;
    if output_len > WASM_ABI_MAX_OUTPUT_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "Wasm executor actor {} output bytes exceed molten.wasm.abi.v1 limit",
            input.actor_id
        )));
    }
    let output_bytes = read_memory_checked(input.actor_id, input.memory, &mut *input.store, output_ptr, output_len)?;
    let output_value = parse_canonical_bytes(&output_bytes).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {} returned invalid canonical Preserves output bytes: {error}",
            input.actor_id
        ))
    })?;
    let output_ref = canonical_hash(&output_value)?;
    let output_len_i32 = i32::try_from(output_len).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {} output length cannot be passed to deallocator: {error}",
            input.actor_id
        ))
    })?;
    input.dealloc.call(&mut *input.store, (output_ptr, output_len_i32)).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "Wasm executor actor {} output deallocator trapped: {error}",
            input.actor_id
        ))
    })?;
    Ok(Output {
        output_ref,
        bytes: output_len as u64,
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
        .ok_or_else(|| wasmtime::format_err!("molten.wasm.abi.v1 hostcall missing exported memory"))?;
    let bytes = read_memory_raw(&memory, &mut *caller, ptr, len).map_err(|error| wasmtime::format_err!("{error}"))?;
    if bytes.len() > WASM_ABI_MAX_HOSTCALL_BYTES {
        return Err(wasmtime::format_err!("molten.wasm.abi.v1 hostcall request exceeds byte limit"));
    }
    parse_canonical_bytes(&bytes)
        .map_err(|error| wasmtime::format_err!("invalid canonical Preserves hostcall bytes: {error}"))?;
    Ok(())
}

fn write_hostcall_response_bytes(
    caller: &mut Caller<'_, WasmExecutionState>,
    response_bytes: &[u8],
) -> wasmtime::Result<()> {
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or_else(|| wasmtime::format_err!("molten.wasm.abi.v1 hostcall missing exported memory"))?;
    memory
        .write(caller, WASM_ABI_HOSTCALL_RESPONSE_PTR, response_bytes)
        .map_err(|_| wasmtime::format_err!("molten.wasm.abi.v1 hostcall response pointer out of guest memory bounds"))
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
