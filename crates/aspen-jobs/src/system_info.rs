//! System information collection for worker health reporting.
//!
//! Reads Linux PSI (Pressure Stall Information) from /proc/pressure/
//! and disk free space via statvfs. Falls back gracefully on non-Linux
//! systems.

use crate::verified::pressure::parse_psi_avg10;

/// PSI pressure readings for CPU, memory, and I/O.
#[derive(Debug, Clone, Copy, Default)]
pub struct PressureReadings {
    /// CPU pressure average over 10 seconds (0.0-100.0).
    pub cpu_avg10: f32,
    /// Memory pressure average over 10 seconds (0.0-100.0).
    pub memory_avg10: f32,
    /// I/O pressure average over 10 seconds (0.0-100.0).
    pub io_avg10: f32,
}

/// Disk free space readings.
#[derive(Debug, Clone, Copy)]
pub struct DiskFreeReadings {
    /// Build directory free space percentage (0.0-100.0).
    pub build_dir_free_pct: f64,
    /// Store directory free space percentage (0.0-100.0).
    pub store_dir_free_pct: f64,
}

impl Default for DiskFreeReadings {
    fn default() -> Self {
        Self {
            build_dir_free_pct: 100.0,
            store_dir_free_pct: 100.0,
        }
    }
}

/// Read PSI metrics from /proc/pressure/.
/// Returns default (0.0) values on non-Linux or if files don't exist.
pub fn read_psi_pressure() -> PressureReadings {
    PressureReadings {
        cpu_avg10: read_psi_file("/proc/pressure/cpu"),
        memory_avg10: read_psi_file("/proc/pressure/memory"),
        io_avg10: read_psi_file("/proc/pressure/io"),
    }
}

fn read_psi_file(path: &str) -> f32 {
    match std::fs::read_to_string(path) {
        Ok(content) => parse_psi_avg10(&content),
        Err(_) => 0.0,
    }
}

/// Read disk free percentages for build and store directories.
/// Returns default (100.0) values on failure.
pub fn read_disk_free(build_dir: Option<&std::path::Path>, store_dir: Option<&std::path::Path>) -> DiskFreeReadings {
    DiskFreeReadings {
        build_dir_free_pct: build_dir.map_or(100.0, disk_free_pct),
        store_dir_free_pct: store_dir.map_or(100.0, disk_free_pct),
    }
}

#[cfg(unix)]
fn disk_free_pct(path: &std::path::Path) -> f64 {
    nix_statvfs(path).unwrap_or(100.0)
}

#[cfg(not(unix))]
fn disk_free_pct(_path: &std::path::Path) -> f64 {
    100.0
}

#[cfg(unix)]
fn nix_statvfs(path: &std::path::Path) -> Option<f64> {
    // Use libc::statvfs directly
    use std::ffi::CString;
    let c_path = CString::new(path.to_str()?).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
    if result != 0 {
        return None;
    }
    if stat.f_blocks == 0 {
        return Some(0.0);
    }
    let free_pct = (stat.f_bavail as f64 / stat.f_blocks as f64) * 100.0;
    Some(free_pct.clamp(0.0, 100.0))
}
