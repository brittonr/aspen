use std::path::Path;
use std::sync::Arc;

use super::MAX_NODE_STATE_ENTRIES;
use super::authority::NodeStateEntryKind;
use super::authority::NodeStateInner;
use super::authority::NodeStateNamespaceKind;
use super::invalid;
use super::locator::NodeStatePath;
use super::namespace::NodeStateEntry;

pub(super) fn entry_kind(file_type: &cap_std::fs::FileType) -> NodeStateEntryKind {
    if file_type.is_file() {
        NodeStateEntryKind::RegularFile
    } else if file_type.is_dir() {
        NodeStateEntryKind::Directory
    } else if file_type.is_symlink() {
        NodeStateEntryKind::Symlink
    } else {
        NodeStateEntryKind::Other
    }
}

pub(super) fn list_entries(
    dir: &cap_std::fs::Dir,
    root: &Arc<NodeStateInner>,
    namespace: NodeStateNamespaceKind,
    scope: &Path,
) -> crate::error::Result<Vec<NodeStateEntry>> {
    let mut entries = Vec::new();
    for entry_result in dir.read_dir(".").map_err(crate::error::MoltenError::from)? {
        let entry = entry_result.map_err(crate::error::MoltenError::from)?;
        if entries.len() >= MAX_NODE_STATE_ENTRIES {
            return Err(invalid(format!("node state entry count exceeds maximum {MAX_NODE_STATE_ENTRIES}")));
        }
        let file_name = entry.file_name();
        let name = file_name.to_str().ok_or_else(|| invalid("node state entry name must be valid UTF-8"))?.to_string();
        let path = NodeStatePath::parse(&name)?;
        entries.push(NodeStateEntry {
            root: Arc::clone(root),
            namespace,
            scope: scope.to_path_buf(),
            name,
            path,
            kind: entry_kind(&entry.file_type().map_err(crate::error::MoltenError::from)?),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}
