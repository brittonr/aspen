use std::ffi::OsStr;
use std::io::Read;
use std::io::Write;
use std::path::Component;
use std::path::Path;

use cap_fs_ext::DirExt;
use cap_fs_ext::FollowSymlinks;
use cap_fs_ext::OpenOptionsFollowExt;

use super::MAX_NODE_STATE_FILE_BYTES;
use super::authority::NodeStateEntryKind;
use super::authority::NodeStateFile;
use super::authority::NodeStateFileObservation;
use super::enumeration::entry_kind;
use super::invalid;

pub(super) fn validate_bootstrap_path(path: &Path) -> crate::error::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(invalid("node state root requires an explicit path"));
    }
    if path == Path::new(".") {
        return Err(invalid("node state root cannot be the ambient current directory"));
    }
    Ok(())
}

pub(super) fn validate_bootstrap_metadata(metadata: &std::fs::Metadata) -> crate::error::Result<()> {
    if metadata.file_type().is_symlink() {
        return Err(invalid("node state root must not be a symlink"));
    }
    if !metadata.is_dir() {
        return Err(invalid("node state root must be a directory"));
    }
    Ok(())
}

pub(super) fn create_dir_components(dir: &cap_std::fs::Dir, path: &Path) -> crate::error::Result<()> {
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(invalid("node state directory path must contain only normal relative components"));
        };
        match current.symlink_metadata(segment) {
            Ok(metadata) => {
                if entry_kind(&metadata.file_type()) != NodeStateEntryKind::Directory {
                    return Err(invalid(format!(
                        "node state directory component {} must be a directory",
                        segment.to_string_lossy()
                    )));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                current.create_dir(segment).map_err(crate::error::MoltenError::from)?;
            }
            Err(error) => return Err(crate::error::MoltenError::from(error)),
        }
        current = current.open_dir_nofollow(segment).map_err(crate::error::MoltenError::from)?;
    }
    Ok(())
}

pub(super) fn open_dir_components(dir: &cap_std::fs::Dir, path: &Path) -> crate::error::Result<cap_std::fs::Dir> {
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in path.components() {
        let Component::Normal(segment) = component else {
            return Err(invalid("node state directory path must contain only normal relative components"));
        };
        current = current.open_dir_nofollow(segment).map_err(crate::error::MoltenError::from)?;
    }
    Ok(current)
}

fn open_parent<'a>(
    dir: &cap_std::fs::Dir,
    path: &'a Path,
    create: bool,
) -> crate::error::Result<(cap_std::fs::Dir, &'a OsStr)> {
    let leaf = path.file_name().ok_or_else(|| invalid("node state path must have a regular leaf component"))?;
    let parent_path = path.parent().filter(|parent| !parent.as_os_str().is_empty());
    let parent = if let Some(parent_path) = parent_path {
        if create {
            create_dir_components(dir, parent_path)?;
        }
        open_dir_components(dir, parent_path)?
    } else {
        dir.try_clone().map_err(crate::error::MoltenError::from)?
    };
    Ok((parent, leaf))
}

pub(super) fn write_regular_file(
    dir: &cap_std::fs::Dir,
    path: &Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
) -> crate::error::Result<()> {
    let (parent, leaf) = open_parent(dir, path, true)?;
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => {
            if entry_kind(&metadata.file_type()) != NodeStateEntryKind::RegularFile {
                return Err(invalid(format!("node state write leaf {} must be a regular file", path.display())));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true).follow(FollowSymlinks::No);
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use cap_std::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = unix_mode;
    let mut file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(invalid(format!("node state write leaf {} changed away from a regular file", path.display())));
    }
    file.write_all(bytes).map_err(crate::error::MoltenError::from)?;
    file.flush().map_err(crate::error::MoltenError::from)
}

pub(super) fn observe_file(dir: &cap_std::fs::Dir, path: &Path) -> crate::error::Result<NodeStateFileObservation> {
    let Some((parent, leaf)) = open_parent_optional(dir, path)? else {
        return Ok(NodeStateFileObservation::Missing);
    };
    let metadata = match parent.symlink_metadata(leaf) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(NodeStateFileObservation::Missing),
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    };
    let kind = entry_kind(&metadata.file_type());
    if kind != NodeStateEntryKind::RegularFile {
        return Ok(NodeStateFileObservation::NonRegular(kind));
    }

    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    let metadata = file.metadata().map_err(crate::error::MoltenError::from)?;
    if !metadata.is_file() {
        return Err(invalid(format!("node state read leaf {} changed away from a regular file", path.display())));
    }
    #[cfg(unix)]
    let unix_mode = {
        use cap_std::fs::PermissionsExt;
        Some(metadata.permissions().mode())
    };
    #[cfg(not(unix))]
    let unix_mode = None;
    Ok(NodeStateFileObservation::Regular(NodeStateFile {
        file,
        size: metadata.len(),
        unix_mode,
    }))
}

pub(super) fn read_open_file_bounded(
    file: cap_std::fs::File,
    observed_size: u64,
    max_bytes: u64,
    label: &str,
) -> crate::error::Result<Vec<u8>> {
    if max_bytes > MAX_NODE_STATE_FILE_BYTES {
        return Err(invalid(format!(
            "node state read bound {max_bytes} exceeds hard maximum {MAX_NODE_STATE_FILE_BYTES}"
        )));
    }
    if observed_size > max_bytes {
        return Err(invalid(format!("{label} size {observed_size} exceeds bound {max_bytes}")));
    }
    let read_limit = max_bytes.checked_add(1).ok_or_else(|| invalid("node state read bound overflow"))?;
    let mut bytes = Vec::new();
    file.take(read_limit).read_to_end(&mut bytes).map_err(crate::error::MoltenError::from)?;
    if u64::try_from(bytes.len()).map_err(|_| invalid("node state read length conversion overflow"))? > max_bytes {
        return Err(invalid(format!("{label} exceeds bound {max_bytes}")));
    }
    Ok(bytes)
}

pub(super) fn read_regular_file_bounded(
    dir: &cap_std::fs::Dir,
    path: &Path,
    max_bytes: u64,
) -> crate::error::Result<Vec<u8>> {
    match observe_file(dir, path)? {
        NodeStateFileObservation::Missing => Err(invalid(format!("node state file {} does not exist", path.display()))),
        NodeStateFileObservation::NonRegular(_) => {
            Err(invalid(format!("node state read leaf {} must be a regular file", path.display())))
        }
        NodeStateFileObservation::Regular(file) => file.read_bounded(max_bytes),
    }
}

pub(super) fn remove_regular_file(dir: &cap_std::fs::Dir, path: &Path) -> crate::error::Result<()> {
    let (parent, leaf) = open_parent(dir, path, false)?;
    let metadata = parent.symlink_metadata(leaf).map_err(crate::error::MoltenError::from)?;
    if entry_kind(&metadata.file_type()) != NodeStateEntryKind::RegularFile {
        return Err(invalid(format!("node state removal leaf {} must be a regular file", path.display())));
    }
    parent.remove_file(leaf).map_err(crate::error::MoltenError::from)
}

pub(super) fn entry_kind_optional(
    dir: &cap_std::fs::Dir,
    path: &Path,
) -> crate::error::Result<Option<NodeStateEntryKind>> {
    let Some((parent, leaf)) = open_parent_optional(dir, path)? else {
        return Ok(None);
    };
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => Ok(Some(entry_kind(&metadata.file_type()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::error::MoltenError::from(error)),
    }
}

fn open_parent_optional<'a>(
    dir: &cap_std::fs::Dir,
    path: &'a Path,
) -> crate::error::Result<Option<(cap_std::fs::Dir, &'a OsStr)>> {
    let leaf = path.file_name().ok_or_else(|| invalid("node state path must have a regular leaf component"))?;
    let Some(parent_path) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) else {
        return dir.try_clone().map(|parent| Some((parent, leaf))).map_err(crate::error::MoltenError::from);
    };
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in parent_path.components() {
        let Component::Normal(segment) = component else {
            return Err(invalid("node state directory path must contain only normal relative components"));
        };
        match current.open_dir_nofollow(segment) {
            Ok(next) => current = next,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(crate::error::MoltenError::from(error)),
        }
    }
    Ok(Some((current, leaf)))
}

pub(super) fn open_database_file(dir: &cap_std::fs::Dir, path: &Path) -> crate::error::Result<std::fs::File> {
    let (parent, leaf) = open_parent(dir, path, true)?;
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => {
            if entry_kind(&metadata.file_type()) != NodeStateEntryKind::RegularFile {
                return Err(invalid(format!("node state database leaf {} must be a regular file", path.display())));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).follow(FollowSymlinks::No);
    let file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(invalid(format!("node state database leaf {} changed away from a regular file", path.display())));
    }
    Ok(file.into_std())
}

pub(super) fn validate_write_size(bytes: &[u8], max_bytes: u64) -> crate::error::Result<()> {
    let byte_count = u64::try_from(bytes.len()).map_err(|_| invalid("node state write length conversion overflow"))?;
    if byte_count > max_bytes {
        Err(invalid(format!("node state write size {byte_count} exceeds maximum {max_bytes}")))
    } else {
        Ok(())
    }
}
