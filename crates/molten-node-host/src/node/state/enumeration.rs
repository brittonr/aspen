pub(super) fn entry_kind(file_type: &cap_std::fs::FileType) -> super::authority::NodeStateEntryKind {
    if file_type.is_file() {
        super::authority::NodeStateEntryKind::RegularFile
    } else if file_type.is_dir() {
        super::authority::NodeStateEntryKind::Directory
    } else if file_type.is_symlink() {
        super::authority::NodeStateEntryKind::Symlink
    } else {
        super::authority::NodeStateEntryKind::Other
    }
}

struct EntryBinding<'a> {
    root: &'a std::sync::Arc<super::authority::RootDirectory>,
    namespace: super::authority::NodeStateNamespaceKind,
    scope: &'a std::path::Path,
}

pub(super) fn list_entries(
    dir: &cap_std::fs::Dir,
    root: &std::sync::Arc<super::authority::RootDirectory>,
    namespace: super::authority::NodeStateNamespaceKind,
    scope: &std::path::Path,
) -> crate::error::Result<Vec<super::namespace::NodeStateEntry>> {
    let binding = EntryBinding { root, namespace, scope };
    let mut entries = Vec::new();
    for entry_result in dir.read_dir(".").map_err(crate::error::MoltenError::from)? {
        let entry = entry_result.map_err(crate::error::MoltenError::from)?;
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

fn admit_entry(entry: &cap_std::fs::DirEntry) -> crate::error::Result<(String, super::locator::NodeStatePath)> {
    let file_name = entry.file_name();
    let name = file_name
        .to_str()
        .ok_or_else(|| super::invalid("node state entry name must be valid UTF-8"))?
        .to_string();
    let path = super::locator::NodeStatePath::parse(&name)?;
    Ok((name, path))
}

fn bind_entry(
    entry: &cap_std::fs::DirEntry,
    binding: &EntryBinding<'_>,
    name: String,
    path: super::locator::NodeStatePath,
) -> crate::error::Result<super::namespace::NodeStateEntry> {
    Ok(super::namespace::NodeStateEntry {
        root: std::sync::Arc::clone(binding.root),
        namespace: binding.namespace,
        scope: binding.scope.to_path_buf(),
        name,
        path,
        kind: entry_kind(&entry.file_type().map_err(crate::error::MoltenError::from)?),
    })
}
