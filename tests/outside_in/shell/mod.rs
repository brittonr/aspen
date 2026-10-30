//! The scenario shell: budgets, owned processes, signals, and teardown.
//!
//! Elapsed time is read only through supervision deadlines, and only to
//! enforce budgets and holds. No observation the judgment reads carries a time.

use molten_core::outside_in::*;

mod owned;
mod partition;
mod process;

pub(crate) use partition::*;
pub(crate) use process::*;

type SupervisionDeadline = molten::fabric_time::SupervisionDeadline;

const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
const SIGNAL_TOOL: &str = "kill";
/// The variable naming the independently provisioned startup source trust; each test captures it.
pub(crate) const SOURCE_TRUST_ENV: &str = "MOLTEN_TEST_SOURCE_TRUST";
const MAX_SOURCE_TRUST_BYTES: usize = 16_384;

/// The signed producer output and its trust record, selected independently of this suite.
struct SignedSource {
    binary: std::path::PathBuf,
    trust: Vec<u8>,
}

impl SignedSource {
    /// The selection the test captured, if any; an unreadable selection is unsupported.
    fn selected(path: Option<&std::ffi::OsStr>) -> StepResult<Option<Self>> {
        let unsupported = || ScenarioStop::Unsupported("source-trust".to_string());
        let Some(path) = path else {
            return Ok(None);
        };
        let trust = std::fs::read(path).map_err(|_| unsupported())?;
        if trust.len() > MAX_SOURCE_TRUST_BYTES {
            return Err(unsupported());
        }
        let selection: serde_json::Value = serde_json::from_slice(&trust).map_err(|_| unsupported())?;
        let output = selection.get("output").and_then(serde_json::Value::as_str).ok_or_else(unsupported)?;
        Ok(Some(Self {
            binary: std::path::Path::new(output).join("bin/molten"),
            trust,
        }))
    }
}

fn clock_stop(_: molten::error::Failure) -> ScenarioStop {
    ScenarioStop::Unsupported("supervision-clock".to_string())
}

/// A deadline for a bounded hold inside the scenario.
pub(crate) fn hold_deadline(duration: std::time::Duration) -> StepResult<SupervisionDeadline> {
    SupervisionDeadline::after(duration).map_err(clock_stop)
}

/// A handle to a process the shell owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessHandle(usize);

/// The shell of one scenario: its deadline, owned processes, and owned scratch paths.
pub(crate) struct Harness {
    deadline: SupervisionDeadline,
    max_processes: usize,
    max_output_bytes: usize,
    processes: Vec<LabeledChild>,
    scratch: Vec<(String, std::path::PathBuf)>,
    workspace: crate::test_support::ProcessWorkspace,
    signal_tool: String,
    /// Absent, processes run the unsigned test build and node startup admission denies them.
    source: Option<SignedSource>,
}

impl Harness {
    pub(crate) fn new(declaration: &ScenarioDeclaration, source_trust: Option<&std::ffi::OsStr>) -> StepResult<Self> {
        let budgets = declaration.budgets;
        let workspace = crate::test_support::process_workspace(&declaration.scenario_id)
            .map_err(|_| ScenarioStop::Unsupported("process-workspace".to_string()))?;
        let max_processes =
            usize::try_from(budgets.max_processes).map_err(|_| ScenarioStop::Budget(BudgetKind::Processes))?;
        Ok(Self {
            deadline: hold_deadline(std::time::Duration::from_millis(budgets.wall_clock_ms))?,
            max_processes,
            max_output_bytes: usize::try_from(budgets.max_retained_output_bytes)
                .map_err(|_| ScenarioStop::Budget(BudgetKind::RetainedOutput))?,
            processes: Vec::with_capacity(max_processes),
            scratch: Vec::new(),
            workspace,
            signal_tool: SIGNAL_TOOL.to_string(),
            source: SignedSource::selected(source_trust)?,
        })
    }

    /// Replace the signal tool, to record the unsupported-host outcome.
    pub(crate) fn set_signal_tool(&mut self, tool: &str) {
        self.signal_tool = tool.to_string();
    }

    /// Install the selected startup source trust in an initialized node root.
    pub(crate) fn install_source_trust(&self, state_root: &std::path::Path) -> StepResult<()> {
        let source = self.source.as_ref().ok_or_else(|| ScenarioStop::Unsupported("source-trust".to_string()))?;
        std::fs::write(state_root.join("source-gate-trust.json"), &source.trust)
            .map_err(|_| ScenarioStop::Unsupported("source-trust-install".to_string()))
    }

    /// Create an owned scratch directory inside the workspace.
    pub(crate) fn scratch_dir(&mut self, label: &str) -> StepResult<std::path::PathBuf> {
        let workspace: &std::path::Path = self.workspace.as_ref();
        let path = workspace.join(label);
        std::fs::create_dir_all(&path).map_err(|_| ScenarioStop::Unsupported(format!("scratch:{label}")))?;
        self.scratch.push((label.to_string(), path.clone()));
        Ok(path)
    }

    pub(crate) fn check_clock(&mut self) -> StepResult<()> {
        if self.deadline.is_expired().map_err(clock_stop)? {
            return Err(ScenarioStop::Budget(BudgetKind::WallClock));
        }
        Ok(())
    }

    /// Wait until `hold` expires, inside the wall-clock budget.
    pub(crate) fn hold_until(&mut self, hold: &mut SupervisionDeadline) -> StepResult<()> {
        while !hold.is_expired().map_err(clock_stop)? {
            self.check_clock()?;
            std::thread::sleep(POLL_INTERVAL.min(hold.remaining().map_err(clock_stop)?));
        }
        self.check_clock()
    }

    pub(crate) fn hold(&mut self, duration: std::time::Duration) -> StepResult<()> {
        self.hold_until(&mut hold_deadline(duration)?)
    }

    /// Start an owned `molten` process from the repository root.
    pub(crate) fn start(&mut self, label: &str, arguments: &[std::ffi::OsString]) -> StepResult<ProcessHandle> {
        self.check_clock()?;
        let live = self.processes.iter_mut().map(LabeledChild::poll).filter(Option::is_none).count();
        if live >= self.max_processes {
            return Err(ScenarioStop::Budget(BudgetKind::Processes));
        }
        let binary = self
            .source
            .as_ref()
            .map_or_else(|| std::path::Path::new(env!("CARGO_BIN_EXE_molten")), |source| source.binary.as_path());
        let mut command = std::process::Command::new(binary);
        command.current_dir(env!("CARGO_MANIFEST_DIR")).args(arguments);
        self.processes.push(LabeledChild::spawn(label, command, self.max_output_bytes)?);
        Ok(ProcessHandle(self.processes.len() - 1))
    }

    /// Wait for an owned process to exit inside the budgets.
    pub(crate) fn wait(&mut self, handle: ProcessHandle) -> StepResult<std::process::ExitStatus> {
        loop {
            let process = &mut self.processes[handle.0];
            if let Some(status) = process.poll() {
                process.drain();
                if process.is_overflowed() {
                    return Err(ScenarioStop::Budget(BudgetKind::RetainedOutput));
                }
                return Ok(status);
            }
            if process.is_overflowed() {
                return Err(ScenarioStop::Budget(BudgetKind::RetainedOutput));
            }
            self.check_clock()?;
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    /// Run a one-shot command to a successful exit and return its output.
    pub(crate) fn run(&mut self, label: &str, arguments: &[std::ffi::OsString]) -> StepResult<String> {
        let handle = self.start(label, arguments)?;
        if !self.wait(handle)?.success() {
            return Err(ScenarioStop::StartupFailed(label.to_string()));
        }
        Ok(self.output(handle))
    }

    /// Wait until `is_ready` holds, failing as a startup failure if the process exits first.
    pub(crate) fn await_ready(
        &mut self,
        handle: ProcessHandle,
        mut is_ready: impl FnMut(&str) -> bool,
    ) -> StepResult<()> {
        loop {
            let process = &mut self.processes[handle.0];
            if is_ready(&process.output()) {
                return Ok(());
            }
            if process.poll().is_some() {
                return Err(ScenarioStop::StartupFailed(process.label.clone()));
            }
            if process.is_overflowed() {
                return Err(ScenarioStop::Budget(BudgetKind::RetainedOutput));
            }
            self.check_clock()?;
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub(crate) fn output(&self, handle: ProcessHandle) -> String {
        self.processes[handle.0].output()
    }

    /// Kill an owned process at the declared boundary and reap it.
    pub(crate) fn kill(&mut self, handle: ProcessHandle) -> StepResult<()> {
        let process = &mut self.processes[handle.0];
        if process.stop() {
            return Ok(());
        }
        Err(ScenarioStop::Unsupported(format!("reap-after-kill:{}", process.label)))
    }

    /// Send a stop or continue signal to an owned process.
    pub(crate) fn signal(&mut self, handle: ProcessHandle, signal: &str) -> StepResult<()> {
        let behavior = format!("signal-{signal}");
        let status = std::process::Command::new(&self.signal_tool)
            .args(["-s", signal, &self.processes[handle.0].pid().to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| ScenarioStop::Unsupported(behavior.clone()))?;
        if status.success() {
            return Ok(());
        }
        Err(ScenarioStop::Unsupported(behavior))
    }

    /// Release every owned process and scratch path, and return each owned
    /// process's retained output.
    pub(crate) fn teardown(mut self) -> (TeardownStatus, Vec<(String, String)>) {
        let mut status = TeardownStatus::Complete;
        let mut logs = Vec::with_capacity(self.processes.len());
        for process in &mut self.processes {
            if !process.stop() && status == TeardownStatus::Complete {
                status = TeardownStatus::OrphanedProcess {
                    process: process.label.clone(),
                };
            }
            logs.push((process.label.clone(), process.output()));
        }
        for (label, path) in self.scratch.drain(..).rev() {
            if std::fs::remove_dir_all(&path).is_err() && status == TeardownStatus::Complete {
                status = TeardownStatus::ScratchPathRetained { path_label: label };
            }
        }
        (status, logs)
    }
}
