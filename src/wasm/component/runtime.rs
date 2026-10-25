mod bindings;
mod denial;
mod shell;
mod sliced;
mod slicedshell;

pub use shell::ExecutionOutcome;
pub use shell::ExecutionRequest;
pub use shell::execute;
pub use slicedshell::SliceProgress;
pub use slicedshell::SlicedExecution;
pub use slicedshell::start_sliced_execution;

type Linker<T> = wasmtime::component::Linker<T>;
type Compiled = wasmtime::component::Component;
type Engine = wasmtime::Engine;
type Store<T> = wasmtime::Store<T>;
type StoreLimits = wasmtime::StoreLimits;
type StoreLimitsBuilder = wasmtime::StoreLimitsBuilder;

#[derive(Debug)]
struct StoreState {
    limits: StoreLimits,
}

pub(crate) struct Session {
    store: Store<StoreState>,
    bindings: bindings::Actor,
}

pub(crate) struct Execution {
    pub output_bytes: Vec<u8>,
    pub fuel_remaining: u64,
}

pub(crate) fn instantiate(
    profile: &super::model::RuntimeProfile,
    component_bytes: &[u8],
    facts: &super::admission::ArtifactFacts,
) -> super::model::Result<Session> {
    let engine = engine(profile)?;
    let component = Compiled::from_binary(&engine, component_bytes).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ComponentCompilationDenied,
            format!("component compilation failed: {error}"),
        )
    })?;
    verify_shape(&engine, &component, facts)?;
    if !facts.imports.is_empty() {
        return Err(super::model::Denial::new("initial component runtime cohort has no admitted host linker bindings"));
    }
    let linker = Linker::new(&engine);
    let mut store = store(&engine, profile)?;
    let bindings = bindings::Actor::instantiate(&mut store, &component, &linker).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ComponentInstantiationDenied,
            format!("component world instantiation failed: {error}"),
        )
    })?;
    Ok(Session { store, bindings })
}

pub(crate) fn invoke(session: &mut Session, input_bytes: &[u8]) -> super::model::Result<Execution> {
    let guest_input = input_bytes.to_vec();
    let output_bytes = session
        .bindings
        .call_invoke(&mut session.store, &guest_input)
        .map_err(|error| invocation_denial(&session.store, error))?
        .map_err(|error| {
            super::model::Denial::classified(
                super::model::DenialClass::GuestDenial,
                format!("component invoke denied: {error}"),
            )
        })?;
    let fuel_remaining = session.store.get_fuel().map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ResourceDenial,
            format!("component fuel observation failed: {error}"),
        )
    })?;
    Ok(Execution {
        output_bytes,
        fuel_remaining,
    })
}

fn invocation_denial(store: &Store<StoreState>, error: wasmtime::Error) -> super::model::Denial {
    match store.get_fuel() {
        Ok(0) => super::model::Denial::classified(
            super::model::DenialClass::FuelExhausted,
            format!("component fuel exhausted during invoke: {error}"),
        ),
        Ok(_) => super::model::Denial::classified(
            super::model::DenialClass::ComponentTrap,
            format!("component invoke trapped: {error}"),
        ),
        Err(fuel_error) => super::model::Denial::classified(
            super::model::DenialClass::ComponentTrap,
            format!("component invoke trapped and fuel observation failed ({fuel_error}): {error}"),
        ),
    }
}

#[cfg(test)]
pub(super) fn test_precompile(
    profile: &super::model::RuntimeProfile,
    component_bytes: &[u8],
) -> super::model::Result<Vec<u8>> {
    engine(profile)?.precompile_component(component_bytes).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ComponentCompilationDenied,
            format!("component test precompilation failed: {error}"),
        )
    })
}

fn engine(profile: &super::model::RuntimeProfile) -> super::model::Result<Engine> {
    let stack_size = usize::try_from(profile.resources.max_stack_bytes).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ResourceDenial,
            format!("component stack bound is unsupported: {error}"),
        )
    })?;
    let mut config = wasmtime::Config::new();
    config
        .wasm_component_model(true)
        .wasm_bulk_memory(true)
        .wasm_multi_value(true)
        .wasm_reference_types(true)
        .wasm_simd(true)
        .consume_fuel(true)
        .cranelift_nan_canonicalization(true)
        .wasm_relaxed_simd(false)
        .relaxed_simd_deterministic(true)
        .wasm_threads(false)
        .wasm_tail_call(false)
        .wasm_multi_memory(false)
        .wasm_exceptions(false)
        .wasm_memory64(false)
        .wasm_extended_const(false)
        .wasm_function_references(false)
        .wasm_gc(false)
        .wasm_custom_page_sizes(false)
        .wasm_wide_arithmetic(false)
        .max_wasm_stack(stack_size);
    Engine::new(&config).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ProfileDenial,
            format!("component engine configuration failed: {error}"),
        )
    })
}

fn store(engine: &Engine, profile: &super::model::RuntimeProfile) -> super::model::Result<Store<StoreState>> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(to_usize("memory", profile.resources.max_memory_bytes)?)
        .table_elements(to_usize("table", profile.resources.max_table_elements)?)
        .instances(to_usize("instances", profile.resources.max_instances)?)
        .memories(to_usize("memories", profile.resources.max_memories)?)
        .tables(to_usize("tables", profile.resources.max_tables)?)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(engine, StoreState { limits });
    store.limiter(|state| &mut state.limits);
    store.set_fuel(profile.resources.fuel).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ResourceDenial,
            format!("component fuel setup failed: {error}"),
        )
    })?;
    Ok(store)
}

fn verify_shape(
    engine: &Engine,
    component: &Compiled,
    facts: &super::admission::ArtifactFacts,
) -> super::model::Result<()> {
    let import_limit = facts.imports.len().saturating_add(1);
    let mut imports = component
        .component_type()
        .imports(engine)
        .take(import_limit)
        .map(|(name, _item)| name.to_string())
        .collect::<Vec<_>>();
    imports.sort();
    let export_limit = facts.exports.len().saturating_add(1);
    let mut exports = component
        .component_type()
        .exports(engine)
        .take(export_limit)
        .map(|(name, _item)| name.to_string())
        .collect::<Vec<_>>();
    exports.sort();
    if imports != facts.imports {
        return Err(super::model::Denial::new("runtime component imports differ from materialization facts"));
    }
    if exports != facts.exports {
        return Err(super::model::Denial::new("runtime component exports differ from materialization facts"));
    }
    Ok(())
}

fn to_usize(label: &str, value: u64) -> super::model::Result<usize> {
    usize::try_from(value).map_err(|error| {
        super::model::Denial::classified(
            super::model::DenialClass::ResourceDenial,
            format!("component {label} bound is unsupported: {error}"),
        )
    })
}
