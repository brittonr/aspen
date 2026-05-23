//! VM-CI readiness checks for dogfood runs.
//!
//! These checks deliberately run before CI jobs are triggered so a host that
//! cannot create VM networking fails with a receipt instead of hanging until the
//! pipeline timeout.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use crate::error::DogfoodError;
use crate::error::DogfoodResult;
use crate::error::HealthCheckSnafu;

const CAP_NET_ADMIN_BIT: u32 = 12;
const DEFAULT_NETWORK_MODE: &str = "tap";
const VM_CI_BRIDGE_NAME: &str = "aspen-ci-br0";
const VM_CI_NETWORK_SETUP_MARKER: &str = "/tmp/aspen-ci-network-configured-v3";
const VM_CI_STARTUP_READINESS_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmCiReadinessInput {
    pub kernel_path: Option<String>,
    pub initrd_path: Option<String>,
    pub toplevel_path: Option<String>,
    pub network_mode: String,
    pub tap_helper_path: Option<String>,
    pub kvm_available: bool,
    pub kvm_writable: bool,
    pub tun_available: bool,
    pub bridge_available: bool,
    pub host_network_configured: bool,
    pub has_net_admin: bool,
}

impl VmCiReadinessInput {
    fn from_host() -> Self {
        Self {
            kernel_path: std::env::var("ASPEN_CI_KERNEL_PATH").ok(),
            initrd_path: std::env::var("ASPEN_CI_INITRD_PATH").ok(),
            toplevel_path: std::env::var("ASPEN_CI_TOPLEVEL_PATH").ok(),
            network_mode: std::env::var("ASPEN_CI_NETWORK_MODE").unwrap_or_else(|_| DEFAULT_NETWORK_MODE.to_string()),
            tap_helper_path: std::env::var("ASPEN_CI_TAP_HELPER_PATH").ok(),
            kvm_available: Path::new("/dev/kvm").exists(),
            kvm_writable: std::fs::OpenOptions::new().read(true).write(true).open("/dev/kvm").is_ok(),
            tun_available: Path::new("/dev/net/tun").exists(),
            bridge_available: Path::new("/sys/class/net").join(VM_CI_BRIDGE_NAME).exists(),
            host_network_configured: Path::new(VM_CI_NETWORK_SETUP_MARKER).exists(),
            has_net_admin: current_process_has_net_admin(),
        }
    }
}

pub fn check_current_environment() -> DogfoodResult<()> {
    check_readiness(&VmCiReadinessInput::from_host()).map_err(|reason| DogfoodError::VmCiReadiness { reason })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmCiStartupReadiness {
    pub router_spawned: bool,
    pub ticket_written: bool,
    pub vm_pool_initialized: bool,
    pub worker_ready: bool,
}

impl VmCiStartupReadiness {
    fn ready(self) -> bool {
        // Worker registration/polling happens after the host initializes the
        // cluster. Requiring it here creates a startup deadlock: dogfood waits
        // for workers, while workers wait for InitCluster. Treat VM pool
        // initialization as the local VM-CI startup gate; worker progress is
        // proven by the later CI rail markers.
        self.router_spawned && self.ticket_written && self.vm_pool_initialized
    }
}

pub async fn wait_for_startup_readiness(data_dir: &str, timeout: Duration) -> DogfoodResult<()> {
    let start = tokio::time::Instant::now();
    let data_dir = PathBuf::from(data_dir);
    loop {
        let readiness = read_startup_readiness(&data_dir).await;
        if readiness.ready() {
            return Ok(());
        }

        if start.elapsed() > timeout {
            return HealthCheckSnafu {
                target: "VM-CI local startup readiness".to_string(),
                reason: format!(
                    "not ready after {}s (router_spawned={}, ticket_written={}, vm_pool_initialized={}, worker_ready={})",
                    timeout.as_secs(),
                    readiness.router_spawned,
                    readiness.ticket_written,
                    readiness.vm_pool_initialized,
                    readiness.worker_ready
                ),
            }
            .fail();
        }

        tokio::time::sleep(VM_CI_STARTUP_READINESS_POLL).await;
    }
}

async fn read_startup_readiness(data_dir: &Path) -> VmCiStartupReadiness {
    let node_log = tokio::fs::read_to_string(data_dir.with_extension("log")).await.unwrap_or_default();
    let mut readiness = parse_startup_readiness(&node_log, "");

    let serial_dir = data_dir.join("ci/vms");
    if let Ok(mut entries) = tokio::fs::read_dir(serial_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with("-serial.log")) {
                let serial = tokio::fs::read_to_string(path).await.unwrap_or_default();
                readiness = merge_startup_readiness(readiness, parse_startup_readiness("", &serial));
            }
        }
    }

    readiness
}

fn parse_startup_readiness(node_log: &str, serial_logs: &str) -> VmCiStartupReadiness {
    VmCiStartupReadiness {
        router_spawned: node_log.contains("Iroh Router spawned"),
        ticket_written: node_log.contains("cluster ticket written to file"),
        vm_pool_initialized: node_log.contains("VM pool initialized"),
        worker_ready: serial_logs.contains("worker registered with cluster")
            || serial_logs.contains("ephemeral CI worker ready")
            || serial_logs.contains("polling for jobs"),
    }
}

fn merge_startup_readiness(left: VmCiStartupReadiness, right: VmCiStartupReadiness) -> VmCiStartupReadiness {
    VmCiStartupReadiness {
        router_spawned: left.router_spawned || right.router_spawned,
        ticket_written: left.ticket_written || right.ticket_written,
        vm_pool_initialized: left.vm_pool_initialized || right.vm_pool_initialized,
        worker_ready: left.worker_ready || right.worker_ready,
    }
}

pub fn check_readiness(input: &VmCiReadinessInput) -> Result<(), String> {
    let mut failures = Vec::new();

    require_existing_path(&mut failures, "ASPEN_CI_KERNEL_PATH", input.kernel_path.as_deref());
    require_existing_path(&mut failures, "ASPEN_CI_INITRD_PATH", input.initrd_path.as_deref());
    require_existing_path(&mut failures, "ASPEN_CI_TOPLEVEL_PATH", input.toplevel_path.as_deref());

    if !input.kvm_available {
        failures.push("/dev/kvm is missing".to_string());
    } else if !input.kvm_writable {
        failures.push("/dev/kvm is not writable by this process".to_string());
    }

    match input.network_mode.as_str() {
        "none" | "isolated" => {}
        "helper" | "tap-helper" => {
            require_executable_path(&mut failures, "ASPEN_CI_TAP_HELPER_PATH", input.tap_helper_path.as_deref());
            require_host_vm_network(&mut failures, input);
        }
        _ => {
            if !input.tun_available {
                failures.push("/dev/net/tun is missing".to_string());
            }
            if !input.has_net_admin {
                failures.push(
                    "tap networking requires CAP_NET_ADMIN; run setup-ci-network/use tap-helper or set ASPEN_CI_NETWORK_MODE=none"
                        .to_string(),
                );
            }
            require_host_vm_network(&mut failures, input);
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("VM-CI readiness failed before pipeline trigger: {}", failures.join("; ")))
    }
}

fn require_existing_path(failures: &mut Vec<String>, name: &str, value: Option<&str>) {
    match value {
        Some(path) if !path.trim().is_empty() && Path::new(path).exists() => {}
        Some(path) if !path.trim().is_empty() => failures.push(format!("{name} does not exist: {path}")),
        _ => failures.push(format!("{name} is not set")),
    }
}

fn require_host_vm_network(failures: &mut Vec<String>, input: &VmCiReadinessInput) {
    if !input.bridge_available {
        failures.push(format!("VM-CI bridge {VM_CI_BRIDGE_NAME} is missing; run sudo nix run .#setup-ci-network"));
    }
    if !input.host_network_configured {
        failures.push(format!(
            "VM-CI host network/firewall marker {VM_CI_NETWORK_SETUP_MARKER} is missing; run sudo nix run .#setup-ci-network"
        ));
    }
}

fn require_executable_path(failures: &mut Vec<String>, name: &str, value: Option<&str>) {
    match value {
        Some(path) if !path.trim().is_empty() => {
            let path_ref = Path::new(path);
            if !path_ref.exists() {
                failures.push(format!("{name} does not exist: {path}"));
                return;
            }
            if !is_executable(path_ref) {
                failures.push(format!("{name} is not executable: {path}"));
            }
        }
        _ => failures.push(format!("{name} is not set")),
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).map(|metadata| metadata.permissions().mode() & 0o111 != 0).unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn current_process_has_net_admin() -> bool {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return false;
    };
    parse_cap_eff_has_net_admin(&status)
}

fn parse_cap_eff_has_net_admin(status: &str) -> bool {
    status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:\t"))
        .and_then(|hex| u64::from_str_radix(hex.trim(), 16).ok())
        .is_some_and(|caps| {
            let mask = 1_u64 << CAP_NET_ADMIN_BIT;
            caps & mask != 0
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_input() -> VmCiReadinessInput {
        let exe = std::env::current_exe().unwrap().display().to_string();
        VmCiReadinessInput {
            kernel_path: Some(exe.clone()),
            initrd_path: Some(exe.clone()),
            toplevel_path: Some(exe),
            network_mode: "tap".to_string(),
            tap_helper_path: None,
            kvm_available: true,
            kvm_writable: true,
            tun_available: true,
            bridge_available: true,
            host_network_configured: true,
            has_net_admin: true,
        }
    }

    #[test]
    fn ready_host_passes() {
        assert!(check_readiness(&ready_input()).is_ok());
    }

    #[test]
    fn startup_readiness_requires_router_ticket_and_pool() {
        let node_log = "Iroh Router spawned\ncluster ticket written to file\nVM pool initialized";
        let serial_log = "worker registered with cluster\nephemeral CI worker ready - polling for jobs";

        let readiness = parse_startup_readiness(node_log, serial_log);

        assert!(readiness.ready());
        assert!(readiness.worker_ready);
    }

    #[test]
    fn startup_readiness_does_not_require_worker_before_cluster_init() {
        let node_log = "Iroh Router spawned\ncluster ticket written to file\nVM pool initialized";

        let readiness = parse_startup_readiness(node_log, "");

        assert!(readiness.ready());
        assert!(readiness.router_spawned);
        assert!(readiness.ticket_written);
        assert!(readiness.vm_pool_initialized);
        assert!(!readiness.worker_ready);
    }

    #[test]
    fn tap_mode_requires_net_admin_before_pipeline_trigger() {
        let mut input = ready_input();
        input.has_net_admin = false;

        let error = check_readiness(&input).unwrap_err();

        assert!(error.contains("VM-CI readiness failed before pipeline trigger"));
        assert!(error.contains("CAP_NET_ADMIN"));
    }

    #[test]
    fn tap_helper_mode_accepts_helper_without_net_admin() {
        let helper = std::env::current_exe().unwrap().display().to_string();
        let mut input = ready_input();
        input.network_mode = "tap-helper".to_string();
        input.tap_helper_path = Some(helper);
        input.has_net_admin = false;

        assert!(check_readiness(&input).is_ok());
    }

    #[test]
    fn tap_helper_mode_requires_host_vm_network_setup_marker() {
        let helper = std::env::current_exe().unwrap().display().to_string();
        let mut input = ready_input();
        input.network_mode = "tap-helper".to_string();
        input.tap_helper_path = Some(helper);
        input.host_network_configured = false;

        let error = check_readiness(&input).unwrap_err();

        assert!(error.contains("VM-CI host network/firewall marker"));
        assert!(error.contains("/tmp/aspen-ci-network-configured-v3"));
        assert!(error.contains("setup-ci-network"));
    }

    #[test]
    fn tap_helper_mode_requires_bridge() {
        let helper = std::env::current_exe().unwrap().display().to_string();
        let mut input = ready_input();
        input.network_mode = "tap-helper".to_string();
        input.tap_helper_path = Some(helper);
        input.bridge_available = false;

        let error = check_readiness(&input).unwrap_err();

        assert!(error.contains("VM-CI bridge aspen-ci-br0 is missing"));
    }

    #[test]
    fn isolated_mode_skips_host_vm_network_setup() {
        let mut input = ready_input();
        input.network_mode = "isolated".to_string();
        input.bridge_available = false;
        input.host_network_configured = false;
        input.has_net_admin = false;

        assert!(check_readiness(&input).is_ok());
    }

    #[test]
    fn tap_helper_mode_requires_executable_helper() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("helper");
        std::fs::write(
            &helper,
            b"#!/bin/sh
",
        )
        .unwrap();
        #[cfg(unix)]
        {
            let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
            permissions.set_mode(0o644);
            std::fs::set_permissions(&helper, permissions).unwrap();
        }

        let mut input = ready_input();
        input.network_mode = "tap-helper".to_string();
        input.tap_helper_path = Some(helper.display().to_string());

        let error = check_readiness(&input).unwrap_err();
        assert!(error.contains("ASPEN_CI_TAP_HELPER_PATH is not executable"));
    }

    #[test]
    fn missing_vm_image_paths_are_reported() {
        let mut input = ready_input();
        input.kernel_path = None;
        input.initrd_path = Some("/definitely/missing/aspen-initrd".to_string());

        let error = check_readiness(&input).unwrap_err();

        assert!(error.contains("ASPEN_CI_KERNEL_PATH is not set"));
        assert!(error.contains("ASPEN_CI_INITRD_PATH does not exist"));
    }

    #[test]
    fn parses_effective_capabilities() {
        assert!(parse_cap_eff_has_net_admin("Name:\ttest\nCapEff:\t0000000000001000\n"));
        assert!(!parse_cap_eff_has_net_admin("Name:\ttest\nCapEff:\t0000000000000000\n"));
    }
}
