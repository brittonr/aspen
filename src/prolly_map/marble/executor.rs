// The bounded executor that confines blocking marble operations.
//
// One named worker thread opens and owns the store state and runs every
// operation on it in submission order. Callers queue a job in a bounded
// channel and wait for its reply; a full queue is refused instead of growing.
// The state never leaves the worker, so blocking heap I/O, recovery, and
// maintenance never run on a caller's thread.

use super::super::*;

type Job<T> = Box<dyn FnOnce(&mut T) + Send>;

pub(super) struct StoreExecutor<T> {
    sender: Option<std::sync::mpsc::SyncSender<Job<T>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl<T: 'static> StoreExecutor<T> {
    /// Start the worker, open the state on it, and wait for the open result.
    // r[impl aspen.marble_store.spike]
    pub(super) fn start(
        queue_depth: u32,
        open: impl FnOnce() -> ProllyPortResult<T> + Send + 'static,
    ) -> ProllyPortResult<Self> {
        let depth = usize::try_from(queue_depth)
            .map_err(|_| executor_error("prolly-marble-executor-bound", "queue depth is out of range", false))?;
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Job<T>>(depth);
        let (opened_sender, opened) = std::sync::mpsc::sync_channel::<ProllyPortResult<()>>(1);
        let worker = std::thread::Builder::new()
            .name("molten-marble-store".to_owned())
            .spawn(move || match open() {
                Ok(mut state) => {
                    let is_delivered = opened_sender.send(Ok(())).is_ok();
                    debug_assert!(is_delivered, "the opener waits for the open result");
                    for job in receiver {
                        job(&mut state);
                    }
                }
                Err(error) => {
                    let is_delivered = opened_sender.send(Err(error)).is_ok();
                    debug_assert!(is_delivered, "the opener waits for the open result");
                }
            })
            .map_err(|error| executor_error("prolly-marble-executor-spawn", error.to_string(), false))?;
        let open_result = opened.recv().unwrap_or_else(|_| {
            Err(executor_error("prolly-marble-executor-stopped", "the worker stopped while opening", false))
        });
        let executor = Self {
            sender: Some(sender),
            worker: Some(worker),
        };
        open_result.map(|()| executor)
    }

    /// Run one job on the worker and wait for its result.
    ///
    /// A full queue refuses the job before it runs. A worker that stops after
    /// accepting a job leaves its outcome unknown.
    pub(super) fn run<R: Send + 'static>(&self, job: impl FnOnce(&mut T) -> R + Send + 'static) -> ProllyPortResult<R> {
        let Some(sender) = self.sender.as_ref() else {
            return Err(executor_error("prolly-marble-executor-stopped", "the executor is stopped", false));
        };
        let (reply_sender, reply) = std::sync::mpsc::sync_channel::<R>(1);
        let reply_job = move |state: &mut T| {
            let is_delivered = reply_sender.send(job(state)).is_ok();
            debug_assert!(is_delivered, "the caller waits for its reply while it borrows the executor");
        };
        sender.try_send(Box::new(reply_job)).map_err(|error| match error {
            std::sync::mpsc::TrySendError::Full(_) => {
                executor_error("prolly-marble-executor-saturated", "the executor queue is full", false)
            }
            std::sync::mpsc::TrySendError::Disconnected(_) => {
                executor_error("prolly-marble-executor-stopped", "the worker is stopped", false)
            }
        })?;
        reply.recv().map_err(|_| {
            executor_error("prolly-marble-executor-stopped", "the worker stopped during the operation", true)
        })
    }
}

impl<T> Drop for StoreExecutor<T> {
    fn drop(&mut self) {
        self.sender = None;
        if let Some(worker) = self.worker.take() {
            let is_clean_exit = worker.join().is_ok();
            debug_assert!(is_clean_exit, "the store worker panicked");
        }
    }
}

fn executor_error(code: &'static str, detail: impl Into<String>, is_outcome_unknown: bool) -> ProllyPortError {
    ProllyPortError::new(code, detail, is_outcome_unknown)
}
