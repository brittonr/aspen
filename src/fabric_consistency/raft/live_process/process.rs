use super::*;

const CHILD_WAIT_POLL_MILLISECONDS: u64 = 10;

pub(super) struct ChildGuard {
    child: std::process::Child,
    finished: bool,
}

impl ChildGuard {
    pub(super) fn spawn(
        executable: &std::path::Path,
        run_directory: &std::path::Path,
        node_id: &str,
        mode: ChildMode,
    ) -> crate::error::Result<Self> {
        let log_path = run_directory.join(format!("{node_id}-child.log"));
        let stdout = std::fs::File::create(log_path).map_err(crate::error::MoltenError::from)?;
        let stderr = stdout.try_clone().map_err(crate::error::MoltenError::from)?;
        let child = std::process::Command::new(executable)
            .args(["--exact", CHILD_TEST_FILTER, "--nocapture"])
            .env(CHILD_NODE_ENV, node_id)
            .env(CHILD_RUN_DIRECTORY_ENV, run_directory)
            .env(CHILD_MODE_ENV, mode.as_str())
            .stdout(std::process::Stdio::from(stdout))
            .stderr(std::process::Stdio::from(stderr))
            .spawn()
            .map_err(crate::error::MoltenError::from)?;
        Ok(Self { child, finished: false })
    }

    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }

    pub(super) fn crash(&mut self) -> crate::error::Result<()> {
        self.child.kill().map_err(crate::error::MoltenError::from)?;
        let _status = self.child.wait().map_err(crate::error::MoltenError::from)?;
        self.finished = true;
        Ok(())
    }

    pub(super) fn wait_success(&mut self, timeout: std::time::Duration) -> crate::error::Result<()> {
        let started = std::time::Instant::now();
        while started.elapsed() < timeout {
            if let Some(status) = self.child.try_wait().map_err(crate::error::MoltenError::from)? {
                self.finished = true;
                if status.success() {
                    return Ok(());
                }
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "live Raft child exited with {status}"
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(CHILD_WAIT_POLL_MILLISECONDS));
        }
        let _kill_result = self.child.kill();
        let _wait_result = self.child.wait();
        self.finished = true;
        Err(crate::error::MoltenError::invalid_harness("live Raft child exceeded its bounded shutdown deadline"))
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
