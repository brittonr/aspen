mod bindings;
mod denial;
mod shell;

pub use shell::ComponentExecutionOutcome;
pub use shell::ComponentExecutionRequest;
pub use shell::execute_component;

type ComponentLinker<T> = wasmtime::component::Linker<T>;
type RuntimeComponent = wasmtime::component::Component;
type RuntimeEngine = wasmtime::Engine;
type RuntimeStore<T> = wasmtime::Store<T>;
type RuntimeStoreLimits = wasmtime::StoreLimits;
type RuntimeStoreLimitsBuilder = wasmtime::StoreLimitsBuilder;

#[derive(Debug)]
struct ComponentStoreState {
    limits: RuntimeStoreLimits,
}

pub(crate) struct ComponentSession {
    store: RuntimeStore<ComponentStoreState>,
    bindings: bindings::Actor,
}

pub(crate) struct RuntimeExecution {
    pub output_bytes: Vec<u8>,
    pub fuel_remaining: u64,
}

pub(crate) fn instantiate_component(
    profile: &super::model::ComponentRuntimeProfile,
    component_bytes: &[u8],
    facts: &super::admission::ComponentArtifactFacts,
) -> super::model::ComponentResult<ComponentSession> {
    let engine = component_engine(profile)?;
    let component = RuntimeComponent::from_binary(&engine, component_bytes).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ComponentCompilationDenied,
            format!("component compilation failed: {error}"),
        )
    })?;
    verify_runtime_shape(&engine, &component, facts)?;
    if !facts.imports.is_empty() {
        return Err(super::model::ComponentDenial::new(
            "initial component runtime cohort has no admitted host linker bindings",
        ));
    }
    let linker = ComponentLinker::new(&engine);
    let mut store = component_store(&engine, profile)?;
    let bindings = bindings::Actor::instantiate(&mut store, &component, &linker).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ComponentInstantiationDenied,
            format!("component world instantiation failed: {error}"),
        )
    })?;
    Ok(ComponentSession { store, bindings })
}

pub(crate) fn invoke_component(
    session: &mut ComponentSession,
    input_bytes: &[u8],
) -> super::model::ComponentResult<RuntimeExecution> {
    let guest_input = input_bytes.to_vec();
    let output_bytes = session
        .bindings
        .call_invoke(&mut session.store, &guest_input)
        .map_err(|error| invocation_denial(&session.store, error))?
        .map_err(|error| {
            super::model::ComponentDenial::classified(
                super::model::ComponentDenialClass::GuestDenial,
                format!("component invoke denied: {error}"),
            )
        })?;
    let fuel_remaining = session.store.get_fuel().map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ResourceDenial,
            format!("component fuel observation failed: {error}"),
        )
    })?;
    Ok(RuntimeExecution {
        output_bytes,
        fuel_remaining,
    })
}

fn invocation_denial(
    store: &RuntimeStore<ComponentStoreState>,
    error: wasmtime::Error,
) -> super::model::ComponentDenial {
    match store.get_fuel() {
        Ok(0) => super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::FuelExhausted,
            format!("component fuel exhausted during invoke: {error}"),
        ),
        Ok(_) => super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ComponentTrap,
            format!("component invoke trapped: {error}"),
        ),
        Err(fuel_error) => super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ComponentTrap,
            format!("component invoke trapped and fuel observation failed ({fuel_error}): {error}"),
        ),
    }
}

#[cfg(test)]
pub(super) fn test_precompile_component(
    profile: &super::model::ComponentRuntimeProfile,
    component_bytes: &[u8],
) -> super::model::ComponentResult<Vec<u8>> {
    component_engine(profile)?.precompile_component(component_bytes).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ComponentCompilationDenied,
            format!("component test precompilation failed: {error}"),
        )
    })
}

fn component_engine(profile: &super::model::ComponentRuntimeProfile) -> super::model::ComponentResult<RuntimeEngine> {
    let stack_size = usize::try_from(profile.resources.max_stack_bytes).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ResourceDenial,
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
    RuntimeEngine::new(&config).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ProfileDenial,
            format!("component engine configuration failed: {error}"),
        )
    })
}

fn component_store(
    engine: &RuntimeEngine,
    profile: &super::model::ComponentRuntimeProfile,
) -> super::model::ComponentResult<RuntimeStore<ComponentStoreState>> {
    let limits = RuntimeStoreLimitsBuilder::new()
        .memory_size(to_usize("memory", profile.resources.max_memory_bytes)?)
        .table_elements(to_usize("table", profile.resources.max_table_elements)?)
        .instances(to_usize("instances", profile.resources.max_instances)?)
        .memories(to_usize("memories", profile.resources.max_memories)?)
        .tables(to_usize("tables", profile.resources.max_tables)?)
        .trap_on_grow_failure(true)
        .build();
    let mut store = RuntimeStore::new(engine, ComponentStoreState { limits });
    store.limiter(|state| &mut state.limits);
    store.set_fuel(profile.resources.fuel).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ResourceDenial,
            format!("component fuel setup failed: {error}"),
        )
    })?;
    Ok(store)
}

fn verify_runtime_shape(
    engine: &RuntimeEngine,
    component: &RuntimeComponent,
    facts: &super::admission::ComponentArtifactFacts,
) -> super::model::ComponentResult<()> {
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
        return Err(super::model::ComponentDenial::new("runtime component imports differ from materialization facts"));
    }
    if exports != facts.exports {
        return Err(super::model::ComponentDenial::new("runtime component exports differ from materialization facts"));
    }
    Ok(())
}

fn to_usize(label: &str, value: u64) -> super::model::ComponentResult<usize> {
    usize::try_from(value).map_err(|error| {
        super::model::ComponentDenial::classified(
            super::model::ComponentDenialClass::ResourceDenial,
            format!("component {label} bound is unsupported: {error}"),
        )
    })
}
