pub(super) fn entry_kind(file_type: &cap_std::fs::FileType) -> super::authority::EntryKind {
    if file_type.is_file() {
        super::authority::EntryKind::RegularFile
    } else if file_type.is_dir() {
        super::authority::EntryKind::Directory
    } else if file_type.is_symlink() {
        super::authority::EntryKind::Symlink
    } else {
        super::authority::EntryKind::Other
    }
}

struct EntryBinding<'a> {
    root: &'a std::sync::Arc<super::authority::RootDirectory>,
    namespace: super::authority::NamespaceKind,
    scope: &'a std::path::Path,
}

pub(super) fn list_entries(
    dir: &cap_std::fs::Dir,
    root: &std::sync::Arc<super::authority::RootDirectory>,
    namespace: super::authority::NamespaceKind,
    scope: &std::path::Path,
) -> crate::error::Result<Vec<super::namespace::DirectoryEntry>> {
    let binding = EntryBinding { root, namespace, scope };
    let mut entries = Vec::new();
    for entry_result in dir.read_dir(".").map_err(crate::error::Failure::from)? {
        let entry = entry_result.map_err(crate::error::Failure::from)?;
        if entries.len() >= super::MAX_NODE_STATE_ENTRIES {
            return Err(super::invalid(format!(
                "node state entry count exceeds maximum {}",
                super::MAX_NODE_STATE_ENTRIES
            )));
        }
        let (name, path) = admit_entry(&entry)?;
        entries.push(bind_entry(&entry, &binding, name, path)?);
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

fn admit_entry(entry: &cap_std::fs::DirEntry) -> crate::error::Result<(String, super::locator::RelativePath)> {
    let file_name = entry.file_name();
    let name = file_name
        .to_str()
        .ok_or_else(|| super::invalid("node state entry name must be valid UTF-8"))?
        .to_string();
    let path = super::locator::RelativePath::parse(&name)?;
    Ok((name, path))
}

fn bind_entry(
    entry: &cap_std::fs::DirEntry,
    binding: &EntryBinding<'_>,
    name: String,
    path: super::locator::RelativePath,
) -> crate::error::Result<super::namespace::DirectoryEntry> {
    Ok(super::namespace::DirectoryEntry {
        root: std::sync::Arc::clone(binding.root),
        namespace: binding.namespace,
        scope: binding.scope.to_path_buf(),
        name,
        path,
        kind: entry_kind(&entry.file_type().map_err(crate::error::Failure::from)?),
    })
}
