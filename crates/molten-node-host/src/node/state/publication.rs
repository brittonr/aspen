use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

#[path = "publication/exclusion.rs"]
pub(super) mod exclusion;

pub use exclusion::LeafLock;

/// Prefix of every publication artifact: `.stage.{leaf byte length}.{leaf}.{kind}`. The decimal
/// length fixes where the leaf ends, so no artifact of one leaf can match another leaf's prefix.
const ARTIFACT_PREFIX: &str = ".stage.";
/// Kind prefix of a payload stage: `stage-{process}-{sequence}`.
const STAGE_KIND: &str = "stage-";
/// Kind of the lock every publication of a leaf holds while it settles.
const PUBLICATION_LOCK_KIND: &str = "lock";
/// Kind of the lock a read-modify-write update of a leaf holds across its read and publication.
pub(crate) const UPDATE_LOCK_KIND: &str = "update-lock";
/// Infix of the earlier `.{leaf}.stage-…` stage names; scans still skip interrupted legacy stages.
const LEGACY_STAGE_INFIX: &str = ".stage-";
#[cfg(unix)]
const LEAF_LOCK_MODE: u32 = 0o600;
/// Attempts to lock a lock file that a settling holder unlinks concurrently before giving up.
const LEAF_LOCK_ATTEMPTS: usize = 64;

static STAGE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Whether a directory entry is a stage or lock of an in-progress or interrupted publication;
/// scans skip it.
pub(crate) fn is_stage(name: &str) -> bool {
    name.starts_with(ARTIFACT_PREFIX) || (name.starts_with('.') && name.contains(LEGACY_STAGE_INFIX))
}

/// Whether `name` uses the reserved publication-artifact prefix; such names are never admitted as
/// application leaves, so internal stages and locks cannot collide with application data.
pub(crate) fn is_reserved_artifact_name(name: &str) -> bool {
    name.starts_with(ARTIFACT_PREFIX)
}

fn artifact_prefix(leaf_name: &str) -> String {
    format!("{ARTIFACT_PREFIX}{}.{leaf_name}.", leaf_name.len())
}

fn artifact_name(leaf_name: &str, kind: &str) -> String {
    format!("{}{kind}", artifact_prefix(leaf_name))
}

// r[impl molten.audit_f03.recover_publication]
// An error after creation retains the partial member for explicit reconciliation.
pub(super) fn write_new_regular_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    bytes: &[u8],
) -> crate::error::Result<()> {
    let (parent, leaf) = super::filesystem::open_parent(dir, path, false)?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = parent.open_with(leaf, &options).map_err(crate::error::Failure::from)?;
    if !file.metadata().map_err(crate::error::Failure::from)?.is_file() {
        return Err(super::invalid("new node state publication is not a regular file"));
    }
    file.write_all(bytes).map_err(crate::error::Failure::from)?;
    file.sync_all().map_err(crate::error::Failure::from)?;
    sync_directory(&parent)
}

/// How a staged publication treats an existing destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Replacement {
    /// The destination must not exist; an existing file is never overwritten.
    NoReplace,
    /// An existing regular-file destination is replaced in one step.
    Replace,
}

/// Publishes `bytes` so a crash leaves the destination absent, unchanged, or holding all of `bytes`
/// with `unix_mode`, never a truncated or partially written file. The payload goes to a stage in
/// the destination directory and is synchronized, then linked (no replacement) or renamed
/// (replacement) onto the leaf, and the directory is synchronized. A failure after the leaf changed
/// is reported as an uncertain outcome.
// r[impl molten.crypto_identity.atomic_key_publication]
pub(crate) fn publish_regular_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
    replacement: Replacement,
) -> crate::error::Result<()> {
    let (parent, leaf) = super::filesystem::open_parent(dir, path, true)?;
    let leaf_name = leaf
        .to_str()
        .ok_or_else(|| super::invalid(format!("node state publication leaf {} is not UTF-8", path.display())))?;
    // Every publication of this leaf, in any process, holds the leaf's lock until it settles, so a
    // stage present while the lock is held here belongs to an interrupted writer, never a live one.
    if is_reserved_artifact_name(leaf_name) {
        return Err(super::invalid(format!("node state publication leaf {leaf_name} uses a reserved name")));
    }
    let _publication = exclusion::lock_leaf(&parent, leaf_name, PUBLICATION_LOCK_KIND)?;
    match parent.symlink_metadata(leaf) {
        Ok(_) if replacement == Replacement::NoReplace => {
            return Err(super::invalid(format!("node state publication {} already exists", path.display())));
        }
        Ok(metadata) => {
            if super::enumeration::entry_kind(&metadata.file_type()) != super::authority::EntryKind::RegularFile {
                return Err(super::invalid(format!("node state write leaf {} must be a regular file", path.display())));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::Failure::from(error)),
    }
    remove_stale_stages(&parent, leaf_name)?;
    let stage = artifact_name(
        leaf_name,
        &format!(
            "{STAGE_KIND}{}-{}",
            std::process::id(),
            STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ),
    );
    if let Err(error) = write_stage(&parent, &stage, bytes, unix_mode) {
        return Err(discard_stage(&parent, &stage, error));
    }
    let committed = match replacement {
        Replacement::NoReplace => parent.hard_link(&stage, &parent, leaf),
        Replacement::Replace => parent.rename(&stage, &parent, leaf),
    };
    if let Err(error) = committed {
        return Err(discard_stage(&parent, &stage, crate::error::Failure::from(error)));
    }
    let settled = match replacement {
        Replacement::NoReplace => parent.remove_file(&stage).map_err(crate::error::Failure::from),
        Replacement::Replace => Ok(()),
    };
    settled.and_then(|()| sync_directory(&parent)).map_err(|error| {
        crate::error::Failure::classified(
            crate::error::FailureClass::UncertainExternalOutcome,
            format!("node state publication {} is visible but not settled: {error}", path.display()),
        )
    })
}

fn write_stage(
    parent: &cap_std::fs::Dir,
    stage: &str,
    bytes: &[u8],
    unix_mode: Option<u32>,
) -> crate::error::Result<()> {
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use cap_std::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _unused_mode = unix_mode;
    let mut file = parent.open_with(stage, &options).map_err(crate::error::Failure::from)?;
    file.write_all(bytes).map_err(crate::error::Failure::from)?;
    file.sync_all().map_err(crate::error::Failure::from)
}

/// Removes this publication's stage after a failure and returns the original error, which takes
/// precedence.
fn discard_stage(parent: &cap_std::fs::Dir, stage: &str, error: crate::error::Failure) -> crate::error::Failure {
    match parent.remove_file(stage) {
        Ok(()) => error,
        Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup) => super::invalid(format!("{error}; stage {stage} was not removed: {cleanup}")),
    }
}

/// Removes stages that an interrupted publication of the same leaf left behind; they are never
/// read. Callers hold the leaf's publication lock, so no live writer owns any of these stages, and
/// the length-delimited prefix never matches another leaf's stages or any lock. Only names of the
/// exact form this module creates (`…stage-{process}-{sequence}`, both decimal) are removed; any
/// other file under the reserved prefix, however it got there, is left untouched.
fn remove_stale_stages(parent: &cap_std::fs::Dir, leaf_name: &str) -> crate::error::Result<()> {
    let prefix = artifact_name(leaf_name, STAGE_KIND);
    for entry in parent.read_dir(".").map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let is_well_formed_stage = name.strip_prefix(&prefix).is_some_and(is_stage_suffix);
        if is_well_formed_stage && entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            match parent.remove_file(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(crate::error::Failure::from(error)),
            }
        }
    }
    Ok(())
}

/// Whether `suffix` is exactly `{process}-{sequence}` with nonempty decimal parts.
fn is_stage_suffix(suffix: &str) -> bool {
    let is_decimal = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    suffix
        .split_once('-')
        .is_some_and(|(process, sequence)| is_decimal(process) && is_decimal(sequence))
}

// Directory capabilities may hold O_PATH descriptors, which cannot be synchronized.
fn sync_directory(parent: &cap_std::fs::Dir) -> crate::error::Result<()> {
    parent
        .open(".")
        .map_err(crate::error::Failure::from)?
        .sync_all()
        .map_err(crate::error::Failure::from)
}

#[cfg(test)]
#[path = "publication/tests.rs"]
mod tests;
