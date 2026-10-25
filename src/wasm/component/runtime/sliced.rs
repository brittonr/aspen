// The time-sliced invocation path.
//
// Under the admitted time-sliced profile the store suspends the guest every
// quantum of fuel through Wasmtime's fuel-based asynchronous yielding. The
// guest export stays a synchronous WIT export: the host drives it through the
// asynchronous entrypoint and polls the invocation one slice at a time, so
// the admitted scheduler decides what runs between slices. A resumed slice
// continues from the suspension point; nothing reruns from the start.

mod async_bindings {
    wasmtime::component::bindgen!({
        path: "wit/molten-component-runtime",
        world: "actor",
        exports: { default: async },
    });
}

type InvokeOutput = (super::Store<super::StoreState>, wasmtime::Result<Result<Vec<u8>, String>>);
type InvokeFuture = std::pin::Pin<Box<dyn Future<Output = InvokeOutput> + Send>>;

/// One invocation in progress under the time-sliced profile.
pub(super) struct Invocation {
    future: Option<InvokeFuture>,
    ledger: super::super::slicing::SliceLedger,
}

/// What one slice of an invocation did.
pub(super) enum SliceStep {
    /// The guest consumed a quantum and yielded; the boundary is recorded.
    Yielded(super::super::slicing::SliceBoundary),
    /// The guest returned or stopped; the result is final.
    Completed(super::super::model::Result<super::Execution>),
}

fn slicing_denial(detail: impl Into<String>) -> super::super::model::Denial {
    super::super::model::Denial::classified(super::super::model::DenialClass::ResourceDenial, detail)
}

/// Compile, verify, and instantiate a component under the time-sliced
/// profile, and prepare its invocation without running any guest code.
// r[impl molten.wasm_component.execution_quantum]
pub(super) fn instantiate(
    profile: &super::super::model::RuntimeProfile,
    component_bytes: &[u8],
    facts: &super::super::admission::ArtifactFacts,
    input_bytes: &[u8],
) -> super::super::model::Result<Invocation> {
    let slicing = profile.slicing.as_ref().ok_or_else(|| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::ProfileDenial,
            "sliced invocation needs an admitted slicing profile",
        )
    })?;
    let ledger = super::super::slicing::SliceLedger::new(slicing.quantum_fuel, slicing.total_budget_fuel)
        .map_err(|issue| slicing_denial(format!("slice ledger denied: {issue:?}")))?;
    let engine = super::engine(profile)?;
    let component = super::Compiled::from_binary(&engine, component_bytes).map_err(|error| {
        super::super::model::Denial::classified(
            super::super::model::DenialClass::ComponentCompilationDenied,
            format!("component compilation failed: {error}"),
        )
    })?;
    super::verify_shape(&engine, &component, facts)?;
    if !facts.imports.is_empty() {
        return Err(super::super::model::Denial::new(
            "initial component runtime cohort has no admitted host linker bindings",
        ));
    }
    let linker = super::Linker::new(&engine);
    let mut store = super::store(&engine, profile)?;
    store
        .set_fuel(slicing.total_budget_fuel)
        .map_err(|error| slicing_denial(format!("slice budget setup failed: {error}")))?;
    store
        .fuel_async_yield_interval(Some(slicing.quantum_fuel))
        .map_err(|error| slicing_denial(format!("slice quantum setup failed: {error}")))?;
    let bindings = instantiate_to_completion(&mut store, &component, &linker, &ledger)?;
    let input = input_bytes.to_vec();
    let future: InvokeFuture = Box::pin(async move {
        let mut store = store;
        let result = bindings.call_invoke(&mut store, &input).await;
        (store, result)
    });
    Ok(Invocation {
        future: Some(future),
        ledger,
    })
}

/// Instantiation runs no guest work for an import-free component, so it
/// completes within the bounded number of polls the budget allows.
fn instantiate_to_completion(
    store: &mut super::Store<super::StoreState>,
    component: &super::Compiled,
    linker: &super::Linker<super::StoreState>,
    ledger: &super::super::slicing::SliceLedger,
) -> super::super::model::Result<async_bindings::Actor> {
    let polls = ledger
        .total_budget_fuel
        .checked_div(ledger.quantum_fuel)
        .and_then(|slices| slices.checked_add(1))
        .ok_or_else(|| slicing_denial("the slice budget admits no instantiation polls"))?;
    let mut instantiation = std::pin::pin!(async_bindings::Actor::instantiate_async(store, component, linker));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    for _ in 0..polls {
        if let std::task::Poll::Ready(result) = instantiation.as_mut().poll(&mut context) {
            return result.map_err(|error| {
                super::super::model::Denial::classified(
                    super::super::model::DenialClass::ComponentInstantiationDenied,
                    format!("component world instantiation failed: {error}"),
                )
            });
        }
    }
    Err(slicing_denial("component instantiation did not complete within the slice budget"))
}

impl Invocation {
    pub(super) const fn ledger(&self) -> &super::super::slicing::SliceLedger {
        &self.ledger
    }

    /// Run one slice: poll the invocation once.
    ///
    /// A pending poll means the guest consumed one quantum and yielded. A
    /// ready poll ends the invocation and records the final partial slice so
    /// the ledger's consumption equals the fuel the store actually spent.
    // r[impl molten.wasm_component.yield_continuation]
    pub(super) fn run_slice(&mut self) -> super::super::model::Result<SliceStep> {
        let Some(future) = self.future.as_mut() else {
            return Err(super::super::model::Denial::new("the sliced invocation already completed"));
        };
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Pending => {
                let boundary = self
                    .ledger
                    .record_boundary(self.ledger.quantum_fuel)
                    .map_err(|issue| slicing_denial(format!("slice boundary denied: {issue:?}")))?;
                Ok(SliceStep::Yielded(boundary))
            }
            std::task::Poll::Ready((store, result)) => {
                self.future = None;
                Ok(SliceStep::Completed(self.complete(&store, result)))
            }
        }
    }

    fn complete(
        &mut self,
        store: &super::Store<super::StoreState>,
        result: wasmtime::Result<Result<Vec<u8>, String>>,
    ) -> super::super::model::Result<super::Execution> {
        let output_bytes = result.map_err(|error| super::invocation_denial(store, error))?.map_err(|error| {
            super::super::model::Denial::classified(
                super::super::model::DenialClass::GuestDenial,
                format!("component invoke denied: {error}"),
            )
        })?;
        let fuel_remaining = store
            .get_fuel()
            .map_err(|error| slicing_denial(format!("component fuel observation failed: {error}")))?;
        let final_slice = self
            .ledger
            .total_budget_fuel
            .checked_sub(fuel_remaining)
            .and_then(|consumed| consumed.checked_sub(self.ledger.consumed_fuel))
            .ok_or_else(|| slicing_denial("the store spent less fuel than the recorded slices"))?;
        self.ledger
            .record_boundary(final_slice)
            .map_err(|issue| slicing_denial(format!("final slice denied: {issue:?}")))?;
        Ok(super::Execution {
            output_bytes,
            fuel_remaining,
        })
    }
}
