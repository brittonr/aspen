//! Runtime detection of patchbay prerequisites.
//!
//! Checks for unprivileged user namespace support and required tools (nft, tc).
//! Tests should use the `skip_unless_patchbay!()` macro to gracefully skip
//! when the environment doesn't support patchbay.

use std::process::Command;
use std::sync::OnceLock;

/// Cached result of patchbay availability check.
static PATCHBAY_AVAILABLE: OnceLock<Result<(), String>> = OnceLock::new();

/// Check if the current environment supports patchbay.
///
/// Verifies:
/// 1. Unprivileged user namespaces are enabled (sysctl or trial clone)
/// 2. `nft` is available in PATH
/// 3. `tc` is available in PATH
///
/// Results are cached after the first call.
pub fn patchbay_available() -> bool {
    PATCHBAY_AVAILABLE.get_or_init(check_patchbay_prerequisites).is_ok()
}

/// Returns the reason patchbay is unavailable, or None if available.
pub fn patchbay_unavailable_reason() -> Option<String> {
    PATCHBAY_AVAILABLE.get_or_init(check_patchbay_prerequisites).as_ref().err().cloned()
}

fn check_patchbay_prerequisites() -> Result<(), String> {
    // Check 1: unprivileged user namespaces
    if !userns_available() {
        return Err("unprivileged user namespaces not available \
             (set kernel.unprivileged_userns_clone=1 or \
             kernel.apparmor_restrict_unprivileged_userns=0)"
            .to_string());
    }

    // Check 2: nft in PATH
    if !tool_available("nft") {
        return Err("nft not found in PATH (install nftables)".to_string());
    }

    // Check 3: tc in PATH
    if !tool_available("tc") {
        return Err("tc not found in PATH (install iproute2)".to_string());
    }

    Ok(())
}

/// Check if unprivileged user namespaces are supported.
///
/// Tries the sysctl first, then falls back to a trial unshare(2).
fn userns_available() -> bool {
    // Try reading the sysctl
    if let Ok(val) = std::fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
        && val.trim() == "0"
    {
        return false;
        // "1" or missing file (sysctl doesn't exist on all kernels) → continue
    }

    // Check AppArmor restriction (Ubuntu 24.04+)
    if let Ok(val) = std::fs::read_to_string("/proc/sys/kernel/apparmor_restrict_unprivileged_userns")
        && val.trim() == "1"
    {
        return false;
    }

    // Trial: attempt to create a user namespace
    match Command::new("unshare").args(["--user", "--map-root-user", "true"]).output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Check if a tool is available in PATH.
fn tool_available(name: &str) -> bool {
    Command::new("which").arg(name).output().map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_available_finds_true() {
        // `true` should always exist
        assert!(tool_available("true"));
    }

    #[test]
    fn test_tool_available_rejects_nonexistent() {
        assert!(!tool_available("this-tool-does-not-exist-aspen-test"));
    }

    #[test]
    fn test_patchbay_available_returns_consistent() {
        let first = patchbay_available();
        let second = patchbay_available();
        assert_eq!(first, second, "cached result should be consistent");
    }
}
