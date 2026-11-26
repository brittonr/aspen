
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
        denied_hostcall: None,
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

// r[impl molten.wasm.harness_nan_canonicalization_and_sync_call_limit]
/// Admit one hostcall dispatch inside the trampoline, before any effect.
///
/// A step admits exactly one call of its own operation. A second call, or a
/// call of another operation, is denied synchronously: nothing is recorded,
/// no response is written into guest memory, and the guest traps.
fn admit_single_hostcall(state: &mut WasmExecutionState, operation: &str, hostcall: &str) -> wasmtime::Result<()> {
    if !state.hostcalls.is_empty() || hostcall != operation {
        state.denied_hostcall = Some(hostcall.to_string());
        return Err(wasmtime::format_err!(
            "molten.wasm.abi.v1 single-call limit denied hostcall {hostcall} before dispatch"
        ));
    }
    state.hostcalls.push(hostcall.to_string());
    Ok(())
}
