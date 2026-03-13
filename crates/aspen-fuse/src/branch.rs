//! Branch management for @branch virtual paths in AspenFs.
//!
//! Feature-gated behind `kv-branch`.

#[cfg(feature = "kv-branch")]
use std::sync::Arc;

#[cfg(feature = "kv-branch")]
use aspen_kv_branch::BranchOverlay;
#[cfg(feature = "kv-branch")]
use dashmap::DashMap;
#[cfg(feature = "kv-branch")]
use tracing::debug;
#[cfg(feature = "kv-branch")]
use tracing::warn;

#[cfg(feature = "kv-branch")]
use crate::client::SharedClient;

/// Branch state stored per active branch.
#[cfg(feature = "kv-branch")]
pub(crate) struct BranchState {
    /// The branch overlay wrapping the shared client's KV store.
    /// Note: SharedClient (FuseSyncClient) doesn't implement KeyValueStore,
    /// so we use an InMemoryBranch that buffers writes and reads from parent via RPC.
    pub overlay: InMemoryBranch,
}

/// In-memory branch for FUSE: buffers writes locally, reads fall through
/// to the parent FuseSyncClient.
#[cfg(feature = "kv-branch")]
pub(crate) struct InMemoryBranch {
    branch_id: String,
    /// Buffered writes: key -> value (None = tombstone).
    dirty: DashMap<String, Option<Vec<u8>>>,
    /// Parent client for read fall-through.
    parent: SharedClient,
}

#[cfg(feature = "kv-branch")]
impl InMemoryBranch {
    pub fn new(branch_id: String, parent: SharedClient) -> Self {
        Self {
            branch_id,
            dirty: DashMap::new(),
            parent,
        }
    }

    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    /// Read a key: check dirty map first, then fall through to parent.
    pub fn read(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        if let Some(entry) = self.dirty.get(key) {
            return Ok(entry.value().clone());
        }
        self.parent.read_key(key)
    }

    /// Write a key: buffer in dirty map.
    pub fn write(&self, key: &str, value: &[u8]) -> anyhow::Result<bool> {
        self.dirty.insert(key.to_owned(), Some(value.to_vec()));
        Ok(true)
    }

    /// Delete a key: insert tombstone.
    pub fn delete(&self, key: &str) -> anyhow::Result<bool> {
        self.dirty.insert(key.to_owned(), None);
        Ok(true)
    }

    /// Scan keys: merge dirty entries with parent scan.
    pub fn scan(&self, prefix: &str, limit: u32) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let parent_results = self.parent.scan_keys(prefix, limit)?;

        // Collect dirty entries matching prefix.
        let mut dirty_entries: Vec<(String, Option<Vec<u8>>)> = self
            .dirty
            .iter()
            .filter(|e| e.key().starts_with(prefix))
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        dirty_entries.sort_by(|a, b| a.0.cmp(&b.0));

        // Merge: dirty overrides parent, tombstones remove.
        let mut result = Vec::new();
        let mut pi = 0;
        let mut di = 0;

        while result.len() < limit as usize && (pi < parent_results.len() || di < dirty_entries.len()) {
            let take_dirty = match (dirty_entries.get(di), parent_results.get(pi)) {
                (Some((dk, _)), Some((pk, _))) => {
                    if dk == pk {
                        pi += 1;
                        true
                    } else {
                        dk < pk
                    }
                }
                (Some(_), None) => true,
                (None, Some(_)) => false,
                (None, None) => break,
            };

            if take_dirty {
                let (key, val) = &dirty_entries[di];
                di += 1;
                if let Some(v) = val {
                    result.push((key.clone(), v.clone()));
                }
                // Tombstone: skip
            } else {
                let (key, val) = &parent_results[pi];
                pi += 1;
                // Check if tombstoned in dirty.
                if self.dirty.get(key).is_some() {
                    continue; // overridden or tombstoned
                }
                result.push((key.clone(), val.clone()));
            }
        }

        Ok(result)
    }

    /// Commit: flush all dirty entries to parent.
    pub fn commit(&self) -> anyhow::Result<()> {
        for entry in self.dirty.iter() {
            let key = entry.key();
            match entry.value() {
                Some(value) => {
                    self.parent.write_key(key, value)?;
                }
                None => {
                    self.parent.delete_key(key)?;
                }
            }
        }
        self.dirty.clear();
        debug!(branch_id = %self.branch_id, "branch committed to parent");
        Ok(())
    }

    /// Abort: discard all dirty state.
    pub fn abort(&self) {
        let count = self.dirty.len();
        self.dirty.clear();
        debug!(branch_id = %self.branch_id, dirty_count = count, "branch aborted");
    }

    /// Check if the dirty map is empty.
    pub fn is_empty(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Number of dirty entries.
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }
}

#[cfg(feature = "kv-branch")]
impl Drop for InMemoryBranch {
    fn drop(&mut self) {
        let count = self.dirty.len();
        if count > 0 {
            warn!(
                branch_id = %self.branch_id,
                dirty_count = count,
                "FUSE branch dropped with uncommitted entries"
            );
        }
    }
}

/// Branch manager: tracks active branches by name.
#[cfg(feature = "kv-branch")]
pub(crate) struct BranchManager {
    branches: DashMap<String, Arc<InMemoryBranch>>,
}

#[cfg(feature = "kv-branch")]
impl BranchManager {
    pub fn new() -> Self {
        Self {
            branches: DashMap::new(),
        }
    }

    /// Create a new branch.
    pub fn create(&self, name: &str, parent: SharedClient) -> anyhow::Result<Arc<InMemoryBranch>> {
        if self.branches.contains_key(name) {
            anyhow::bail!("branch '{}' already exists", name);
        }
        let branch = Arc::new(InMemoryBranch::new(name.to_owned(), parent));
        self.branches.insert(name.to_owned(), Arc::clone(&branch));
        debug!(branch = name, "FUSE branch created");
        Ok(branch)
    }

    /// Get an existing branch.
    pub fn get(&self, name: &str) -> Option<Arc<InMemoryBranch>> {
        self.branches.get(name).map(|e| Arc::clone(e.value()))
    }

    /// Commit and remove a branch.
    pub fn commit(&self, name: &str) -> anyhow::Result<()> {
        let branch = self.branches.remove(name).ok_or_else(|| anyhow::anyhow!("branch '{}' not found", name))?;
        branch.1.commit()?;
        Ok(())
    }

    /// Abort and remove a branch.
    pub fn abort(&self, name: &str) -> anyhow::Result<()> {
        let branch = self.branches.remove(name).ok_or_else(|| anyhow::anyhow!("branch '{}' not found", name))?;
        branch.1.abort();
        Ok(())
    }

    /// List all branch names.
    pub fn list(&self) -> Vec<String> {
        self.branches.iter().map(|e| e.key().clone()).collect()
    }

    /// Drop all branches (for unmount).
    pub fn drop_all(&self) {
        let names: Vec<String> = self.branches.iter().map(|e| e.key().clone()).collect();
        for name in &names {
            if let Some((_, branch)) = self.branches.remove(name) {
                branch.abort();
            }
        }
    }
}

/// Parse a path to detect @branch prefix.
///
/// Returns `(Some(branch_name), rest_of_path)` if the first component
/// starts with `@`, otherwise `(None, full_path)`.
pub fn parse_branch_path(path: &str) -> (Option<&str>, &str) {
    let trimmed = path.trim_start_matches('/');
    if let Some(branch_part) = trimmed.strip_prefix('@') {
        if let Some(slash_pos) = branch_part.find('/') {
            let branch_name = &branch_part[..slash_pos];
            let rest = &branch_part[slash_pos + 1..];
            (Some(branch_name), rest)
        } else {
            // Just "@branch-name" with no trailing path
            (Some(branch_part), "")
        }
    } else {
        (None, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_branch_path_with_branch() {
        let (branch, rest) = parse_branch_path("/@experiment/src/main.rs");
        assert_eq!(branch, Some("experiment"));
        assert_eq!(rest, "src/main.rs");
    }

    #[test]
    fn parse_branch_path_branch_only() {
        let (branch, rest) = parse_branch_path("/@experiment");
        assert_eq!(branch, Some("experiment"));
        assert_eq!(rest, "");
    }

    #[test]
    fn parse_branch_path_no_branch() {
        let (branch, rest) = parse_branch_path("/src/main.rs");
        assert_eq!(branch, None);
        assert_eq!(rest, "/src/main.rs");
    }

    #[test]
    fn parse_branch_path_root() {
        let (branch, rest) = parse_branch_path("/");
        assert_eq!(branch, None);
        assert_eq!(rest, "/");
    }
}
