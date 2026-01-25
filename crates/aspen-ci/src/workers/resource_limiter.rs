//! Resource limiter for CI job isolation using cgroups v2.
//!
//! This module provides cgroup-based resource isolation for CI jobs to prevent:
//! - Memory exhaustion from test processes
//! - CPU starvation of Raft consensus
//! - Fork bombs and runaway parallelism
//!
//! # Architecture
//!
//! Each CI job gets its own cgroup with:
//! - `memory.max`: Hard memory limit (4 GB default)
//! - `memory.high`: Soft limit for throttling (3 GB default)
//! - `pids.max`: Maximum processes (4096 default)
//! - `cpu.weight`: Relative CPU priority (50 default, lower than system)
//!
//! # Tiger Style
//!
//! - Fixed limits from `aspen_constants`
//! - Fail-fast on cgroup setup errors
//! - Bounded resource allocation per job
//! - Cleanup on job completion

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use snafu::ResultExt;
use snafu::Snafu;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use aspen_constants::CI_JOB_CPU_WEIGHT;
use aspen_constants::MAX_CI_JOB_MEMORY_BYTES;
use aspen_constants::MAX_CI_JOB_MEMORY_HIGH_BYTES;
use aspen_constants::MAX_CI_JOB_PIDS;

/// Errors that can occur during resource limiting.
#[derive(Debug, Snafu)]
pub enum ResourceLimiterError {
    /// Failed to create cgroup directory.
    #[snafu(display("failed to create cgroup at {}: {}", path.display(), source))]
    CreateCgroup {
        /// Path where cgroup creation was attempted.
        path: PathBuf,
        /// IO error encountered.
        source: io::Error,
    },

    /// Failed to write cgroup controller setting.
    #[snafu(display("failed to set {} to {}: {}", controller, value, source))]
    SetController {
        /// Name of the cgroup controller file.
        controller: String,
        /// Value that was being written.
        value: String,
        /// IO error encountered.
        source: io::Error,
    },

    /// Failed to add process to cgroup.
    #[snafu(display("failed to add PID {} to cgroup: {}", pid, source))]
    AddProcess {
        /// Process ID that was being added.
        pid: u32,
        /// IO error encountered.
        source: io::Error,
    },

    /// Failed to remove cgroup.
    #[snafu(display("failed to remove cgroup at {}: {}", path.display(), source))]
    RemoveCgroup {
        /// Path of the cgroup being removed.
        path: PathBuf,
        /// IO error encountered.
        source: io::Error,
    },

    /// Cgroups v2 not available.
    #[snafu(display("cgroups v2 not available at /sys/fs/cgroup"))]
    CgroupsNotAvailable,

    /// Invalid job ID for cgroup name.
    #[snafu(display("invalid job ID for cgroup name: {}", job_id))]
    InvalidJobId {
        /// The invalid job ID.
        job_id: String,
    },
}

type Result<T> = std::result::Result<T, ResourceLimiterError>;

/// Configuration for CI job resource limits.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (hard limit).
    pub memory_max_bytes: u64,
    /// High memory threshold for throttling (soft limit).
    pub memory_high_bytes: u64,
    /// CPU weight (1-10000, default 100).
    pub cpu_weight: u32,
    /// Maximum number of PIDs.
    pub pids_max: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_max_bytes: MAX_CI_JOB_MEMORY_BYTES,
            memory_high_bytes: MAX_CI_JOB_MEMORY_HIGH_BYTES,
            cpu_weight: CI_JOB_CPU_WEIGHT,
            pids_max: MAX_CI_JOB_PIDS,
        }
    }
}

/// Cgroup-based resource limiter for CI jobs.
///
/// Creates a cgroup v2 hierarchy for the job and applies resource limits.
/// The cgroup is cleaned up when the limiter is dropped.
pub struct ResourceLimiter {
    /// Path to the cgroup directory.
    cgroup_path: PathBuf,
    /// Job ID for logging.
    job_id: String,
    /// Whether to clean up on drop.
    cleanup_on_drop: bool,
}

impl ResourceLimiter {
    /// Base path for aspen CI cgroups.
    const CGROUP_BASE: &'static str = "/sys/fs/cgroup/aspen-ci";

    /// Create a new resource limiter for a CI job.
    ///
    /// # Arguments
    ///
    /// * `job_id` - Unique job identifier (used as cgroup name)
    /// * `limits` - Resource limits to apply
    ///
    /// # Returns
    ///
    /// A new `ResourceLimiter` with the cgroup created and limits applied.
    pub fn create(job_id: &str, limits: &ResourceLimits) -> Result<Self> {
        // Validate job ID for safe path construction
        if job_id.is_empty() || job_id.contains('/') || job_id.contains('\0') {
            return Err(ResourceLimiterError::InvalidJobId {
                job_id: job_id.to_string(),
            });
        }

        // Check if cgroups v2 is available
        let cgroup_root = Path::new("/sys/fs/cgroup");
        if !cgroup_root.join("cgroup.controllers").exists() {
            warn!("cgroups v2 not available, resource limiting disabled");
            return Err(ResourceLimiterError::CgroupsNotAvailable);
        }

        // Create base directory for aspen-ci cgroups if needed
        let base_path = PathBuf::from(Self::CGROUP_BASE);
        if !base_path.exists() {
            fs::create_dir_all(&base_path).context(CreateCgroupSnafu {
                path: base_path.clone(),
            })?;

            // Enable controllers in base cgroup
            Self::enable_controllers(&base_path)?;
        }

        // Create job-specific cgroup
        let cgroup_path = base_path.join(job_id);
        fs::create_dir_all(&cgroup_path).context(CreateCgroupSnafu {
            path: cgroup_path.clone(),
        })?;

        info!(
            job_id = %job_id,
            cgroup_path = %cgroup_path.display(),
            memory_max_mb = limits.memory_max_bytes / (1024 * 1024),
            pids_max = limits.pids_max,
            "created cgroup for CI job"
        );

        let limiter = Self {
            cgroup_path: cgroup_path.clone(),
            job_id: job_id.to_string(),
            cleanup_on_drop: true,
        };

        // Apply resource limits
        limiter.apply_limits(limits)?;

        Ok(limiter)
    }

    /// Create a no-op limiter for when cgroups are unavailable.
    ///
    /// This allows CI jobs to run without resource limits on systems
    /// where cgroups v2 is not available.
    pub fn noop(job_id: &str) -> Self {
        Self {
            cgroup_path: PathBuf::new(),
            job_id: job_id.to_string(),
            cleanup_on_drop: false,
        }
    }

    /// Check if cgroups v2 is available on this system.
    pub fn is_available() -> bool {
        Path::new("/sys/fs/cgroup/cgroup.controllers").exists()
    }

    /// Enable required controllers in the parent cgroup.
    fn enable_controllers(path: &Path) -> Result<()> {
        // Enable memory, pids, and cpu controllers
        let subtree_control = path.join("cgroup.subtree_control");
        if subtree_control.exists() {
            fs::write(&subtree_control, "+memory +pids +cpu").context(SetControllerSnafu {
                controller: "subtree_control".to_string(),
                value: "+memory +pids +cpu".to_string(),
            })?;
        }
        Ok(())
    }

    /// Apply resource limits to the cgroup.
    fn apply_limits(&self, limits: &ResourceLimits) -> Result<()> {
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(()); // No-op limiter
        }

        // Set memory.max (hard limit)
        let memory_max_path = self.cgroup_path.join("memory.max");
        if memory_max_path.exists() {
            fs::write(&memory_max_path, limits.memory_max_bytes.to_string()).context(SetControllerSnafu {
                controller: "memory.max".to_string(),
                value: limits.memory_max_bytes.to_string(),
            })?;
            debug!(
                job_id = %self.job_id,
                memory_max_bytes = limits.memory_max_bytes,
                "set memory.max"
            );
        }

        // Set memory.high (soft limit for throttling)
        let memory_high_path = self.cgroup_path.join("memory.high");
        if memory_high_path.exists() {
            fs::write(&memory_high_path, limits.memory_high_bytes.to_string()).context(SetControllerSnafu {
                controller: "memory.high".to_string(),
                value: limits.memory_high_bytes.to_string(),
            })?;
            debug!(
                job_id = %self.job_id,
                memory_high_bytes = limits.memory_high_bytes,
                "set memory.high"
            );
        }

        // Set pids.max
        let pids_max_path = self.cgroup_path.join("pids.max");
        if pids_max_path.exists() {
            fs::write(&pids_max_path, limits.pids_max.to_string()).context(SetControllerSnafu {
                controller: "pids.max".to_string(),
                value: limits.pids_max.to_string(),
            })?;
            debug!(
                job_id = %self.job_id,
                pids_max = limits.pids_max,
                "set pids.max"
            );
        }

        // Set cpu.weight
        let cpu_weight_path = self.cgroup_path.join("cpu.weight");
        if cpu_weight_path.exists() {
            fs::write(&cpu_weight_path, limits.cpu_weight.to_string()).context(SetControllerSnafu {
                controller: "cpu.weight".to_string(),
                value: limits.cpu_weight.to_string(),
            })?;
            debug!(
                job_id = %self.job_id,
                cpu_weight = limits.cpu_weight,
                "set cpu.weight"
            );
        }

        Ok(())
    }

    /// Add a process to the cgroup.
    ///
    /// This should be called after forking but before exec to ensure
    /// the child process runs under the resource limits.
    pub fn add_process(&self, pid: u32) -> Result<()> {
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(()); // No-op limiter
        }

        let procs_path = self.cgroup_path.join("cgroup.procs");
        fs::write(&procs_path, pid.to_string()).context(AddProcessSnafu { pid })?;

        debug!(
            job_id = %self.job_id,
            pid = pid,
            "added process to cgroup"
        );

        Ok(())
    }

    /// Get the cgroup path for this limiter.
    pub fn cgroup_path(&self) -> &Path {
        &self.cgroup_path
    }

    /// Get current memory usage in bytes.
    pub fn memory_current(&self) -> Option<u64> {
        if self.cgroup_path.as_os_str().is_empty() {
            return None;
        }

        let memory_current_path = self.cgroup_path.join("memory.current");
        fs::read_to_string(memory_current_path).ok().and_then(|s| s.trim().parse().ok())
    }

    /// Get current PID count.
    pub fn pids_current(&self) -> Option<u32> {
        if self.cgroup_path.as_os_str().is_empty() {
            return None;
        }

        let pids_current_path = self.cgroup_path.join("pids.current");
        fs::read_to_string(pids_current_path).ok().and_then(|s| s.trim().parse().ok())
    }

    /// Cleanup the cgroup.
    ///
    /// This is called automatically on drop, but can be called manually
    /// for explicit cleanup.
    pub fn cleanup(&self) -> Result<()> {
        if self.cgroup_path.as_os_str().is_empty() {
            return Ok(()); // No-op limiter
        }

        // First, kill all processes in the cgroup
        let procs_path = self.cgroup_path.join("cgroup.procs");
        if let Ok(procs) = fs::read_to_string(&procs_path) {
            for line in procs.lines() {
                if let Ok(pid) = line.parse::<u32>() {
                    // SIGKILL the process via /proc filesystem
                    // This avoids needing the libc crate
                    let _ = std::process::Command::new("kill").args(["-9", &pid.to_string()]).output();
                }
            }
        }

        // Wait a bit for processes to die
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Remove the cgroup directory
        match fs::remove_dir(&self.cgroup_path) {
            Ok(()) => {
                info!(
                    job_id = %self.job_id,
                    cgroup_path = %self.cgroup_path.display(),
                    "removed cgroup for CI job"
                );
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Already removed
                Ok(())
            }
            Err(e) if e.raw_os_error() == Some(16) => {
                // EBUSY = 16 on Linux: Cgroup still has processes, try again
                warn!(
                    job_id = %self.job_id,
                    "cgroup still has processes, deferring cleanup"
                );
                Ok(())
            }
            Err(e) => Err(ResourceLimiterError::RemoveCgroup {
                path: self.cgroup_path.clone(),
                source: e,
            }),
        }
    }

    /// Disable cleanup on drop (for manual cleanup).
    pub fn disable_cleanup_on_drop(&mut self) {
        self.cleanup_on_drop = false;
    }
}

impl Drop for ResourceLimiter {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            if let Err(e) = self.cleanup() {
                error!(
                    job_id = %self.job_id,
                    error = %e,
                    "failed to cleanup cgroup on drop"
                );
            }
        }
    }
}

/// Create a resource limiter for a CI job, falling back to no-op if unavailable.
///
/// This is the recommended way to create a limiter as it handles systems
/// without cgroups v2 gracefully.
pub fn create_limiter(job_id: &str, limits: &ResourceLimits) -> ResourceLimiter {
    match ResourceLimiter::create(job_id, limits) {
        Ok(limiter) => limiter,
        Err(e) => {
            warn!(
                job_id = %job_id,
                error = %e,
                "failed to create cgroup limiter, running without resource limits"
            );
            ResourceLimiter::noop(job_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_max_bytes, MAX_CI_JOB_MEMORY_BYTES);
        assert_eq!(limits.memory_high_bytes, MAX_CI_JOB_MEMORY_HIGH_BYTES);
        assert_eq!(limits.cpu_weight, CI_JOB_CPU_WEIGHT);
        assert_eq!(limits.pids_max, MAX_CI_JOB_PIDS);
    }

    #[test]
    fn test_invalid_job_id() {
        let limits = ResourceLimits::default();

        // Empty job ID
        assert!(ResourceLimiter::create("", &limits).is_err());

        // Job ID with path separator
        assert!(ResourceLimiter::create("foo/bar", &limits).is_err());

        // Job ID with null byte
        assert!(ResourceLimiter::create("foo\0bar", &limits).is_err());
    }

    #[test]
    fn test_noop_limiter() {
        let limiter = ResourceLimiter::noop("test-job");
        assert!(limiter.cgroup_path().as_os_str().is_empty());
        assert!(limiter.memory_current().is_none());
        assert!(limiter.pids_current().is_none());
        assert!(limiter.add_process(12345).is_ok());
        assert!(limiter.cleanup().is_ok());
    }

    #[test]
    fn test_is_available() {
        // This test just verifies the function doesn't panic
        let _ = ResourceLimiter::is_available();
    }
}
