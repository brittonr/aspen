use super::*;

/// Why the resolved executable could not be bound to its admitted measurement. Either way nothing
/// was spawned.
pub(super) enum ExecutableMeasurementFailure {
    /// The executable could not be opened, read, or bounded.
    Unavailable(String),
    /// The opened executable bytes differ from the admitted executable identity.
    Mismatch,
}

/// The resolved executable, opened once and measured from that handle. The process is spawned
/// through the same handle, so the bytes that run are the bytes that were measured.
pub(super) struct MeasuredExecutable {
    file: std::fs::File,
}

impl MeasuredExecutable {
    /// The handle-relative path the spawned program resolves to; it stays valid while `self` is
    /// alive. It is also the child's `argv[0]`: `bounded-exec` sets no separate `argv[0]`, so an
    /// executable that selects its behavior from that name (a BusyBox or coreutils multicall
    /// binary) is unsupported by the live adapter.
    pub(super) fn process_path(&self) -> Result<std::path::PathBuf, String> {
        #[cfg(target_os = "linux")]
        {
            let descriptor = std::os::fd::AsRawFd::as_raw_fd(&self.file);
            Ok(std::path::PathBuf::from(format!("/proc/{}/fd/{descriptor}", std::process::id())))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = &self.file;
            Err("measured executable handles require the admitted Linux process profile".to_string())
        }
    }
}

/// The `fstat` identity of an opened file. A rename, replacement, size change, or metadata change
/// alters at least one field; a same-size write within one timestamp tick of a coarse-grained
/// filesystem may not, so excluding writers remains the resolver capability's responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FileStamp {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileStamp {
    fn of(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt as _;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

/// Executable identities this adapter has already measured, each with the stamp of the handle it
/// hashed. A handle whose stamp differs from the recorded one is hashed again.
#[derive(Debug, Default)]
pub(super) struct MeasurementCache {
    measured: std::collections::BTreeMap<String, FileStamp>,
}

const MEASUREMENT_BUFFER_BYTES: usize = 1 << 20;

// r[impl molten.bounded_execution.live_adapter_binding_and_no_replay]
pub(super) fn measure_executable(
    request: &Request,
    resolved: &ResolvedContext,
    cache: &mut MeasurementCache,
) -> Result<MeasuredExecutable, ExecutableMeasurementFailure> {
    // Non-blocking open, so naming a FIFO cannot stall the adapter before the regular-file check.
    let file = std::os::unix::fs::OpenOptionsExt::custom_flags(
        std::fs::OpenOptions::new().read(true),
        rustix::fs::OFlags::NONBLOCK.bits().cast_signed(),
    )
    .open(&resolved.executable_path)
    .map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be opened: {error}"))
    })?;
    // The length comes from the same handle; only a regular file has one, so a device or pipe cannot
    // make measurement unbounded.
    let metadata = file.metadata().map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be inspected: {error}"))
    })?;
    if !metadata.is_file() {
        return Err(ExecutableMeasurementFailure::Unavailable("resolved executable is not a regular file".to_string()));
    }
    let identity = &request.plan.resolution.executable_identity_ref;
    let stamp = FileStamp::of(&metadata);
    if cache.measured.get(identity) == Some(&stamp) {
        return Ok(MeasuredExecutable { file });
    }
    let mut hasher = blake3::Hasher::new();
    // Large reads let BLAKE3 hash many chunks per update instead of one small buffer at a time.
    let mut bounded = std::io::BufReader::with_capacity(
        MEASUREMENT_BUFFER_BYTES,
        std::io::Read::take(&file, metadata.len().saturating_add(1)),
    );
    let measured_bytes = std::io::copy(&mut bounded, &mut hasher).map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be measured: {error}"))
    })?;
    if measured_bytes != metadata.len() {
        return Err(ExecutableMeasurementFailure::Unavailable(
            "resolved executable changed length while it was measured".to_string(),
        ));
    }
    if crate::preserves_rail::content_ref_from_blake3_hash(hasher.finalize()) != *identity {
        cache.measured.remove(identity);
        return Err(ExecutableMeasurementFailure::Mismatch);
    }
    // A write during hashing changes the stamp; such a measurement is refused rather than cached.
    let after = file.metadata().map_err(|error| {
        ExecutableMeasurementFailure::Unavailable(format!("resolved executable could not be inspected: {error}"))
    })?;
    if FileStamp::of(&after) != stamp {
        return Err(ExecutableMeasurementFailure::Unavailable(
            "resolved executable changed while it was measured".to_string(),
        ));
    }
    cache.measured.insert(identity.clone(), stamp);
    Ok(MeasuredExecutable { file })
}
