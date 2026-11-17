// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit opt-in process fault injection for the same-host conformance fixture.
//! Never enabled by policy files, peer requests, or environment variables.
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use molten_batch_application::Error;
#[derive(Clone)]
enum FaultAction {
    Exit,
    Pause(Arc<AtomicBool>),
}
#[derive(Clone, Default)]
pub struct ProcessFault(Option<(String, FaultAction)>);
impl ProcessFault {
    pub fn at(point: Option<String>) -> Result<Self, Error> {
        Self::configured(point, FaultAction::Exit)
    }
    pub fn pause_at(point: Option<String>) -> Result<Self, Error> {
        Self::configured(point, FaultAction::Pause(Arc::new(AtomicBool::new(false))))
    }
    pub fn resume(&self) {
        if let Some((_, FaultAction::Pause(resumed))) = &self.0 {
            resumed.store(true, Ordering::Release);
        }
    }
    fn configured(point: Option<String>, action: FaultAction) -> Result<Self, Error> {
        if point.as_deref().is_some_and(|p| {
            !matches!(
                p,
                "before-dispatch"
                    | "after-dispatch"
                    | "before-output-persist"
                    | "after-output-persist"
                    | "before-terminal-commit"
                    | "after-terminal-commit"
                    | "before-progress-commit"
                    | "after-progress-commit"
                    | "before-progress-reply"
                    | "after-progress-ack"
                    | "before-worker-accept"
                    | "after-worker-accept"
                    | "before-worker-begin"
                    | "after-worker-begin"
                    | "before-worker-calculation"
                    | "after-worker-calculation"
                    | "before-worker-output-persist"
                    | "after-worker-output-persist"
                    | "before-worker-terminal-commit"
                    | "after-worker-terminal-commit"
                    | "before-worker-reply"
                    | "after-worker-ack"
            )
        }) {
            return Err(Error::Invalid("unknown process fault point".into()));
        }
        Ok(Self(point.map(|point| (point, action))))
    }
    pub(crate) fn hit(&self, point: &str) {
        if let Some((selected, action)) = &self.0 {
            if selected != point {
                return;
            }
            match action {
                FaultAction::Exit => {
                    eprintln!("orchestrator injected-process-fault point={point}");
                    // Abrupt exit: only exclusive reopen may infer stopped work.
                    std::process::exit(86);
                }
                FaultAction::Pause(resumed) => {
                    if resumed.load(Ordering::Acquire) {
                        return;
                    }
                    eprintln!("orchestrator paused-process-boundary point={point}");
                    // The conformance owner may resume once via SIGUSR1, or
                    // SIGKILL and wait before reopening the durable owner.
                    while !resumed.load(Ordering::Acquire) {
                        std::thread::park_timeout(Duration::from_millis(10));
                    }
                }
            }
        }
    }
}
