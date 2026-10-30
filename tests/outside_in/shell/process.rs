//! Labelled scenario processes and the shell's early-stop outcomes.

use molten_core::outside_in::*;

/// Why the shell stopped a scenario before its observation phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScenarioStop {
    Budget(BudgetKind),
    StartupFailed(String),
    Unsupported(String),
}

impl ScenarioStop {
    pub(crate) fn into_execution(self) -> ScenarioExecution {
        match self {
            Self::Budget(kind) => ScenarioExecution::BudgetExhausted(kind),
            Self::StartupFailed(process) => ScenarioExecution::StartupFailed { process },
            Self::Unsupported(behavior) => ScenarioExecution::Unsupported { behavior },
        }
    }
}

pub(crate) type StepResult<T> = Result<T, ScenarioStop>;

/// One owned scenario process, labelled for diagnostics and addressable by signal.
pub(crate) struct LabeledChild {
    pub(crate) label: String,
    pid: u32,
    owned: super::owned::Child,
}

impl LabeledChild {
    pub(crate) fn spawn(label: &str, command: std::process::Command, limit: usize) -> StepResult<Self> {
        let child = super::owned::spawn_piped(command).map_err(|_| ScenarioStop::StartupFailed(label.to_string()))?;
        Ok(Self {
            label: label.to_string(),
            pid: child.id(),
            owned: super::owned::Child::adopt(child, limit),
        })
    }

    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }

    pub(crate) fn poll(&mut self) -> Option<std::process::ExitStatus> {
        self.owned.poll()
    }

    /// Join the output readers of an exited process, so its output is whole.
    pub(crate) fn drain(&mut self) {
        self.owned.drain();
    }

    pub(crate) fn is_overflowed(&self) -> bool {
        self.owned.is_overflowed()
    }

    pub(crate) fn output(&self) -> String {
        self.owned.output()
    }

    /// Kill and reap the process. A reaped process reports its exit status.
    pub(crate) fn stop(&mut self) -> bool {
        self.owned.stop()
    }
}
