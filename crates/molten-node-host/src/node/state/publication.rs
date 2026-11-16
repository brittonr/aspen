use std::io::Write;

use cap_fs_ext::OpenOptionsFollowExt;

/// Separates a stage name from its destination leaf: `.{leaf}.stage-{process}-{sequence}`.
const STAGE_INFIX: &str = ".stage-";

static STAGE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Whether a directory entry is a stage of an in-progress or interrupted publication; scans skip
/// it.
pub(crate) fn is_stage(name: &str) -> bool {
    name.starts_with('.') && name.contains(STAGE_INFIX)
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
    let stage = format!(
        ".{leaf_name}{STAGE_INFIX}{}-{}",
        std::process::id(),
        STAGE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
/// read.
fn remove_stale_stages(parent: &cap_std::fs::Dir, leaf_name: &str) -> crate::error::Result<()> {
    let prefix = format!(".{leaf_name}{STAGE_INFIX}");
    for entry in parent.read_dir(".").map_err(crate::error::Failure::from)? {
        let entry = entry.map_err(crate::error::Failure::from)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(&prefix) && entry.file_type().map_err(crate::error::Failure::from)?.is_file() {
            match parent.remove_file(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(crate::error::Failure::from(error)),
            }
        }
    }
    Ok(())
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
mod tests {
    use super::*;

    const SECRET_MODE: u32 = 0o600;

    fn tempdir() -> cap_tempfile::TempDir {
        cap_tempfile::tempdir(cap_tempfile::ambient_authority()).expect("temporary directory")
    }

    fn mode(dir: &cap_std::fs::Dir, leaf: &str) -> u32 {
        use cap_std::fs::PermissionsExt;
        dir.metadata(leaf).expect("metadata").permissions().mode() & 0o777
    }

    fn stages(dir: &cap_std::fs::Dir) -> Vec<String> {
        dir.entries()
            .expect("entries")
            .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(STAGE_INFIX))
            .collect()
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn no_replace_creates_once_and_never_overwrites() {
        let dir = tempdir();
        let path = std::path::Path::new("keys/transport.key");
        publish_regular_file(&dir, path, b"first", Some(SECRET_MODE), Replacement::NoReplace).expect("first key");
        let keys = dir.open_dir("keys").expect("key directory");
        assert_eq!(keys.read("transport.key").expect("key bytes"), b"first");
        assert_eq!(mode(&keys, "transport.key"), SECRET_MODE);

        let second = publish_regular_file(&dir, path, b"second", Some(SECRET_MODE), Replacement::NoReplace);
        assert!(second.is_err(), "an existing key must not be replaced");
        assert_eq!(keys.read("transport.key").expect("key bytes"), b"first");
        assert!(stages(&keys).is_empty());
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn replacement_swaps_whole_content_and_restricts_a_loose_existing_file() {
        let dir = tempdir();
        dir.write("transport.key", b"old generation").expect("existing key");
        {
            use cap_std::fs::PermissionsExt;
            dir.set_permissions("transport.key", cap_std::fs::Permissions::from_mode(0o644))
                .expect("loose mode");
        }
        let path = std::path::Path::new("transport.key");
        publish_regular_file(&dir, path, b"new", Some(SECRET_MODE), Replacement::Replace).expect("rotation");
        assert_eq!(dir.read("transport.key").expect("key bytes"), b"new");
        assert_eq!(mode(&dir, "transport.key"), SECRET_MODE);
        assert!(stages(&dir).is_empty());
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[test]
    fn interrupted_stages_are_removed_and_never_become_the_key() {
        let dir = tempdir();
        dir.write(format!(".transport.key{STAGE_INFIX}1-0"), b"half written").expect("stale stage");
        dir.write(format!(".other.key{STAGE_INFIX}1-0"), b"other leaf").expect("other stage");
        let path = std::path::Path::new("transport.key");
        publish_regular_file(&dir, path, b"whole", Some(SECRET_MODE), Replacement::NoReplace).expect("publication");
        assert_eq!(dir.read("transport.key").expect("key bytes"), b"whole");
        assert_eq!(stages(&dir), vec![format!(".other.key{STAGE_INFIX}1-0")]);
    }

    // r[verify molten.crypto_identity.atomic_key_publication]
    #[cfg(unix)]
    #[test]
    fn a_symlinked_or_non_regular_destination_is_refused() {
        let dir = tempdir();
        dir.create_dir("elsewhere").expect("directory leaf");
        let refused = publish_regular_file(
            &dir,
            std::path::Path::new("elsewhere"),
            b"key",
            Some(SECRET_MODE),
            Replacement::Replace,
        );
        assert!(refused.is_err(), "a directory leaf must not be replaced");

        cap_fs_ext::DirExt::symlink(&*dir, "target.key", "linked.key").expect("symlink leaf");
        let through_link = publish_regular_file(
            &dir,
            std::path::Path::new("linked.key"),
            b"key",
            Some(SECRET_MODE),
            Replacement::Replace,
        );
        assert!(through_link.is_err(), "a symlink leaf must not be followed or replaced");
        assert!(dir.symlink_metadata("target.key").is_err(), "nothing is written through the link");
        assert!(stages(&dir).is_empty());
    }
}
