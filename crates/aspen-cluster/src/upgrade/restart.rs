//! Process restart logic for node upgrades.
//!
//! Two strategies:
//! - Systemd: `systemctl restart <unit>` (preferred, preserves supervision)
//! - Execve: replace current process with new binary (fallback)
//!
//! Detection: check `$NOTIFY_SOCKET` or `$INVOCATION_ID` for systemd.

use tracing::info;

use super::types::NodeUpgradeError;
use super::types::RestartMethod;
use super::types::Result;

/// Detect the appropriate restart method for the current environment.
///
/// Checks for systemd indicators (`$NOTIFY_SOCKET`, `$INVOCATION_ID`)
/// and falls back to execve if not running under systemd.
pub fn detect_restart_method() -> RestartMethod {
    // Check for systemd environment variables.
    let has_notify_socket = std::env::var("NOTIFY_SOCKET").is_ok();
    let has_invocation_id = std::env::var("INVOCATION_ID").is_ok();

    if has_notify_socket || has_invocation_id {
        // Running under systemd. Derive unit name from INVOCATION_ID
        // or use default.
        let unit_name = std::env::var("ASPEN_SYSTEMD_UNIT").unwrap_or_else(|_| "aspen-node".to_string());

        info!(unit_name, has_notify_socket, has_invocation_id, "detected systemd environment");

        RestartMethod::Systemd { unit_name }
    } else {
        info!("no systemd detected, will use execve for restart");
        RestartMethod::Execve
    }
}

/// Restart the process using the specified method.
///
/// For systemd: executes `systemctl restart` and returns (the current process
/// will be killed by systemd).
///
/// For execve: replaces the current process with the new binary. This function
/// does not return on success.
pub async fn restart_process(method: &RestartMethod) -> Result<()> {
    match method {
        RestartMethod::Systemd { unit_name } => restart_systemd(unit_name).await,
        RestartMethod::Execve => restart_execve(),
    }
}

/// Restart via systemd.
async fn restart_systemd(unit_name: &str) -> Result<()> {
    info!(unit_name, "requesting systemd restart");

    let output =
        tokio::process::Command::new("systemctl").args(["restart", unit_name]).output().await.map_err(|e| {
            NodeUpgradeError::RestartFailed {
                reason: format!("systemctl exec failed: {e}"),
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NodeUpgradeError::RestartFailed {
            reason: format!("systemctl restart failed: {stderr}"),
        });
    }

    // systemctl restart is async — it returns immediately and systemd
    // handles the actual restart. This process will be killed by systemd.
    info!(unit_name, "systemctl restart issued");
    Ok(())
}

/// Restart via execve (replaces current process).
///
/// Reads `/proc/self/exe` to find the current binary path and
/// `std::env::args()` for the original arguments.
fn restart_execve() -> Result<()> {
    info!("executing process re-exec via execve");

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        // Read the current binary path from /proc/self/exe.
        let exe_path = std::fs::read_link("/proc/self/exe").map_err(|e| NodeUpgradeError::RestartFailed {
            reason: format!("read /proc/self/exe: {e}"),
        })?;

        let exe_cstring =
            CString::new(exe_path.as_os_str().as_bytes()).map_err(|e| NodeUpgradeError::RestartFailed {
                reason: format!("invalid exe path: {e}"),
            })?;

        // Collect original args.
        let args: Vec<CString> =
            std::env::args().map(|a| CString::new(a).unwrap_or_else(|_| CString::new("").unwrap())).collect();

        let arg_refs: Vec<&std::ffi::CStr> = args.iter().map(|a| a.as_c_str()).collect();

        info!(exe = %exe_path.display(), args_count = arg_refs.len(), "calling execv");

        // This does not return on success.
        nix::unistd::execv(&exe_cstring, &arg_refs).map_err(|e| NodeUpgradeError::RestartFailed {
            reason: format!("execv failed: {e}"),
        })?;

        // Unreachable on success.
        unreachable!("execv returned without error");
    }

    #[cfg(not(unix))]
    {
        Err(NodeUpgradeError::RestartFailed {
            reason: "execve not supported on this platform".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_restart_no_systemd() {
        // Remove systemd env vars (if present) for a clean test.
        // This test verifies the fallback path.
        let has_notify = std::env::var("NOTIFY_SOCKET").is_ok();
        let has_invocation = std::env::var("INVOCATION_ID").is_ok();

        if !has_notify && !has_invocation {
            let method = detect_restart_method();
            assert_eq!(method, RestartMethod::Execve);
        }
        // If systemd vars are set (e.g., running tests under systemd),
        // we can't meaningfully test the no-systemd path.
    }

    #[test]
    fn test_restart_method_equality() {
        let a = RestartMethod::Systemd {
            unit_name: "aspen-node".into(),
        };
        let b = RestartMethod::Systemd {
            unit_name: "aspen-node".into(),
        };
        assert_eq!(a, b);

        let c = RestartMethod::Execve;
        let d = RestartMethod::Execve;
        assert_eq!(c, d);

        assert_ne!(a, c);
    }
}
