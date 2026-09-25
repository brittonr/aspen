
fn validate_materialization_path(value: &str, max_path_bytes: usize) -> crate::error::Result<()> {
    if value.is_empty() {
        return Err(invalid("materialization member path cannot be empty"));
    }
    if value.len() > max_path_bytes {
        return Err(invalid("materialization member path exceeds configured byte bound"));
    }
    if value.starts_with('/')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value.contains('\0')
        || value.contains("://")
    {
        return Err(invalid("materialization member path is absolute or separator-ambiguous"));
    }
    let bytes = value.as_bytes();
    if bytes.first().is_some_and(u8::is_ascii_alphabetic) && bytes.get(1) == Some(&b':') {
        return Err(invalid("materialization member path has a platform prefix"));
    }
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(invalid("materialization member path contains an unsafe component"));
        }
    }
    for component in std::path::Path::new(value).components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return Err(invalid("materialization member path must be relative and normalized"));
        }
    }
    Ok(())
}

fn stage_path(plan: &MaterializationPlan) -> crate::error::Result<std::path::PathBuf> {
    let token = plan
        .plan_ref
        .strip_prefix("blake3:")
        .ok_or_else(|| invalid("materialization plan ref is not BLAKE3"))?;
    Ok(std::path::Path::new(STAGING_DIRECTORY).join(token))
}

fn create_staging_root(dir: &cap_std::fs::Dir, stage_path: &std::path::Path) -> crate::error::Result<()> {
    match entry_kind(dir, std::path::Path::new(STAGING_DIRECTORY))? {
        None => dir.create_dir(STAGING_DIRECTORY).map_err(crate::error::MoltenError::from)?,
        Some(MaterializationMemberKind::Directory) => {}
        Some(_) => return Err(invalid("materialization staging root must be a real directory")),
    }
    ensure_no_symlink_components(dir, stage_path.parent())?;
    dir.create_dir(stage_path).map_err(crate::error::MoltenError::from)?;
    create_directory_tree(dir, Some(&stage_path.join(STAGING_TREE_DIRECTORY)))?;
    Ok(())
}

fn create_directory_tree(dir: &cap_std::fs::Dir, path: Option<&std::path::Path>) -> crate::error::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(invalid("capability directory creation received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => dir.create_dir(&current).map_err(crate::error::MoltenError::from)?,
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn create_directory_tree_recording(
    dir: &cap_std::fs::Dir,
    path: Option<&std::path::Path>,
    created: &mut impl crate::bounded::VecSink<std::path::PathBuf>,
) -> crate::error::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(invalid("capability directory creation received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => {
                dir.create_dir(&current).map_err(crate::error::MoltenError::from)?;
                created.push_item(current.clone());
            }
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn ensure_no_symlink_components(dir: &cap_std::fs::Dir, path: Option<&std::path::Path>) -> crate::error::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        let std::path::Component::Normal(component) = component else {
            return Err(invalid("materialization parent check received a non-relative component"));
        };
        current.push(component);
        match entry_kind(dir, &current)? {
            None => return Ok(()),
            Some(MaterializationMemberKind::Directory) => {}
            Some(_) => return Err(invalid("materialization parent is a symlink or non-directory entry")),
        }
    }
    Ok(())
}

fn entry_kind(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
) -> crate::error::Result<Option<MaterializationMemberKind>> {
    match dir.symlink_metadata(path) {
        Ok(metadata) => Ok(Some(member_kind(&metadata.file_type()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(crate::error::MoltenError::from(error)),
    }
}

fn member_kind(file_type: &cap_std::fs::FileType) -> MaterializationMemberKind {
    if file_type.is_file() {
        MaterializationMemberKind::RegularFile
    } else if file_type.is_dir() {
        MaterializationMemberKind::Directory
    } else {
        link_kind(file_type)
    }
}

fn link_kind(file_type: &cap_std::fs::FileType) -> MaterializationMemberKind {
    if file_type.is_symlink() {
        MaterializationMemberKind::Symlink
    } else {
        MaterializationMemberKind::Special
    }
}

fn write_create_new(dir: &cap_std::fs::Dir, path: &std::path::Path, bytes: &[u8]) -> crate::error::Result<()> {
    ensure_no_symlink_components(dir, path.parent())?;
    let mut options = cap_std::fs::OpenOptions::new();
    options.write(true).create_new(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = dir.open_with(path, &options).map_err(crate::error::MoltenError::from)?;
    file.write_all(bytes).map_err(crate::error::MoltenError::from)?;
    file.flush().map_err(crate::error::MoltenError::from)
}

fn read_regular_file_bounded(
    dir: &cap_std::fs::Dir,
    path: &std::path::Path,
    max_bytes: u64,
) -> crate::error::Result<Vec<u8>> {
    ensure_no_symlink_components(dir, path.parent())?;
    if entry_kind(dir, path)? != Some(MaterializationMemberKind::RegularFile) {
        return Err(invalid("materialization read target must be a regular file"));
    }
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).follow(cap_fs_ext::FollowSymlinks::No);
    let mut file = dir.open_with(path, &options).map_err(crate::error::MoltenError::from)?;
    read_bounded(&mut file, max_bytes)
}

fn read_bounded(reader: &mut impl Read, max_bytes: u64) -> crate::error::Result<Vec<u8>> {
    let read_limit = max_bytes.checked_add(1).ok_or_else(|| invalid("materialization read bound overflow"))?;
    let mut bytes = Vec::new();
    reader.take(read_limit).read_to_end(&mut bytes).map_err(crate::error::MoltenError::from)?;
    if u64::try_from(bytes.len()).map_err(|_| invalid("materialization read size does not fit u64"))? > max_bytes {
        return Err(invalid("materialization read exceeded configured byte bound"));
    }
    Ok(bytes)
}

struct PublicationState {
    final_path: std::path::PathBuf,
    backup_path: Option<std::path::PathBuf>,
    published: bool,
}

fn restore_current_backup(dir: &cap_std::fs::Dir, state: &PublicationState) -> crate::error::Result<()> {
    let Some(backup) = state.backup_path.as_ref() else {
        return Ok(());
    };
    dir.rename(backup, dir, &state.final_path).map_err(crate::error::MoltenError::from)
}

fn setup_failure(
    dir: &cap_std::fs::Dir,
    created_directories: &[std::path::PathBuf],
    primary: crate::error::MoltenError,
) -> crate::error::MoltenError {
    match rollback_created_directories(dir, created_directories) {
        Ok(()) => primary,
        Err(rollback) => invalid(format!(
            "materialization publication setup failed: {primary}; directory rollback failed: {rollback}"
        )),
    }
}

fn publication_failure(
    dir: &cap_std::fs::Dir,
    current: &PublicationState,
    prior: &[PublicationState],
    created_directories: &[std::path::PathBuf],
    primary: crate::error::MoltenError,
) -> crate::error::MoltenError {
    let current_result = restore_current_backup(dir, current);
    let prior_result = rollback_publication(dir, prior);
    let directory_result = rollback_created_directories(dir, created_directories);
    match (current_result, prior_result, directory_result) {
        (Ok(()), Ok(()), Ok(())) => primary,
        (current, prior, directories) => invalid(format!(
            "materialization publication failed: {primary}; current rollback: {current:?}; prior rollback: {prior:?}; directory rollback: {directories:?}"
        )),
    }
}

fn rollback_failure(
    dir: &cap_std::fs::Dir,
    states: &[PublicationState],
    created_directories: &[std::path::PathBuf],
    primary: crate::error::MoltenError,
) -> crate::error::MoltenError {
    let publication_result = rollback_publication(dir, states);
    let directory_result = rollback_created_directories(dir, created_directories);
    match (publication_result, directory_result) {
        (Ok(()), Ok(())) => primary,
        (publication, directories) => invalid(format!(
            "materialization verification failed: {primary}; publication rollback: {publication:?}; directory rollback: {directories:?}"
        )),
    }
}

fn rollback_created_directories(
    dir: &cap_std::fs::Dir,
    created_directories: &[std::path::PathBuf],
) -> crate::error::Result<()> {
    let mut diagnostics = Vec::with_capacity(created_directories.len());
    for directory in created_directories.iter().rev() {
        match dir.remove_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => diagnostics.push(format!("remove directory {}: {error}", directory.display())),
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!("materialization directory rollback failed: {}", diagnostics.join("; "))))
    }
}

fn rollback_publication(dir: &cap_std::fs::Dir, states: &[PublicationState]) -> crate::error::Result<()> {
    // Each publication state reports at most one remove and one restore failure.
    const DIAGNOSTICS_PER_STATE: usize = 2;
    let mut diagnostics = Vec::with_capacity(states.len().saturating_mul(DIAGNOSTICS_PER_STATE));
    for state in states.iter().rev() {
        if state.published {
            match dir.remove_file(&state.final_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => diagnostics.push(format!("remove {}: {error}", state.final_path.display())),
            }
        }
        if let Some(backup) = state.backup_path.as_ref()
            && let Err(error) = dir.rename(backup, dir, &state.final_path)
        {
            diagnostics.push(format!("restore {}: {error}", state.final_path.display()));
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(invalid(format!("materialization rollback failed: {}", diagnostics.join("; "))))
    }
}

fn invalid(message: impl Into<String>) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(message.into())
}
