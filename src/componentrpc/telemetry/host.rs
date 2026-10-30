//! Real Wasmtime host callbacks for the selected upstream WASI OTel tracing interface.
use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;
use wasmtime_rpc::component;

struct State {
    receipt: String,
    active: bool,
    spans: Vec<preserves::IOValue>,
    limits: wasmtime_rpc::StoreLimits,
}
pub struct ExportInput<'a> {
    pub bundle: &'a super::Bundle,
    pub lease: super::Lease,
    pub sink: &'a cap_std::fs::Dir,
    pub sink_ref: &'a str,
    pub mode: u32,
    pub tick: u64,
    pub generation: u64,
}

pub async fn export(input: ExportInput<'_>) -> super::Result<preserves::IOValue> {
    input.lease.check(input.tick, input.generation)?;
    input.bundle.check(input.lease.scope())?;
    if input.sink_ref != input.lease.scope().sink || input.mode > 2 {
        return Err("telemetry sink/mode not admitted".into());
    }
    let engine = super::super::runtime::engine()?;
    let component = component::Component::new(&engine, input.bundle.component()).map_err(super::super::failure)?;
    let mut linker = component::Linker::<State>::new(&engine);
    admit_imports(&component, &mut linker)?;
    let mut interface = linker.instance(super::INTERFACE).map_err(super::super::failure)?;
    interface
        .func_new("current-span-context", |store, args, results| {
            anyhow::ensure!(args.is_empty() && results.len() == 1, "wrong current-span-context type");
            results[0] = super::projection::context(&store.data().receipt);
            Ok(())
        })
        .map_err(super::super::failure)?;
    interface
        .func_new("on-start", |mut store, args, results| {
            anyhow::ensure!(args.len() == 1 && results.is_empty(), "wrong on-start type");
            let state = store.data_mut();
            anyhow::ensure!(
                !state.active && state.spans.len() < super::MAX_SPANS,
                "span lifecycle/queue budget denied"
            );
            anyhow::ensure!(args[0] == super::projection::context(&state.receipt), "untrusted span context");
            state.active = true;
            Ok(())
        })
        .map_err(super::super::failure)?;
    interface
        .func_new("on-end", |mut store, args, results| {
            anyhow::ensure!(args.len() == 1 && results.is_empty(), "wrong on-end type");
            let state = store.data_mut();
            anyhow::ensure!(state.active && state.spans.len() < super::MAX_SPANS, "span lifecycle/queue budget denied");
            let value = super::projection::span(&args[0], &state.receipt).map_err(anyhow::Error::msg)?;
            state.spans.push(value);
            state.active = false;
            Ok(())
        })
        .map_err(super::super::failure)?;
    let state = State {
        receipt: input.lease.scope().receipt.clone(),
        active: false,
        spans: Vec::new(),
        limits: wasmtime_rpc::StoreLimitsBuilder::new()
            .memory_size(super::super::profile::MEMORY_BYTES)
            .instances(64)
            .memories(16)
            .tables(16)
            .table_elements(65_536)
            .build(),
    };
    let mut store = wasmtime_rpc::Store::new(&engine, state);
    store.limiter(|state| &mut state.limits);
    store.set_fuel(super::super::profile::FUEL).map_err(super::super::failure)?;
    let instance = linker.instantiate_async(&mut store, &component).await.map_err(super::super::failure)?;
    let run = instance.get_typed_func::<(u32,), (u32,)>(&mut store, "run").map_err(super::super::failure)?;
    let (result,) = run.call_async(&mut store, (input.mode,)).await.map_err(|error| format!("{error:#}"))?;
    run.post_return_async(&mut store).await.map_err(super::super::failure)?;
    let state = store.into_data();
    if result != 42 || state.active || state.spans.is_empty() {
        return Err("telemetry emitter lifecycle/result differs".into());
    }
    let diagnostic = crate::codec::record("component-rpc-otel-diagnostic-export-v1", vec![
        crate::codec::string("diagnostic-only-not-canonical-evidence"),
        input.lease.scope().value(),
        crate::codec::string(input.lease.authority()),
        crate::codec::string(state.spans.len().to_string()),
        crate::codec::record("spans", state.spans),
    ]);
    persist(input.sink, &diagnostic)?;
    Ok(diagnostic)
}

fn admit_imports(component: &component::Component, linker: &mut component::Linker<State>) -> super::Result<()> {
    let mut has_tracing = false;
    for (name, item) in component.component_type().imports(component.engine()) {
        let component::types::ComponentItem::ComponentInstance(interface) = item else {
            return Err("telemetry import is not an interface".into());
        };
        if name == super::INTERFACE {
            let functions: Vec<_> = interface.exports(component.engine()).collect();
            if functions.len() != 3 {
                return Err("OTel tracing function set differs".into());
            }
            for (name, function) in functions {
                let component::types::ComponentItem::ComponentFunc(function) = function else {
                    return Err("OTel effect is not a function".into());
                };
                let expected = match name {
                    "on-start" | "on-end" => (1, 0),
                    "current-span-context" => (0, 1),
                    _ => return Err("unapproved OTel tracing function".into()),
                };
                if (function.params().len(), function.results().len()) != expected {
                    return Err("OTel tracing signature differs".into());
                }
            }
            has_tracing = true;
        } else if ["wasi:otel/types@0.2.0-rc.2", "wasi:clocks/wall-clock@0.2.0"].contains(&name)
            && interface
                .exports(component.engine())
                .all(|(_, item)| matches!(item, component::types::ComponentItem::Type(_)))
        {
            linker.instance(name).map_err(super::super::failure)?;
        } else {
            return Err("OTel emitter requests an unadmitted effect import".into());
        }
    }
    if !has_tracing {
        return Err("telemetry emitter lacks the actual selected tracing import".into());
    }
    Ok(())
}
fn persist(root: &cap_std::fs::Dir, value: &preserves::IOValue) -> super::Result<()> {
    let bytes = crate::codec::canonical_bytes(value).map_err(super::super::failure)?;
    if bytes.len() > 65_536 {
        return Err("diagnostic export exceeds total byte budget".into());
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = root.open_with("otel-diagnostic.preserves", &options).map_err(super::super::failure)?;
    file.write_all(&bytes).and_then(|()| file.sync_all()).map_err(super::super::failure)?;
    root.open(".").and_then(|directory| directory.sync_all()).map_err(super::super::failure)
}
