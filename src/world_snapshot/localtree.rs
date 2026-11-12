use molten_node_host::node_state::EntryKind;
use molten_node_host::node_state::Scope;

use crate::error::Failure;
use crate::error::Result;

pub(super) const MAX_LOCAL_FILE_BYTES: u64 = 16 * 1_048_576;
const MAX_LOCAL_TREE_BYTES: u64 = 256 * 1_048_576;
const MAX_LOCAL_FILES: usize = 4_096;
const COMPONENT_DOMAIN: &str = "onixresearch.molten.world-snapshot.local-component.v1";

struct Tree {
    files: Vec<(String, Vec<u8>)>,
}

pub(super) fn collect_tree(scope: Scope) -> Result<Vec<(String, Vec<u8>)>> {
    let mut tree = Tree { files: Vec::new() };
    let mut pending = vec![(scope, String::new())];
    while let Some((current, prefix)) = pending.pop() {
        for entry in current.list_entries()? {
            if entry.name.starts_with('.') || entry.name.contains('/') {
                return Err(Failure::invalid_harness("local snapshot bundle contains an unexpected name"));
            }
            let relative = format!("{prefix}{}", entry.name);
            match entry.kind {
                EntryKind::RegularFile => {
                    tree.files.push((relative, current.read_entry(&entry, MAX_LOCAL_FILE_BYTES)?));
                }
                EntryKind::Directory => {
                    let child = current
                        .existing_subdir(&entry.path)?
                        .ok_or_else(|| Failure::invalid_harness("local snapshot bundle directory disappeared"))?;
                    pending.push((child, format!("{relative}/")));
                }
                _ => return Err(Failure::invalid_harness("local snapshot bundle contains a non-regular entry")),
            }
        }
    }
    let total = tree.files.iter().try_fold(0_u64, |total, (_, bytes)| total.checked_add(bytes.len() as u64));
    if total.is_none_or(|total| total > MAX_LOCAL_TREE_BYTES) || tree.files.len() > MAX_LOCAL_FILES {
        return Err(Failure::invalid_harness("local snapshot tree exceeds its bound"));
    }
    tree.files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(tree.files)
}

pub(super) fn tree_identity(files: &[(String, Vec<u8>)]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(COMPONENT_DOMAIN);
    for (path, bytes) in files {
        hasher.update(&(path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}
