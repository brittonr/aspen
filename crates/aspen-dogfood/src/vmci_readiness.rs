//! VM-CI readiness checks for dogfood runs.
//!
//! These checks deliberately run before CI jobs are triggered so a host that
//! cannot create VM networking fails with a receipt instead of hanging until the
//! pipeline timeout.

use std::path::Path;

use crate::error::DogfoodError;
use crate::error::DogfoodResult;

const CAP_NET_ADMIN_BIT: u32 = 12;
const DEFAULT_NETWORK_MODE: &str = "tap";

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
            has_net_admin: current_process_has_net_admin(),
        }
    }
}

pub fn check_current_environment() -> DogfoodResult<()> {
    check_readiness(&VmCiReadinessInput::from_host()).map_err(|reason| DogfoodError::VmCiReadiness { reason })
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
            require_existing_path(&mut failures, "ASPEN_CI_TAP_HELPER_PATH", input.tap_helper_path.as_deref());
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
            has_net_admin: true,
        }
    }

    #[test]
    fn ready_host_passes() {
        assert!(check_readiness(&ready_input()).is_ok());
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
