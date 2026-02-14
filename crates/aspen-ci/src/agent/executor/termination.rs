//! Process group termination with graceful shutdown.

use std::time::Duration;

use command_group::AsyncGroupChild;

/// Terminate a process group gracefully.
///
/// On Unix:
/// 1. Send SIGTERM to process group
/// 2. Wait for grace period
/// 3. Send SIGKILL if still running
/// 4. Reap the process
#[cfg(unix)]
pub(crate) async fn terminate_process_group(child: &mut AsyncGroupChild, grace: Duration) {
    use nix::sys::signal::Signal;
    use nix::sys::signal::{self};
    use nix::unistd::Pid;
    use tracing::warn;

    let Some(pid) = child.inner().id() else {
        return; // Already exited
    };
    let pgid = Pid::from_raw(-(pid as i32));

    // Send SIGTERM to process group
    if let Err(e) = signal::kill(pgid, Signal::SIGTERM)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGTERM to process group failed");
    }

    // Wait for graceful exit
    let deadline = tokio::time::Instant::now() + grace;
    while tokio::time::Instant::now() < deadline {
        if child.inner().try_wait().ok().flatten().is_some() {
            return; // Exited gracefully
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Force kill
    if let Err(e) = signal::kill(pgid, Signal::SIGKILL)
        && e != nix::errno::Errno::ESRCH
    {
        warn!(pid, error = ?e, "SIGKILL to process group failed");
    }

    // Reap
    let _ = child.wait().await;
}

#[cfg(not(unix))]
pub(crate) async fn terminate_process_group(child: &mut AsyncGroupChild, _grace: Duration) {
    // On non-Unix, just kill directly via the async method
    let _ = child.kill().await;
    let _ = child.wait().await;
}
