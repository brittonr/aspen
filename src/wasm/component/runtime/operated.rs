//! Product-owned epoch operation around the shared execution mechanism.

const EPOCH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(1);
const DEADLINE_TICKS: u64 = 1_000;

/// The only host import this component cohort can bind.
/// Implementations retain current admission, effect execution and recording.
pub trait EffectRequestHost {
    fn invoke(&mut self, bytes: &[u8]) -> super::super::model::Result<Vec<u8>>;
}

/// One admitted engine and its operated, engine-specific epoch service.
/// Epoch deadlines interrupt guest work, not a blocked synchronous host call.
pub struct Engine {
    adapter: super::shared::EngineAdapter,
    epochs: EpochDriver,
}

impl Engine {
    pub fn build(profile: &super::super::model::RuntimeProfile) -> super::super::model::Result<Self> {
        let adapter = super::shared::EngineAdapter::build(profile)?;
        let epochs = EpochDriver::start(adapter.epoch_service())?;
        Ok(Self { adapter, epochs })
    }

    pub fn execute(&self, request: &super::shell::ExecutionRequest<'_>) -> super::shell::ExecutionOutcome {
        if !self.epochs.is_running() {
            return super::shared::denied_outcome(request, driver_denial());
        }
        self.adapter.execute(request, DEADLINE_TICKS)
    }

    pub fn execute_hosted<H: EffectRequestHost + 'static>(
        &self,
        request: &super::shell::ExecutionRequest<'_>,
        host: H,
    ) -> super::shell::ExecutionOutcome {
        if !self.epochs.is_running() {
            return super::shared::denied_outcome(request, driver_denial());
        }
        self.adapter.execute_hosted(request, DEADLINE_TICKS, host)
    }
}

fn driver_denial() -> super::super::model::Denial {
    super::super::model::Denial::classified(
        super::super::model::DenialClass::ResourceDenial,
        "the component engine epoch service is unavailable",
    )
}

struct EpochDriver {
    stop: Option<std::sync::mpsc::SyncSender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl EpochDriver {
    fn start(service: hardened_wasmtime_shell::EpochService) -> super::super::model::Result<Self> {
        let (stop, receiver) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("molten-component-epochs".to_string())
            .spawn(move || {
                while let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = receiver.recv_timeout(EPOCH_INTERVAL) {
                    service.advance();
                }
            })
            .map_err(|_| driver_denial())?;
        Ok(Self {
            stop: Some(stop),
            worker: Some(worker),
        })
    }

    fn is_running(&self) -> bool {
        self.worker.as_ref().is_some_and(|worker| !worker.is_finished())
    }
}

impl Drop for EpochDriver {
    fn drop(&mut self) {
        // Disconnect wakes the worker immediately; shutdown never waits for a tick.
        self.stop = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
