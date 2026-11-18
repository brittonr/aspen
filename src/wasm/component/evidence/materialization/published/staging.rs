//! Private staging bridges authorized directory descriptors to the published
//! verifier and bounded source copier. No ambient root is discovered here.

use std::os::fd::AsRawFd;

use cap_fs_ext::DirExt;
use cap_fs_ext::OpenOptionsFollowExt;
use cap_std::fs::PermissionsExt;

/// Faithful source-tree copies reproduce source directory modes, so a Nix
/// store closure stages as read-only `0555` directories whose entries cannot
/// be unlinked. The stage is private to one operation, so cleanup restores
/// owner access first.
const PRIVATE_STAGE_DIRECTORY_MODE: u32 = 0o700;

/// Mantle's source-tree entry bound plus every measured member's parents.
const MAX_PRIVATE_STAGE_DIRECTORIES: usize = 16_384;

pub(super) fn with_private_stage<T>(
    parent: &cap_std::fs::Dir,
    leaf: &str,
    run: impl FnOnce(&cap_std::fs::Dir) -> super::Result<T>,
) -> super::Result<T> {
    if leaf.is_empty() || leaf == "." || leaf == ".." || leaf.contains('/') || leaf.contains('\0') {
        return Err(super::Denial::new("materialization staging requires an explicitly admitted single leaf"));
    }
    // create_dir, not create_dir_all: an existing or reused lease always denies.
    parent
        .create_dir(leaf)
        .map_err(|_| super::Denial::new("materialization staging leaf already exists or is unavailable"))?;
    let directory = match parent.open_dir_nofollow(leaf) {
        Ok(directory) => directory,
        Err(_) => {
            parent
                .remove_dir(leaf)
                .map_err(|_| super::Denial::new("new materialization staging open and cleanup failed"))?;
            return Err(super::Denial::new("new materialization staging capability could not be opened"));
        }
    };
    let result = directory
        .entries()
        .map_err(|_| super::Denial::new("materialization staging inventory is unavailable"))
        .and_then(|mut entries| {
            if entries.next().is_some() {
                Err(super::Denial::new("materialization staging capability is not empty"))
            } else {
                run(&directory)
            }
        });
    restore_owner_access(&directory)
        .and_then(|()| directory.remove_open_dir_all())
        .map_err(|_| super::Denial::new("private materialization staging cleanup failed"))?;
    result
}

/// Visits only directories below the private stage, never following symlinks.
fn restore_owner_access(stage: &cap_std::fs::Dir) -> std::io::Result<()> {
    let mut pending = vec![stage.try_clone()?];
    let mut restored = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in directory.entries()? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            restored += 1;
            if restored > MAX_PRIVATE_STAGE_DIRECTORIES {
                return Err(std::io::Error::other("private staging directory bound exceeded"));
            }
            let name = entry.file_name();
            directory.set_permissions(&name, cap_std::fs::Permissions::from_mode(PRIVATE_STAGE_DIRECTORY_MODE))?;
            pending.push(directory.open_dir_nofollow(&name)?);
        }
    }
    Ok(())
}

pub(super) fn create_member(root: &cap_std::fs::Dir, relative: &str) -> super::Result<cap_std::fs::File> {
    let mut parent = root.try_clone().map_err(|_| super::Denial::new("private staging capability is unavailable"))?;
    let mut parts = relative.split('/').peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            let mut options = cap_std::fs::OpenOptions::new();
            options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
            return parent
                .open_with(part, &options)
                .map_err(|_| super::Denial::new("private staging member exists or cannot be created"));
        }
        match parent.create_dir(part) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(super::Denial::new("private staging parent cannot be created")),
        }
        parent = parent
            .open_dir_nofollow(part)
            .map_err(|_| super::Denial::new("private staging parent is not a no-follow directory"))?;
    }
    Err(super::Denial::new("private staging member has no leaf"))
}

pub(super) fn consumer_root(root: &cap_std::fs::Dir) -> super::Result<mantle_wasm_consumer_verifier::ConsumerRoot> {
    // Linux descriptor bridge only. Both cap-std cohorts retain opened root
    // capabilities; the caller keeps this descriptor live for the complete
    // synchronous operation. Provider descendant traversal stays no-follow.
    let descriptor = std::path::PathBuf::from(format!("/proc/self/fd/{}", root.as_raw_fd()));
    mantle_wasm_consumer_verifier::ConsumerRoot::open(&descriptor)
        .map_err(|_| super::Denial::new("published verifier cannot open the authorized directory descriptor"))
}
