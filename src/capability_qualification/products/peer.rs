//! Distinct-process peers for product producers. A peer is the same executable
//! as the producer, started with one fixed role and an exclusively owned run
//! directory; it emits no qualification evidence. The producer owns every
//! peer it starts and kills/reaps it on every exit path.

const PEER_POLL_MILLISECONDS: u64 = 10;
const PEER_FILE_MAX_BYTES: u64 = 1_048_576;
#[cfg(test)]
pub(in crate::capability_qualification) const PEER_ROLE_ENV: &str = "MOLTEN_QUALIFICATION_PEER_ROLE";
#[cfg(test)]
pub(in crate::capability_qualification) const PEER_RUN_DIRECTORY_ENV: &str = "MOLTEN_QUALIFICATION_PEER_RUN_DIRECTORY";
#[cfg(test)]
const PEER_TEST_FILTER: &str = "capability_qualification::tests::peer_entry";

#[cfg(test)]
mod harness;

/// How the producer starts a peer of itself.
#[derive(Debug, Clone)]
pub enum Launcher {
    /// `molten evidence qualify-peer --role <role> --run-directory <dir>` of the
    /// executing (admitted) binary.
    Cli { program: std::path::PathBuf },
    /// The unit-test binary running only its peer entry test.
    #[cfg(test)]
    TestHarness { program: std::path::PathBuf },
}

impl Launcher {
    pub fn current() -> crate::error::Result<Self> {
        Ok(Self::Cli {
            program: std::env::current_exe().map_err(crate::error::Failure::from)?,
        })
    }

    pub(super) fn spawn(&self, role: &str, run_directory: &std::path::Path) -> crate::error::Result<Process> {
        let mut command = match self {
            Self::Cli { program } => {
                let mut command = std::process::Command::new(program);
                command.args(["evidence", "qualify-peer", "--role", role, "--run-directory"]).arg(run_directory);
                command
            }
            #[cfg(test)]
            Self::TestHarness { program } => {
                let mut command = std::process::Command::new(program);
                command
                    .args(["--exact", PEER_TEST_FILTER, "--nocapture", "--test-threads", "1"])
                    .env(PEER_ROLE_ENV, role)
                    .env(PEER_RUN_DIRECTORY_ENV, run_directory);
                command
            }
        };
        let log =
            std::fs::File::create(run_directory.join(format!("{role}.log"))).map_err(crate::error::Failure::from)?;
        let error_log = log.try_clone().map_err(crate::error::Failure::from)?;
        let child = command
            .stdin(std::process::Stdio::null())
            .stdout(log)
            .stderr(error_log)
            .spawn()
            .map_err(crate::error::Failure::from)?;
        Ok(Process { child: Some(child) })
    }
}

/// An owned peer process; dropped peers are killed and reaped.
pub(super) struct Process {
    child: Option<std::process::Child>,
}

impl Process {
    pub(super) fn id(&self) -> Option<u32> {
        self.child.as_ref().map(std::process::Child::id)
    }

    /// Abrupt process loss (SIGKILL) and reap.
    pub(super) fn kill(&mut self) -> crate::error::Result<()> {
        if let Some(mut child) = self.child.take() {
            child.kill().map_err(crate::error::Failure::from)?;
            child.wait().map_err(crate::error::Failure::from)?;
        }
        Ok(())
    }

    /// Wait for a clean exit within the bound; otherwise kill and deny.
    pub(super) fn wait_success(&mut self, timeout: std::time::Duration) -> crate::error::Result<()> {
        let Some(child) = self.child.as_mut() else {
            return Err(super::super::invalid("peer was already reaped"));
        };
        let Some(status) = poll(timeout, || child.try_wait().map_err(crate::error::Failure::from))? else {
            self.kill()?;
            return Err(super::super::invalid("qualification peer did not exit within its bound"));
        };
        self.child = None;
        if status.success() {
            Ok(())
        } else {
            Err(super::super::invalid("qualification peer exited unsuccessfully"))
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Atomically publish one canonical value in a run directory.
pub(super) fn write_value(path: &std::path::Path, value: &preserves::IOValue) -> crate::error::Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, crate::preserves_rail::canonical_bytes(value)?).map_err(crate::error::Failure::from)?;
    std::fs::rename(temporary, path).map_err(crate::error::Failure::from)
}

/// Wait (bounded) for a peer-published canonical value and strictly decode it.
pub(super) fn wait_value(
    path: &std::path::Path,
    timeout: std::time::Duration,
) -> crate::error::Result<preserves::IOValue> {
    let value = poll(timeout, || {
        if !path.is_file() {
            return Ok(None);
        }
        let metadata = std::fs::metadata(path).map_err(crate::error::Failure::from)?;
        if metadata.len() > PEER_FILE_MAX_BYTES {
            return Err(super::super::invalid("peer value exceeds its byte bound"));
        }
        let bytes = std::fs::read(path).map_err(crate::error::Failure::from)?;
        Ok(Some(crate::preserves_rail::strict_canonical_decode(&bytes)?.value))
    })?;
    value.ok_or_else(|| super::super::invalid("timed out waiting for a qualification peer value"))
}

/// Probe until it yields a value, sleeping one poll interval between probes,
/// until the monotonic deadline `timeout` expires (also capped at the number
/// of intervals that fit in `timeout`), with one final probe at the bound.
fn poll<T>(
    timeout: std::time::Duration,
    mut probe: impl FnMut() -> crate::error::Result<Option<T>>,
) -> crate::error::Result<Option<T>> {
    let mut deadline = crate::fabric_time::SupervisionDeadline::after(timeout)?;
    let attempts = timeout.as_millis().div_ceil(u128::from(PEER_POLL_MILLISECONDS));
    for _ in 0..attempts {
        if let Some(value) = probe()? {
            return Ok(Some(value));
        }
        if deadline.is_expired()? {
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(PEER_POLL_MILLISECONDS));
    }
    probe()
}

/// Entry for a peer process: dispatch one fixed role.
pub(in crate::capability_qualification) fn run(
    role: &str,
    run_directory: &std::path::Path,
) -> crate::error::Result<()> {
    if !run_directory.is_dir() {
        return Err(super::super::invalid("qualification peer requires an existing run directory"));
    }
    if let Some(result) = super::transport::peer(role, run_directory) {
        return result;
    }
    if let Some(result) = super::control_plane::peer(role, run_directory) {
        return result;
    }
    Err(super::super::invalid("unsupported qualification peer role"))
}
