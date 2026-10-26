// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use async_trait::async_trait;
use molten_batch_application::*;
use molten_batch_core::*;
use wasmtime::component::Component;
use wasmtime::component::Linker;
use wasmtime::Config;
use wasmtime::Engine;
use wasmtime::Store;
use wasmtime::StoreLimits;
use wasmtime::StoreLimitsBuilder;

struct Slot {
    cancel: Arc<AtomicBool>,
    started: bool,
}
#[derive(Default)]
pub struct PureComponentRuntime {
    slots: Mutex<BTreeMap<Id, Slot>>,
    database_guard: Option<Arc<crate::Disk>>,
}
impl PureComponentRuntime {
    pub fn for_worker(disk: Arc<crate::Disk>) -> Self {
        Self {
            slots: Mutex::new(BTreeMap::new()),
            database_guard: Some(disk),
        }
    }
    fn claim(&self, id: Id) -> Result<Arc<AtomicBool>, Error> {
        let mut slots = self.slots.lock().map_err(|_| Error::Unavailable)?;
        if !slots.contains_key(&id) && slots.len() >= MAX_WORKER_RECORDS {
            return Err(Error::Bound);
        }
        let slot = slots.entry(id).or_insert_with(|| Slot {
            cancel: Arc::new(AtomicBool::new(false)),
            started: false,
        });
        if slot.started {
            return Err(Error::Conflict);
        }
        slot.started = true;
        Ok(slot.cancel.clone())
    }
}
#[async_trait]
impl ExecutionPort for PureComponentRuntime {
    async fn run(
        &self,
        r: ExecutionRequest,
        component: Content,
        inputs: Vec<Vec<u8>>,
    ) -> Result<ExecutionOutput, Error> {
        validate_request(&r)?;
        if crate::content_id(&component)? != r.component || component.manifest.kind != ContentKind::Component {
            return Err(Error::Denied);
        }
        let total = inputs.iter().try_fold(0_usize, |sum, v| sum.checked_add(v.len())).ok_or(Error::Bound)?;
        if inputs.len() > MAX_INPUTS || total > MAX_CONTENT_BYTES {
            return Err(Error::Bound);
        }
        let cancel = self.claim(r.fence.assignment)?;
        // Never abort this blocking task and then claim it stopped. The service
        // retains its journal Arc until the awaited execution actually returns.
        let database_guard = self.database_guard.clone();
        let output = tokio::task::spawn_blocking(move || {
            // Remains held even if the async caller is aborted. No other process
            // can reopen this worker journal and infer teardown before return.
            let _exclusive_database_lifetime = database_guard;
            run_component(r, component.bytes, inputs, cancel)
        })
        .await
        .map_err(|_| Error::Unknown)?;
        Ok(ExecutionOutput { stopped: true, output })
    }
    fn cancel(&self, id: Id) -> Result<(), Error> {
        let mut slots = self.slots.lock().map_err(|_| Error::Unavailable)?;
        if !slots.contains_key(&id) && slots.len() >= MAX_WORKER_RECORDS {
            return Err(Error::Bound);
        }
        let slot = slots.entry(id).or_insert_with(|| Slot {
            cancel: Arc::new(AtomicBool::new(true)),
            started: false,
        });
        slot.cancel.store(true, Ordering::Release);
        Ok(())
    }
}
struct Watchdog {
    done: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Watchdog {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
struct StoreData {
    limits: StoreLimits,
}
fn run_component(
    r: ExecutionRequest,
    bytes: Vec<u8>,
    inputs: Vec<Vec<u8>>,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<u8>, FailureCode> {
    if cancel.load(Ordering::Acquire) {
        return Err(FailureCode::Interrupted);
    }
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .consume_fuel(true)
        .epoch_interruption(true)
        .cranelift_nan_canonicalization(true);
    let engine = Engine::new(&config).map_err(|_| FailureCode::Internal)?;
    let done = Arc::new(AtomicBool::new(false));
    let done_thread = done.clone();
    let engine_thread = engine.clone();
    let cancel_thread = cancel.clone();
    let deadline = Instant::now().checked_add(Duration::from_millis(r.timeout_ms)).ok_or(FailureCode::Internal)?;
    let thread = std::thread::Builder::new()
        .name("orchestrator-wasm-deadline".into())
        .spawn(move || {
            while !done_thread.load(Ordering::Acquire) {
                if cancel_thread.load(Ordering::Acquire) || Instant::now() >= deadline {
                    engine_thread.increment_epoch();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
        .map_err(|_| FailureCode::Internal)?;
    let _watchdog = Watchdog {
        done,
        thread: Some(thread),
    };
    // No AOT deserialization, filesystem cache or host imports. Compilation itself
    // is not interruptible by fuel/epoch; host cgroup limits remain necessary.
    let component = Component::new(&engine, &bytes).map_err(|_| FailureCode::Compile)?;
    if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
        return Err(FailureCode::FuelOrDeadline);
    }
    let limits = StoreLimitsBuilder::new()
        .memory_size(usize::try_from(r.resources.memory_bytes).map_err(|_| FailureCode::Rejected)?)
        .instances(8)
        .memories(1)
        .tables(4)
        .table_elements(65_536)
        .build();
    let mut store = Store::new(&engine, StoreData { limits });
    store.limiter(|s| &mut s.limits);
    store.set_fuel(r.fuel).map_err(|_| FailureCode::Internal)?;
    store.set_epoch_deadline(1);
    let linker = Linker::<StoreData>::new(&engine);
    let instance = linker.instantiate(&mut store, &component).map_err(|_| FailureCode::Instantiate)?;
    let run = instance
        .get_typed_func::<(Vec<Vec<u8>>,), (Vec<u8>,)>(&mut store, "run")
        .map_err(|_| FailureCode::Instantiate)?;
    // Wasmtime 45 performs component post-return automatically inside call.
    let (output,) = run.call(&mut store, (inputs,)).map_err(|_| FailureCode::Trap)?;
    if output.len() as u64 > r.max_output_bytes {
        return Err(FailureCode::OutputLimit);
    }
    drop(store); // stopped=true is only constructed after this function returns.
    Ok(output)
}
