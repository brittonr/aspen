//! Internal helper functions for PijulStore.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::PathBuf;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;

use super::PijulStore;
use crate::error::PijulResult;
use crate::types::ChangeHash;
use crate::types::ChangeMetadata;

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Get the path to a repository's pristine database.
    pub(super) fn pristine_path(&self, repo_id: &RepoId) -> PathBuf {
        self.data_dir.join("pijul").join(repo_id.to_string()).join("pristine")
    }

    /// Get the set of changes already applied to a channel in the local pristine.
    ///
    /// Note: This is a simplified implementation that returns an empty set,
    /// relying on libpijul to handle duplicate application gracefully.
    /// A more optimized version could query the pristine's changeset.
    pub(super) fn get_local_channel_changes(
        &self,
        _pristine: &crate::pristine::PristineHandle,
        _channel: &str,
    ) -> PijulResult<HashSet<ChangeHash>> {
        // For now, return empty set - libpijul will handle duplicates internally
        // by checking if a change is already applied before applying it again.
        // This is safe but may be less efficient for large changelogs.
        Ok(HashSet::new())
    }

    /// Order changes by dependencies (oldest/no-deps first).
    pub(super) fn order_changes_by_dependencies(
        &self,
        log: &[ChangeMetadata],
        target_hashes: &[ChangeHash],
    ) -> Vec<ChangeHash> {
        let target_set: HashSet<_> = target_hashes.iter().copied().collect();
        let meta_map: HashMap<_, _> = log.iter().map(|m| (m.hash, m)).collect();

        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<ChangeHash, usize> = HashMap::new();
        let mut deps_of: HashMap<ChangeHash, Vec<ChangeHash>> = HashMap::new();

        for &hash in &target_set {
            in_degree.entry(hash).or_insert(0);
            if let Some(meta) = meta_map.get(&hash) {
                for dep in &meta.dependencies {
                    if target_set.contains(dep) {
                        *in_degree.entry(hash).or_insert(0) += 1;
                        deps_of.entry(*dep).or_default().push(hash);
                    }
                }
            }
        }

        // Start with changes that have no dependencies in target set
        let mut queue: VecDeque<_> =
            in_degree.iter().filter(|&(_, degree)| *degree == 0).map(|(&hash, _)| hash).collect();

        let mut ordered = Vec::with_capacity(target_hashes.len());

        while let Some(hash) = queue.pop_front() {
            ordered.push(hash);
            if let Some(dependents) = deps_of.get(&hash) {
                for &dep in dependents {
                    if let Some(degree) = in_degree.get_mut(&dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        ordered
    }
}

/// Extract conflict information from a libpijul Conflict.
///
/// This attempts to extract the file path and involved change hashes
/// from libpijul's conflict representation.
///
/// # Returns
///
/// A tuple of (file_path, involved_changes) where:
/// - file_path: The path of the conflicting file
/// - involved_changes: List of ChangeHash values involved in the conflict
pub(super) fn extract_conflict_info(conflict: &libpijul::output::Conflict) -> (String, Vec<ChangeHash>) {
    use libpijul::output::Conflict;

    match conflict {
        // Name conflicts involve file path conflicts
        Conflict::Name { path, .. } => {
            let path_str = path.clone();
            // Name conflicts don't have specific change hashes easily accessible
            (path_str, Vec::new())
        }

        // Zombie conflicts are more complex - represent content conflicts
        Conflict::ZombieFile { path, .. } => {
            let path_str = path.clone();
            // Zombie conflicts contain change information but require deeper analysis
            // For now, return empty list - full extraction would require traversing
            // the conflict graph which is complex
            (path_str, Vec::new())
        }

        // Order conflicts involve ordering dependencies
        Conflict::Order { path, .. } => {
            let path_str = path.clone();
            // Order conflicts also contain change information that's difficult to extract
            (path_str, Vec::new())
        }

        // Cyclical order conflicts
        Conflict::Cyclic { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }

        // Multiple names conflict
        Conflict::MultipleNames { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }

        // Zombie conflicts (alternative form)
        Conflict::Zombie { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }
    }
}
