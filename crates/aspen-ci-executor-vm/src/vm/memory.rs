//! Per-VM memory tracking via /proc/{pid}/smaps_rollup and /proc/{pid}/status.
//!
//! Estimates dirty (private) memory per restored VM by reading RSS from
//! the Cloud Hypervisor process. For COW-restored VMs, RSS tracks only
//! the pages actually touched (dirtied) — shared pages from the snapshot
//! backing file are NOT counted as private.

use std::path::Path;

use tracing::debug;
use tracing::warn;

use super::ManagedCiVm;

/// Memory usage for a single VM process.
#[derive(Debug, Clone, Copy, Default)]
pub struct VmMemoryUsage {
    /// Process ID of the Cloud Hypervisor process.
    pub pid: u32,
    /// Resident Set Size in bytes (total pages in physical memory).
    pub rss_bytes: u64,
    /// Private dirty pages in bytes (COW-dirtied memory, not shared).
    /// Falls back to RSS if smaps_rollup is unavailable.
    pub private_dirty_bytes: u64,
    /// Shared clean pages in bytes (from snapshot backing file, COW-shared).
    pub shared_clean_bytes: u64,
}

impl ManagedCiVm {
    /// Get the Cloud Hypervisor process PID, if running.
    pub async fn get_pid(&self) -> Option<u32> {
        let guard = self.process.read().await;
        guard.as_ref().and_then(|child| child.id())
    }

    /// Estimate memory usage for this VM's Cloud Hypervisor process.
    ///
    /// Reads `/proc/{pid}/smaps_rollup` for accurate private dirty tracking.
    /// Falls back to `/proc/{pid}/status` (VmRSS) if smaps_rollup is
    /// unavailable (requires `CONFIG_PROC_PAGE_MONITOR`).
    pub async fn estimate_memory_usage(&self) -> Option<VmMemoryUsage> {
        let pid = self.get_pid().await?;
        let usage = read_process_memory(pid);

        if let Some(ref u) = usage {
            debug!(
                vm_id = %self.id,
                pid = pid,
                rss_mb = u.rss_bytes / (1024 * 1024),
                private_dirty_mb = u.private_dirty_bytes / (1024 * 1024),
                shared_clean_mb = u.shared_clean_bytes / (1024 * 1024),
                "VM memory usage"
            );
        }

        usage
    }
}

/// Read memory usage for a process from procfs.
///
/// Tries `/proc/{pid}/smaps_rollup` first (accurate per-mapping breakdown),
/// falls back to `/proc/{pid}/status` (RSS only).
pub fn read_process_memory(pid: u32) -> Option<VmMemoryUsage> {
    // Try smaps_rollup first (aggregated smaps, lower overhead than full smaps)
    let rollup_path = format!("/proc/{pid}/smaps_rollup");
    if let Some(usage) = parse_smaps_rollup(Path::new(&rollup_path), pid) {
        return Some(usage);
    }

    // Fallback: read RSS from /proc/{pid}/status
    let status_path = format!("/proc/{pid}/status");
    parse_status_rss(Path::new(&status_path), pid)
}

/// Parse `/proc/{pid}/smaps_rollup` for memory breakdown.
///
/// smaps_rollup provides aggregated memory stats:
/// - Rss: total resident pages
/// - Private_Dirty: COW-dirtied pages (per-fork unique memory)
/// - Shared_Clean: shared pages from backing file (COW-shared base)
fn parse_smaps_rollup(path: &Path, pid: u32) -> Option<VmMemoryUsage> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let mut rss_kb: u64 = 0;
    let mut private_dirty_kb: u64 = 0;
    let mut shared_clean_kb: u64 = 0;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("Rss:") {
            rss_kb = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("Private_Dirty:") {
            private_dirty_kb = parse_kb_value(val);
        } else if let Some(val) = line.strip_prefix("Shared_Clean:") {
            shared_clean_kb = parse_kb_value(val);
        }
    }

    Some(VmMemoryUsage {
        pid,
        rss_bytes: rss_kb * 1024,
        private_dirty_bytes: private_dirty_kb * 1024,
        shared_clean_bytes: shared_clean_kb * 1024,
    })
}

/// Parse RSS from `/proc/{pid}/status` as a fallback.
fn parse_status_rss(path: &Path, pid: u32) -> Option<VmMemoryUsage> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!(pid = pid, error = %e, "failed to read /proc/{pid}/status");
            return None;
        }
    };

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("VmRSS:") {
            let rss_kb = parse_kb_value(val);
            let rss_bytes = rss_kb * 1024;
            return Some(VmMemoryUsage {
                pid,
                rss_bytes,
                // Without smaps, use RSS as a conservative upper bound for private dirty
                private_dirty_bytes: rss_bytes,
                shared_clean_bytes: 0,
            });
        }
    }

    warn!(pid = pid, "VmRSS not found in /proc/{pid}/status");
    None
}

/// Parse a value like "  12345 kB" from procfs.
fn parse_kb_value(val: &str) -> u64 {
    val.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0)
}

/// Estimate total memory usage across multiple VM processes.
///
/// Returns (total_rss_bytes, total_private_dirty_bytes).
pub fn estimate_total_vm_memory(pids: &[u32]) -> (u64, u64) {
    let mut total_rss: u64 = 0;
    let mut total_dirty: u64 = 0;

    for &pid in pids {
        if let Some(usage) = read_process_memory(pid) {
            total_rss = total_rss.saturating_add(usage.rss_bytes);
            total_dirty = total_dirty.saturating_add(usage.private_dirty_bytes);
        }
    }

    (total_rss, total_dirty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kb_value() {
        assert_eq!(parse_kb_value("  12345 kB"), 12345);
        assert_eq!(parse_kb_value("12345 kB"), 12345);
        assert_eq!(parse_kb_value("0 kB"), 0);
        assert_eq!(parse_kb_value(""), 0);
        assert_eq!(parse_kb_value("  "), 0);
    }

    #[test]
    fn test_read_process_memory_for_self() {
        // Read our own process memory — should work on any Linux system
        let pid = std::process::id();
        let usage = read_process_memory(pid);

        // Should succeed for our own process
        assert!(usage.is_some(), "should be able to read own process memory");
        let usage = usage.unwrap();
        assert_eq!(usage.pid, pid);
        assert!(usage.rss_bytes > 0, "RSS should be non-zero for running process");
    }

    #[test]
    fn test_read_process_memory_for_nonexistent_pid() {
        // PID 4294967295 (u32::MAX) should not exist
        let usage = read_process_memory(u32::MAX);
        assert!(usage.is_none());
    }

    #[test]
    fn test_estimate_total_vm_memory() {
        let self_pid = std::process::id();
        let (total_rss, total_dirty) = estimate_total_vm_memory(&[self_pid]);
        assert!(total_rss > 0);
        assert!(total_dirty > 0);
    }

    #[test]
    fn test_estimate_total_vm_memory_empty() {
        let (total_rss, total_dirty) = estimate_total_vm_memory(&[]);
        assert_eq!(total_rss, 0);
        assert_eq!(total_dirty, 0);
    }

    #[test]
    fn test_estimate_total_vm_memory_with_nonexistent() {
        // Mix of valid and invalid PIDs — invalid ones should be skipped
        let self_pid = std::process::id();
        let (total_rss, _) = estimate_total_vm_memory(&[self_pid, u32::MAX]);
        // Should still have data from our own process
        assert!(total_rss > 0);
    }

    #[test]
    fn test_vm_memory_usage_default() {
        let usage = VmMemoryUsage::default();
        assert_eq!(usage.pid, 0);
        assert_eq!(usage.rss_bytes, 0);
        assert_eq!(usage.private_dirty_bytes, 0);
        assert_eq!(usage.shared_clean_bytes, 0);
    }
}
