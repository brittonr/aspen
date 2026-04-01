//! Process management for aspen-node child processes.

use std::collections::HashMap;
use std::time::Duration;

use snafu::ResultExt;
use tokio::process::Child;
use tokio::process::Command;
use tracing::info;
use tracing::warn;

use crate::RunConfig;
use crate::error::DogfoodResult;
use crate::error::NodeCrashSnafu;
use crate::error::ProcessSpawnSnafu;

/// Information returned after spawning and health-checking a node.
pub struct NodeInfo {
    pub pid: u32,
    pub ticket: String,
    pub endpoint_addr: String,
}

/// Manages spawned aspen-node child processes.
pub struct NodeManager {
    children: HashMap<String, Child>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
        }
    }

    /// Spawn an aspen-node process and return its PID.
    ///
    /// The node writes its cluster ticket to `<data_dir>/cluster-ticket.txt`
    /// once it's ready.
    pub async fn spawn_node(
        &mut self,
        config: &RunConfig,
        node_id: u32,
        label: &str,
        data_dir: &str,
        iroh_secret_key: &str,
        extra_env: &[(&str, &str)],
    ) -> DogfoodResult<u32> {
        let cookie = config.cookie();

        let mut cmd = Command::new(&config.aspen_node_bin);
        cmd.arg("--node-id")
            .arg(node_id.to_string())
            .arg("--data-dir")
            .arg(data_dir)
            .arg("--cookie")
            .arg(&cookie)
            .arg("--iroh-secret-key")
            .arg(iroh_secret_key)
            .arg("--disable-mdns")
            .arg("--heartbeat-interval-ms")
            .arg("500")
            .arg("--election-timeout-min-ms")
            .arg("1500")
            .arg("--election-timeout-max-ms")
            .arg("3000")
            .arg("--enable-workers")
            .arg("--enable-ci")
            .arg("--ci-auto-trigger");

        if let Some(ref gw_bin) = config.nix_cache_gateway_bin {
            // Only add gateway URL if the binary is available
            if std::path::Path::new(gw_bin).exists() || which_exists(gw_bin) {
                cmd.arg("--nix-cache-gateway-url").arg("http://127.0.0.1:8380");
            }
        }

        // Standard env vars the node expects
        cmd.env("ASPEN_DOCS_ENABLED", "true")
            .env("ASPEN_DOCS_IN_MEMORY", "true")
            .env("ASPEN_HOOKS_ENABLED", "true")
            .env("ASPEN_RESTART_METHOD", "execve")
            .env("ASPEN_PROFILE_PATH", format!("{data_dir}/nix-profile"));

        if config.nix_cache_gateway_bin.is_some() {
            cmd.env("ASPEN_NIX_CACHE_ENABLED", "true");
        }

        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        let log_path = format!("{}/{label}.log", config.cluster_dir);
        let log_file = std::fs::File::create(&log_path).context(ProcessSpawnSnafu {
            binary: format!("log file {log_path}"),
        })?;
        let stderr_file = log_file.try_clone().context(ProcessSpawnSnafu {
            binary: format!("log file clone {log_path}"),
        })?;

        cmd.stdout(std::process::Stdio::from(log_file)).stderr(std::process::Stdio::from(stderr_file));

        let child = cmd.spawn().context(ProcessSpawnSnafu {
            binary: &config.aspen_node_bin,
        })?;

        let pid = child.id().unwrap_or(0);
        info!("  spawned {label} (pid {pid})");

        self.children.insert(label.to_string(), child);
        Ok(pid)
    }

    /// Check if a managed node is still running. Returns stderr on crash.
    #[allow(dead_code)]
    pub fn check_alive(&mut self, label: &str) -> DogfoodResult<()> {
        let child = match self.children.get_mut(label) {
            Some(c) => c,
            None => return Ok(()), // not managed by us
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                // Node exited unexpectedly
                let pid = child.id().unwrap_or(0);
                NodeCrashSnafu {
                    pid,
                    stderr: format!("exited with {status}"),
                }
                .fail()
            }
            Ok(None) => Ok(()), // still running
            Err(_) => Ok(()),   // can't check, assume alive
        }
    }

    /// Gracefully stop all managed children (SIGTERM → 10s → SIGKILL).
    #[allow(dead_code)]
    pub async fn stop_all(&mut self) -> DogfoodResult<()> {
        for (label, child) in self.children.iter_mut() {
            let pid = child.id().unwrap_or(0);
            info!("  stopping {label} (pid {pid})...");

            // Send SIGTERM
            let _ = child.kill().await;
        }

        // Wait for all children to exit with a timeout
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        for (_label, child) in self.children.iter_mut() {
            let timeout = deadline.saturating_duration_since(tokio::time::Instant::now());
            let _ = tokio::time::timeout(timeout, child.wait()).await;
        }

        self.children.clear();
        Ok(())
    }
}

/// Wait for a cluster-ticket.txt to appear in `data_dir`, up to `timeout`.
pub async fn wait_for_ticket(data_dir: &str, timeout: Duration) -> DogfoodResult<String> {
    let ticket_path = format!("{data_dir}/cluster-ticket.txt");
    let start = tokio::time::Instant::now();

    loop {
        if let Ok(ticket) = tokio::fs::read_to_string(&ticket_path).await {
            let ticket = ticket.trim().to_string();
            if !ticket.is_empty() {
                return Ok(ticket);
            }
        }

        if start.elapsed() > timeout {
            return crate::error::TimeoutSnafu {
                operation: format!("waiting for ticket at {ticket_path}"),
                timeout_secs: timeout.as_secs(),
            }
            .fail();
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Stop nodes by PID (used by `stop` subcommand operating from state file).
pub async fn stop_nodes_by_pids(pids: &[u32]) -> DogfoodResult<()> {
    for &pid in pids {
        // Check if process is alive
        let is_alive = unsafe { libc::kill(pid as i32, 0) == 0 };
        if !is_alive {
            continue;
        }

        // Send SIGTERM
        info!("  sending SIGTERM to pid {pid}");
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
    }

    // Grace period
    tokio::time::sleep(Duration::from_secs(2)).await;

    // SIGKILL any survivors
    for &pid in pids {
        let is_alive = unsafe { libc::kill(pid as i32, 0) == 0 };
        if is_alive {
            warn!("  pid {pid} still alive, sending SIGKILL");
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }

    Ok(())
}

fn which_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
