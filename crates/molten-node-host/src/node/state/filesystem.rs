#[path = "filesystem/bounded_read.rs"]
pub(super) mod bounded_read;

use std::io::Write;

use cap_fs_ext::DirExt;
use cap_fs_ext::OpenOptionsFollowExt;

pub(super) fn validate_bootstrap_path(path: &std::path::Path) -> crate::error::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(super::invalid("node state root requires an explicit path"));
    }
    if path == std::path::Path::new(".") {
        return Err(super::invalid("node state root cannot be the ambient current directory"));
    }
    Ok(())
}

pub(super) fn validate_bootstrap_metadata(metadata: &std::fs::Metadata) -> crate::error::Result<()> {
    if metadata.file_type().is_symlink() {
        return Err(super::invalid("node state root must not be a symlink"));
    }
    if !metadata.is_dir() {
        return Err(super::invalid("node state root must be a directory"));
    }
    Ok(())
}

pub(super) fn create_dir_components(dir: &cap_std::fs::Dir, path: &std::path::Path) -> crate::error::Result<()> {
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(super::invalid("node state directory path must contain only normal relative components"));
        };
        match current.symlink_metadata(segment) {
            Ok(metadata) => {
                if super::enumeration::entry_kind(&metadata.file_type())
                    != super::authority::NodeStateEntryKind::Directory
                {
                    return Err(super::invalid(format!(
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

pub(super) fn open_dir_components(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<cap_std::fs::Dir> {
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(super::invalid("node state directory path must contain only normal relative components"));
        };
        current = current.open_dir_nofollow(segment).map_err(crate::error::MoltenError::from)?;
    }
    Ok(current)
}

fn open_parent<'a>(
    dir: &cap_std::fs::Dir,
    path: &'a std::path::Path,
    create: bool,
) -> crate::error::Result<(cap_std::fs::Dir, &'a std::ffi::OsStr)> {
    let leaf = path
        .file_name()
        .ok_or_else(|| super::invalid("node state path must have a regular leaf component"))?;
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
    path: &std::path::Path,
    bytes: &[u8],
    unix_mode: Option<u32>,
) -> crate::error::Result<()> {
    let (parent, leaf) = open_parent(dir, path, true)?;
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => {
            if super::enumeration::entry_kind(&metadata.file_type())
                != super::authority::NodeStateEntryKind::RegularFile
            {
                return Err(super::invalid(format!("node state write leaf {} must be a regular file", path.display())));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true).follow(cap_fs_ext::FollowSymlinks::No);
    #[cfg(unix)]
    if let Some(mode) = unix_mode {
        use cap_std::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = unix_mode;
    let mut file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(super::invalid(format!(
            "node state write leaf {} changed away from a regular file",
            path.display()
        )));
    }
    file.write_all(bytes).map_err(crate::error::MoltenError::from)?;
    file.flush().map_err(crate::error::MoltenError::from)
}

pub(super) fn observe_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<super::authority::NodeStateFileObservation> {
    let Some((parent, leaf)) = open_parent_optional(dir, path)? else {
        return Ok(super::authority::NodeStateFileObservation::Missing);
    };
    let metadata = match parent.symlink_metadata(leaf) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(super::authority::NodeStateFileObservation::Missing);
        }
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    };
    let kind = super::enumeration::entry_kind(&metadata.file_type());
    if kind != super::authority::NodeStateEntryKind::RegularFile {
        return Ok(super::authority::NodeStateFileObservation::NonRegular(kind));
    }

    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    let metadata = file.metadata().map_err(crate::error::MoltenError::from)?;
    if !metadata.is_file() {
        return Err(super::invalid(format!(
            "node state read leaf {} changed away from a regular file",
            path.display()
        )));
    }
    #[cfg(unix)]
    let unix_mode = {
        use cap_std::fs::PermissionsExt;
        Some(metadata.permissions().mode())
    };
    #[cfg(not(unix))]
    let unix_mode = None;
    Ok(super::authority::NodeStateFileObservation::Regular(super::authority::NodeStateFile {
        file,
        size: metadata.len(),
        unix_mode,
    }))
}

pub(super) fn read_regular_file_bounded(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    max_bytes: u64,
) -> crate::error::Result<Vec<u8>> {
    match observe_file(dir, path)? {
        super::authority::NodeStateFileObservation::Missing => {
            Err(super::invalid(format!("node state file {} does not exist", path.display())))
        }
        super::authority::NodeStateFileObservation::NonRegular(_) => {
            Err(super::invalid(format!("node state read leaf {} must be a regular file", path.display())))
        }
        super::authority::NodeStateFileObservation::Regular(file) => file.read_bounded(max_bytes),
    }
}

pub(super) fn remove_regular_file(dir: &cap_std::fs::Dir, path: &std::path::Path) -> crate::error::Result<()> {
    let (parent, leaf) = open_parent(dir, path, false)?;
    let metadata = parent.symlink_metadata(leaf).map_err(crate::error::MoltenError::from)?;
    if super::enumeration::entry_kind(&metadata.file_type()) != super::authority::NodeStateEntryKind::RegularFile {
        return Err(super::invalid(format!("node state removal leaf {} must be a regular file", path.display())));
    }
    parent.remove_file(leaf).map_err(crate::error::MoltenError::from)
}

pub(super) fn entry_kind_optional(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<Option<super::authority::NodeStateEntryKind>> {
    let Some((parent, leaf)) = open_parent_optional(dir, path)? else {
        return Ok(None);
    };
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => Ok(Some(super::enumeration::entry_kind(&metadata.file_type()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::error::MoltenError::from(error)),
    }
}

fn open_parent_optional<'a>(
    dir: &cap_std::fs::Dir,
    path: &'a std::path::Path,
) -> crate::error::Result<Option<(cap_std::fs::Dir, &'a std::ffi::OsStr)>> {
    let leaf = path
        .file_name()
        .ok_or_else(|| super::invalid("node state path must have a regular leaf component"))?;
    let Some(parent_path) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) else {
        return dir.try_clone().map(|parent| Some((parent, leaf))).map_err(crate::error::MoltenError::from);
    };
    let mut current = dir.try_clone().map_err(crate::error::MoltenError::from)?;
    for component in parent_path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(super::invalid("node state directory path must contain only normal relative components"));
        };
        match current.open_dir_nofollow(segment) {
            Ok(next) => current = next,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(crate::error::MoltenError::from(error)),
        }
    }
    Ok(Some((current, leaf)))
}

pub(super) fn open_database_file(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<std::fs::File> {
    let (parent, leaf) = open_parent(dir, path, true)?;
    match parent.symlink_metadata(leaf) {
        Ok(metadata) => {
            if super::enumeration::entry_kind(&metadata.file_type())
                != super::authority::NodeStateEntryKind::RegularFile
            {
                return Err(super::invalid(format!(
                    "node state database leaf {} must be a regular file",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(crate::error::MoltenError::from(error)),
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).follow(cap_fs_ext::FollowSymlinks::No);
    let file = parent.open_with(leaf, &options).map_err(crate::error::MoltenError::from)?;
    if !file.metadata().map_err(crate::error::MoltenError::from)?.is_file() {
        return Err(super::invalid(format!(
            "node state database leaf {} changed away from a regular file",
            path.display()
        )));
    }
    Ok(file.into_std())
}

pub(super) fn validate_write_size(bytes: &[u8], max_bytes: u64) -> crate::error::Result<()> {
    let byte_count =
        u64::try_from(bytes.len()).map_err(|_| super::invalid("node state write length conversion overflow"))?;
    if byte_count > max_bytes {
        Err(super::invalid(format!("node state write size {byte_count} exceeds maximum {max_bytes}")))
    } else {
        Ok(())
    }
}
