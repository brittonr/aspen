use super::*;
use crate::fabric_simulation::WorkloadExecutor;

pub(super) struct Driver<'a> {
    input: &'a crate::fabric_simulation::bounded_client::Input,
    directory: &'a std::path::Path,
    executable: &'a std::path::Path,
    children: Vec<process::ChildGuard>,
    pub(super) executed: Vec<String>,
    pub(super) failure: Option<String>,
}

impl<'a> Driver<'a> {
    pub(super) fn new(
        input: &'a crate::fabric_simulation::bounded_client::Input,
        directory: &'a std::path::Path,
        executable: &'a std::path::Path,
    ) -> Self {
        Self {
            input,
            directory,
            executable,
            children: Vec::with_capacity(input.participants.len()),
            executed: Vec::with_capacity(input.steps.len()),
            failure: None,
        }
    }

    fn spawn(&mut self, mode: ChildMode) -> crate::error::Result<()> {
        for participant in &self.input.participants {
            self.children.push(process::ChildGuard::spawn(self.executable, self.directory, participant, mode)?);
        }
        wait_for_nodes(self.directory, "ready")
    }

    fn command(&mut self, command: crate::fabric_simulation::bounded_client::Command) -> crate::error::Result<()> {
        match command {
            crate::fabric_simulation::bounded_client::Command::Start => self.spawn(ChildMode::Fresh),
            crate::fabric_simulation::bounded_client::Command::CommitReadSnapshotPartition => {
                write_signal(&self.directory.join(START_FILE), "start")?;
                wait_for_file(&self.directory.join(LEADER_DONE_FILE))?;
                // Request != application: each follower must record an actual
                // received event dropped by the existing transport fault gate.
                for participant in &self.input.participants[1..] {
                    wait_for_file(&self.directory.join(format!("{participant}-client-fault-applied.preserves")))?;
                }
                Ok(())
            }
            crate::fabric_simulation::bounded_client::Command::Checkpoint => {
                write_signal(&self.directory.join(CHECKPOINT_FILE), "checkpoint")?;
                wait_for_nodes(self.directory, "checkpoint")
            }
            crate::fabric_simulation::bounded_client::Command::Crash => {
                for child in &mut self.children {
                    child.crash()?;
                }
                self.children.clear();
                write_signal(
                    &self.directory.join("client-crash-applied.preserves"),
                    "all-owned-children-killed-and-reaped",
                )?;
                clear_phase_files(self.directory)
            }
            crate::fabric_simulation::bounded_client::Command::Reopen => self.spawn(ChildMode::Recover),
            crate::fabric_simulation::bounded_client::Command::FaultOffRecovery => {
                if self.directory.join(PARTITION_FILE).try_exists().map_err(crate::error::Failure::from)? {
                    return Err(crate::error::Failure::invalid_harness("recovery phase still has a disruptive fault"));
                }
                write_signal(&self.directory.join(RESTART_START_FILE), "restart-start")?;
                wait_for_file(&self.directory.join(STALE_FENCED_FILE))?;
                for participant in &self.input.participants {
                    wait_for_file(&self.directory.join(format!("{participant}-client-horizon.preserves")))?;
                }
                write_signal(&self.directory.join(STOP_FILE), "stop")?;
                wait_for_nodes(self.directory, "terminal")?;
                for child in &mut self.children {
                    child.wait_success(std::time::Duration::from_secs(CHILD_TIMEOUT_SECONDS))?;
                }
                Ok(())
            }
        }
    }
}

// r[impl molten.chaoscontrol_client.real_execution]
impl WorkloadExecutor for Driver<'_> {
    fn execute_step(
        &mut self,
        step: &crate::fabric_simulation::LiveWorkloadStep,
    ) -> Option<crate::fabric_simulation::OutcomeClass> {
        if self.failure.is_some() {
            return None;
        }
        let position = self.executed.len();
        let Some(command) = self.input.steps.get(position) else {
            return Some(crate::fabric_simulation::OutcomeClass::Unsupported);
        };
        if step.sequence != position as u64
            || step.request_ref != command.request_ref
            || step.class != crate::fabric::FabricPortClass::Consistency
            || step.ordering_key.as_deref() != Some(crate::fabric_simulation::bounded_client::NATIVE_PROFILE)
        {
            self.failure = Some("executor request identity/order mismatch".to_string());
            return None;
        }
        match self.command(command.command) {
            Ok(()) => {
                self.executed.push(step.request_ref.clone());
                Some(crate::fabric_simulation::OutcomeClass::Delivered)
            }
            Err(error) => {
                self.failure = Some(error.to_string());
                None
            }
        }
    }
}
