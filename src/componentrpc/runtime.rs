//! Isolated Wasmtime 33 / wRPC 0.15 host; never consumes shared Wasmtime 45 handles.
use std::collections;

use wasmtime_rpc::component;
use wasmtime_wasi_rpc::p2::IoView;
use wasmtime_wasi_rpc::p2::WasiView;
use wrpc_runtime_wasmtime::WrpcView;

pub struct Context<T> {
    transport: T,
    table: component::ResourceTable,
    wasi: wasmtime_wasi_rpc::p2::WasiCtx,
    shared: wrpc_runtime_wasmtime::SharedResourceTable,
    limits: wasmtime_rpc::StoreLimits,
}
impl<T: Send> IoView for Context<T> {
    fn table(&mut self) -> &mut component::ResourceTable {
        &mut self.table
    }
}
impl<T: Send> WasiView for Context<T> {
    fn ctx(&mut self) -> &mut wasmtime_wasi_rpc::p2::WasiCtx {
        &mut self.wasi
    }
}
impl<T: wrpc_transport::Invoke<Context = ()>> WrpcView for Context<T> {
    type Invoke = T;
    fn context(&self) {}
    fn client(&self) -> &T {
        &self.transport
    }
    fn shared_resources(&mut self) -> &mut wrpc_runtime_wasmtime::SharedResourceTable {
        &mut self.shared
    }
    fn timeout(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(10))
    }
}

pub fn engine() -> super::Result<wasmtime_rpc::Engine> {
    let mut config = wasmtime_rpc::Config::new();
    config.async_support(true).wasm_component_model(true).consume_fuel(true);
    config
        .cranelift_nan_canonicalization(true)
        .wasm_simd(false)
        .wasm_relaxed_simd(false)
        .wasm_tail_call(false)
        .wasm_multi_memory(false)
        .wasm_memory64(false)
        .max_wasm_stack(1_048_576);
    wasmtime_rpc::Engine::new(&config).map_err(super::failure)
}
fn store<T: Send>(engine: &wasmtime_rpc::Engine, transport: T) -> super::Result<wasmtime_rpc::Store<Context<T>>> {
    let context = Context {
        transport,
        table: component::ResourceTable::new(),
        wasi: wasmtime_wasi_rpc::p2::WasiCtxBuilder::new().build(),
        shared: wrpc_runtime_wasmtime::SharedResourceTable::default(),
        limits: wasmtime_rpc::StoreLimitsBuilder::new()
            .memory_size(super::profile::MEMORY_BYTES)
            .instances(64)
            .tables(16)
            .memories(16)
            .table_elements(65_536)
            .build(),
    };
    let mut store = wasmtime_rpc::Store::new(engine, context);
    store.limiter(|context| &mut context.limits);
    store.set_fuel(super::profile::FUEL).map_err(super::failure)?;
    Ok(store)
}

// r[impl molten.component_rpc.transport]
pub async fn run_client<T: wrpc_transport::Invoke<Context = ()> + 'static>(
    bundle: &super::materialization::Bundle,
    transport: T,
    scope: &super::bridge::Scope,
) -> super::Result<u32> {
    let engine = engine()?;
    let component = component::Component::new(&engine, bundle.client()).map_err(super::failure)?;
    bundle.check(scope)?;
    let mut linker = component::Linker::<Context<T>>::new(&engine);
    let component_type = component.component_type();
    let imports: Vec<_> = component_type.imports(&engine).collect();
    if imports.len() != 1 || imports[0].0 != super::INTERFACE {
        return Err("client remaining imports differ from unary profile".into());
    }
    let component::types::ComponentItem::ComponentInstance(instance) = &imports[0].1 else {
        return Err("hello import is not an instance".into());
    };
    let exports: Vec<_> = instance.exports(&engine).collect();
    if exports.len() != 1 || exports[0].0 != super::FUNCTION {
        return Err("wrong imported function".into());
    }
    let component::types::ComponentItem::ComponentFunc(function) = &exports[0].1 else {
        return Err("wrong hello type".into());
    };
    check_function(function)?;
    wrpc_runtime_wasmtime::link_item(
        &engine,
        &mut linker.root(),
        [],
        collections::HashMap::new(),
        imports[0].1.clone(),
        "",
        super::INTERFACE,
    )
    .map_err(super::failure)?;
    let mut store = store(&engine, transport)?;
    let instance = linker.instantiate_async(&mut store, &component).await.map_err(super::failure)?;
    let run = instance.get_typed_func::<(), (u32,)>(&mut store, "run").map_err(super::failure)?;
    let (result,) = run.call_async(&mut store, ()).await.map_err(super::failure)?;
    run.post_return_async(&mut store).await.map_err(super::failure)?;
    if result != 42 {
        return Err("materialized client rejected the remote result".into());
    }
    Ok(result)
}

fn check_function(function: &wasmtime_rpc::component::types::ComponentFunc) -> super::Result<()> {
    let results: Vec<_> = function.results().collect();
    if function.params().len() != 0 || results != [component::Type::String] {
        return Err("hello type differs; futures/streams/resources denied".into());
    }
    Ok(())
}

pub async fn call_server<T: wrpc_transport::Invoke<Context = ()> + 'static>(
    bundle: &super::materialization::Bundle,
    transport: T,
    lease: &super::authority::Lease,
) -> super::Result<Vec<u8>> {
    let engine = engine()?;
    let component = component::Component::new(&engine, bundle.server()).map_err(super::failure)?;
    bundle.check(lease.scope())?;
    if component.component_type().imports(&engine).next().is_some() {
        return Err("server must be deny-all WASI virtualized with no remaining imports".into());
    }
    let ty = component
        .component_type()
        .exports(&engine)
        .find(|(name, _)| *name == super::INTERFACE)
        .ok_or("server missing pinned interface")?
        .1;
    let component::types::ComponentItem::ComponentInstance(interface) = ty else {
        return Err("wrong server interface type".into());
    };
    let component::types::ComponentItem::ComponentFunc(function) = interface
        .exports(&engine)
        .find(|(name, _)| *name == super::FUNCTION)
        .ok_or("server missing hello")?
        .1
    else {
        return Err("wrong server function type".into());
    };
    check_function(&function)?;
    let results: Vec<_> = function.results().collect();
    let mut store = store(&engine, transport)?;
    let instance = component::Linker::new(&engine)
        .instantiate_async(&mut store, &component)
        .await
        .map_err(super::failure)?;
    let interface = instance.get_export_index(&mut store, None, super::INTERFACE).ok_or("missing interface index")?;
    let index = instance
        .get_export_index(&mut store, Some(&interface), super::FUNCTION)
        .ok_or("missing hello index")?;
    let function = instance.get_func(&mut store, index).ok_or("missing hello function")?;
    let writer = super::channels::Writer::default();
    wrpc_runtime_wasmtime::call(
        &mut store,
        super::channels::Reader(std::io::Cursor::new(Vec::new())),
        writer.clone(),
        &[],
        &collections::HashMap::new(),
        [].iter(),
        &results,
        function,
    )
    .await
    .map_err(super::failure)?;
    let bytes = writer.bytes()?;
    super::bridge::decode_result(&bytes)?;
    Ok(bytes)
}
