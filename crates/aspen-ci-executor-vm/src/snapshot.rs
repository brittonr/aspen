//! Golden snapshot management for fast VM restore.
//!
//! A golden snapshot captures the state of a fully-booted VM at the "idle and
//! cluster-joined" point. Subsequent VMs restore from this snapshot instead of
//! cold-booting, skipping kernel boot, systemd init, and cluster join.

use std::path::Path;
use std::path::PathBuf;

use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::CloudHypervisorWorkerConfig;
use crate::error::CloudHypervisorError;
use crate::error::Result;
use crate::vm::ManagedCiVm;

/// A validated golden VM snapshot on disk.
///
/// The snapshot directory contains:
/// - `state.json` — Cloud Hypervisor serialized VM state
/// - `memory` — guest RAM backing file (sparse, mmap'd COW by forks)
/// - `ticket.txt` — cluster ticket at snapshot time (for staleness detection)
#[derive(Debug, Clone)]
pub struct GoldenSnapshot {
    /// Path to the snapshot directory.
    pub dir: PathBuf,
    /// Path to the memory backing file.
    pub memory_path: PathBuf,
    /// Path to the VM state file.
    pub state_path: PathBuf,
    /// Path to the ticket file.
    pub ticket_path: PathBuf,
}

impl GoldenSnapshot {
    /// Create a `GoldenSnapshot` reference from config paths.
    ///
    /// Does NOT validate the snapshot — call `validate()` separately.
    pub fn from_config(config: &CloudHypervisorWorkerConfig) -> Self {
        Self {
            dir: config.snapshot_dir(),
            memory_path: config.snapshot_memory_path(),
            state_path: config.snapshot_state_path(),
            ticket_path: config.snapshot_ticket_path(),
        }
    }

    /// Validate that the snapshot is usable.
    ///
    /// Checks:
    /// 1. Snapshot directory exists
    /// 2. Memory backing file exists
    /// 3. Embedded cluster ticket matches the current ticket
    ///
    /// Returns `Ok(())` if valid, `Err` with reason if not.
    pub fn validate(&self, current_ticket: &str) -> Result<()> {
        // Check directory exists
        if !self.dir.exists() {
            return Err(CloudHypervisorError::RestoreFailed {
                path: self.dir.clone(),
                reason: "snapshot directory does not exist".to_string(),
            });
        }

        // Check memory file exists
        if !self.memory_path.exists() {
            return Err(CloudHypervisorError::RestoreFailed {
                path: self.memory_path.clone(),
                reason: "memory backing file does not exist".to_string(),
            });
        }

        // Check ticket matches
        let stored_ticket =
            std::fs::read_to_string(&self.ticket_path).map_err(|e| CloudHypervisorError::RestoreFailed {
                path: self.ticket_path.clone(),
                reason: format!("failed to read ticket file: {e}"),
            })?;

        let stored_ticket = stored_ticket.trim();
        if stored_ticket != current_ticket {
            return Err(CloudHypervisorError::RestoreFailed {
                path: self.dir.clone(),
                reason: format!(
                    "stale cluster ticket (snapshot has '{}...', current is '{}...')",
                    &stored_ticket[..stored_ticket.len().min(20)],
                    &current_ticket[..current_ticket.len().min(20)],
                ),
            });
        }

        debug!(dir = %self.dir.display(), "golden snapshot validated");
        Ok(())
    }

    /// Create a golden snapshot from a running VM.
    ///
    /// The VM must be in the Idle state. This method:
    /// 1. Pauses the VM
    /// 2. Creates the snapshot via Cloud Hypervisor API
    /// 3. Writes the cluster ticket to the snapshot directory
    /// 4. Resumes the VM
    pub async fn create(vm: &ManagedCiVm, config: &CloudHypervisorWorkerConfig) -> Result<Self> {
        let snapshot = Self::from_config(config);

        let current_ticket = config.get_cluster_ticket().ok_or_else(|| CloudHypervisorError::SnapshotFailed {
            vm_id: vm.id.clone(),
            reason: "no cluster ticket configured".to_string(),
        })?;

        // Create snapshot directory
        tokio::fs::create_dir_all(&snapshot.dir).await.map_err(|e| CloudHypervisorError::SnapshotFailed {
            vm_id: vm.id.clone(),
            reason: format!("failed to create snapshot directory: {e}"),
        })?;

        info!(vm_id = %vm.id, dir = %snapshot.dir.display(), "creating golden snapshot");

        // Pause → snapshot → write ticket → resume
        vm.pause().await?;

        let snapshot_result = async {
            // Create snapshot via API
            vm.snapshot(&snapshot.dir).await?;

            // Write ticket file
            tokio::fs::write(&snapshot.ticket_path, current_ticket.trim()).await.map_err(|e| {
                CloudHypervisorError::SnapshotFailed {
                    vm_id: vm.id.clone(),
                    reason: format!("failed to write ticket file: {e}"),
                }
            })?;

            Ok::<(), CloudHypervisorError>(())
        }
        .await;

        // Always resume, even if snapshot failed
        if let Err(e) = vm.resume().await {
            warn!(vm_id = %vm.id, error = %e, "failed to resume VM after snapshot");
        }

        snapshot_result?;

        // Verify memory backing file is sparse (non-fatal — just logs)
        match snapshot.verify_sparse_memory() {
            Ok(info) if !info.is_sparse => {
                warn!(
                    vm_id = %vm.id,
                    apparent_mb = info.apparent_bytes / (1024 * 1024),
                    disk_mb = info.disk_bytes / (1024 * 1024),
                    "golden snapshot memory is not sparse — each restore will duplicate full memory"
                );
            }
            Err(e) => {
                warn!(vm_id = %vm.id, error = %e, "could not verify memory file sparseness");
            }
            Ok(_) => {} // Sparse — already logged inside verify_sparse_memory()
        }

        info!(
            vm_id = %vm.id,
            dir = %snapshot.dir.display(),
            "golden snapshot created"
        );

        Ok(snapshot)
    }

    /// Invalidate (delete) this golden snapshot.
    pub async fn invalidate(&self, reason: &str) -> Result<()> {
        warn!(
            dir = %self.dir.display(),
            reason = %reason,
            "invalidating golden snapshot"
        );

        if self.dir.exists() {
            tokio::fs::remove_dir_all(&self.dir).await.map_err(|e| CloudHypervisorError::RestoreFailed {
                path: self.dir.clone(),
                reason: format!("failed to delete snapshot directory: {e}"),
            })?;
        }

        info!(dir = %self.dir.display(), "golden snapshot invalidated");
        Ok(())
    }

    /// Check if the snapshot directory exists on disk.
    pub fn exists(&self) -> bool {
        self.dir.exists()
    }

    /// Get the `file://` URL for Cloud Hypervisor restore API.
    pub fn source_url(&self) -> String {
        format!("file://{}", self.dir.display())
    }

    /// Verify the memory backing file is sparse.
    ///
    /// Compares on-disk block usage against apparent file size. A sparse file
    /// uses fewer blocks than its apparent size because zero-filled regions
    /// are not allocated on disk. Returns `(apparent_bytes, disk_bytes)`.
    ///
    /// Logs a warning if the file is NOT sparse (disk usage >= 90% of apparent
    /// size), since COW performance depends on the kernel page cache sharing
    /// the backing file's unwritten regions.
    pub fn verify_sparse_memory(&self) -> Result<SparseFileInfo> {
        verify_sparse_file(&self.memory_path)
    }
}

/// Information about a file's sparse/dense allocation.
#[derive(Debug, Clone, Copy)]
pub struct SparseFileInfo {
    /// Apparent file size in bytes (what `ls -l` shows).
    pub apparent_bytes: u64,
    /// Actual disk usage in bytes (allocated blocks × 512).
    pub disk_bytes: u64,
    /// Whether the file appears sparse (disk < 90% of apparent).
    pub is_sparse: bool,
}

/// Verify that a file is sparse by comparing stat block count to file size.
///
/// On Linux, `st_blocks` reports 512-byte block count regardless of filesystem
/// block size. A sparse file has `st_blocks * 512 < st_size`.
pub fn verify_sparse_file(path: &Path) -> Result<SparseFileInfo> {
    use std::os::unix::fs::MetadataExt;

    let metadata = std::fs::metadata(path).map_err(|e| CloudHypervisorError::RestoreFailed {
        path: path.to_path_buf(),
        reason: format!("failed to stat memory backing file: {e}"),
    })?;

    let apparent_bytes = metadata.len();
    // st_blocks is in 512-byte units on Linux
    let disk_bytes = metadata.blocks() * 512;
    // Consider sparse if disk usage is under 90% of apparent size
    let is_sparse = apparent_bytes > 0 && disk_bytes < (apparent_bytes * 9 / 10);

    let info = SparseFileInfo {
        apparent_bytes,
        disk_bytes,
        is_sparse,
    };

    if is_sparse {
        info!(
            path = %path.display(),
            apparent_mb = apparent_bytes / (1024 * 1024),
            disk_mb = disk_bytes / (1024 * 1024),
            ratio_pct = (disk_bytes * 100).checked_div(apparent_bytes).unwrap_or(0),
            "memory backing file is sparse"
        );
    } else {
        warn!(
            path = %path.display(),
            apparent_mb = apparent_bytes / (1024 * 1024),
            disk_mb = disk_bytes / (1024 * 1024),
            "memory backing file is NOT sparse — COW savings may be reduced"
        );
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn setup_snapshot_dir(dir: &std::path::Path, ticket: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("memory"), b"fake-memory-data").unwrap();
        fs::write(dir.join("ticket.txt"), ticket).unwrap();
    }

    #[test]
    fn test_validate_valid_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");
        let ticket = "test-ticket-abc123";

        setup_snapshot_dir(&dir, ticket);

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: dir.join("memory"),
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        assert!(snapshot.validate(ticket).is_ok());
    }

    #[test]
    fn test_validate_missing_directory() {
        let snapshot = GoldenSnapshot {
            dir: PathBuf::from("/nonexistent/path"),
            memory_path: PathBuf::from("/nonexistent/path/memory"),
            state_path: PathBuf::from("/nonexistent/path/state.json"),
            ticket_path: PathBuf::from("/nonexistent/path/ticket.txt"),
        };

        let err = snapshot.validate("any-ticket").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("does not exist"), "expected 'does not exist', got: {msg}");
    }

    #[test]
    fn test_validate_missing_memory_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("ticket.txt"), "test-ticket").unwrap();

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: dir.join("memory"),
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        let err = snapshot.validate("test-ticket").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("memory backing file"), "expected memory error, got: {msg}");
    }

    #[test]
    fn test_validate_stale_ticket() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");

        setup_snapshot_dir(&dir, "old-ticket-123");

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: dir.join("memory"),
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        let err = snapshot.validate("new-ticket-456").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("stale cluster ticket"), "expected stale ticket error, got: {msg}");
    }

    #[test]
    fn test_exists_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");
        fs::create_dir_all(&dir).unwrap();

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: dir.join("memory"),
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        assert!(snapshot.exists());
    }

    #[test]
    fn test_exists_when_absent() {
        let snapshot = GoldenSnapshot {
            dir: PathBuf::from("/nonexistent/snapshot"),
            memory_path: PathBuf::from("/nonexistent/snapshot/memory"),
            state_path: PathBuf::from("/nonexistent/snapshot/state.json"),
            ticket_path: PathBuf::from("/nonexistent/snapshot/ticket.txt"),
        };

        assert!(!snapshot.exists());
    }

    #[test]
    fn test_source_url() {
        let snapshot = GoldenSnapshot {
            dir: PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden"),
            memory_path: PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/memory"),
            state_path: PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/state.json"),
            ticket_path: PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/ticket.txt"),
        };

        assert_eq!(snapshot.source_url(), "file:///var/lib/aspen/ci/vms/snapshots/golden");
    }

    #[tokio::test]
    async fn test_invalidate_removes_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");
        setup_snapshot_dir(&dir, "test-ticket");

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: dir.join("memory"),
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        assert!(dir.exists());
        snapshot.invalidate("test reason").await.unwrap();
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn test_invalidate_nonexistent_is_ok() {
        let snapshot = GoldenSnapshot {
            dir: PathBuf::from("/nonexistent/snapshot"),
            memory_path: PathBuf::from("/nonexistent/snapshot/memory"),
            state_path: PathBuf::from("/nonexistent/snapshot/state.json"),
            ticket_path: PathBuf::from("/nonexistent/snapshot/ticket.txt"),
        };

        assert!(snapshot.invalidate("test").await.is_ok());
    }

    #[test]
    fn test_from_config() {
        let config = CloudHypervisorWorkerConfig {
            state_dir: PathBuf::from("/var/lib/aspen/ci/vms"),
            ..Default::default()
        };

        let snapshot = GoldenSnapshot::from_config(&config);

        assert_eq!(snapshot.dir, PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden"));
        assert_eq!(snapshot.memory_path, PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/memory-ranges"));
        assert_eq!(snapshot.state_path, PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/state.json"));
        assert_eq!(snapshot.ticket_path, PathBuf::from("/var/lib/aspen/ci/vms/snapshots/golden/ticket.txt"));
    }

    #[test]
    fn test_verify_sparse_file_with_sparse_file() {
        use std::io::Seek;
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sparse-mem");

        // Create a sparse file: write 1 byte at offset 1GB (allocates ~1 block, appears 1GB)
        let mut f = fs::File::create(&path).unwrap();
        f.seek(std::io::SeekFrom::Start(1024 * 1024 * 1024)).unwrap();
        f.write_all(b"x").unwrap();
        f.sync_all().unwrap();

        let info = verify_sparse_file(&path).unwrap();
        assert!(info.is_sparse, "file with a 1GB hole should be sparse");
        assert!(info.apparent_bytes > 1024 * 1024 * 1024);
        assert!(info.disk_bytes < info.apparent_bytes / 10);
    }

    #[test]
    fn test_verify_sparse_file_with_dense_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("dense-mem");

        // Write 64KB of actual data — not sparse
        let data = vec![0xABu8; 64 * 1024];
        fs::write(&path, &data).unwrap();

        let info = verify_sparse_file(&path).unwrap();
        // A 64KB fully-written file is not sparse
        assert!(!info.is_sparse, "fully-written file should not be sparse");
        assert_eq!(info.apparent_bytes, 64 * 1024);
    }

    #[test]
    fn test_verify_sparse_file_nonexistent() {
        let result = verify_sparse_file(Path::new("/nonexistent/memory"));
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_verify_sparse_memory() {
        use std::io::Seek;
        use std::io::Write;

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("snapshots/golden");
        fs::create_dir_all(&dir).unwrap();

        // Create sparse memory file
        let mem_path = dir.join("memory");
        let mut f = fs::File::create(&mem_path).unwrap();
        f.seek(std::io::SeekFrom::Start(512 * 1024 * 1024)).unwrap();
        f.write_all(b"x").unwrap();
        f.sync_all().unwrap();

        let snapshot = GoldenSnapshot {
            dir: dir.clone(),
            memory_path: mem_path,
            state_path: dir.join("state.json"),
            ticket_path: dir.join("ticket.txt"),
        };

        let info = snapshot.verify_sparse_memory().unwrap();
        assert!(info.is_sparse);
    }
}
