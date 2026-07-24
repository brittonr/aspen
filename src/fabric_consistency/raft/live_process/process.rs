use std::fs::File;
use std::path::Path;
use std::process::Child;
use std::process::Command;
use std::process::Stdio;
use std::time::Duration;
use std::time::Instant;

use super::*;

const CHILD_WAIT_POLL_MILLISECONDS: u64 = 10;

pub(super) struct ChildGuard {
    child: Child,
    finished: bool,
}

impl ChildGuard {
    pub(super) fn spawn(executable: &Path, run_directory: &Path, node_id: &str, mode: ChildMode) -> Result<Self> {
        let log_path = run_directory.join(format!("{node_id}-child.log"));
        let stdout = File::create(log_path).map_err(MoltenError::from)?;
        let stderr = stdout.try_clone().map_err(MoltenError::from)?;
        let child = Command::new(executable)
            .args(["--exact", CHILD_TEST_FILTER, "--nocapture"])
            .env(CHILD_NODE_ENV, node_id)
            .env(CHILD_RUN_DIRECTORY_ENV, run_directory)
            .env(CHILD_MODE_ENV, mode.as_str())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(MoltenError::from)?;
        Ok(Self { child, finished: false })
    }

    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn crash(&mut self) -> Result<()> {
        self.child.kill().map_err(MoltenError::from)?;
        let _status = self.child.wait().map_err(MoltenError::from)?;
        self.finished = true;
        Ok(())
    }

    pub(super) fn wait_success(&mut self, timeout: Duration) -> Result<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if let Some(status) = self.child.try_wait().map_err(MoltenError::from)? {
                self.finished = true;
                if status.success() {
                    return Ok(());
                }
                return Err(MoltenError::invalid_harness(format!("live Raft child exited with {status}")));
            }
            std::thread::sleep(Duration::from_millis(CHILD_WAIT_POLL_MILLISECONDS));
        }
        let _kill_result = self.child.kill();
        let _wait_result = self.child.wait();
        self.finished = true;
        Err(MoltenError::invalid_harness("live Raft child exceeded its bounded shutdown deadline"))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.finished {
            let _kill_result = self.child.kill();
            let _wait_result = self.child.wait();
            self.finished = true;
        }
    }
}
