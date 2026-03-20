//! Three-way tree merge engine.
//!
//! Merges two trees against a common base using `classify_three_way` from
//! the verified module. Recurses into subdirectories, skips unchanged
//! subtrees, and reports conflicts.

use std::collections::BTreeMap;

use aspen_blob::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use super::object::TreeEntry;
use super::object::TreeObject;
use super::store::GitBlobStore;
use crate::constants::MAX_MERGE_CONFLICTS;
use crate::constants::MAX_MERGE_DEPTH;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::verified::merge::ThreeWayClass;
use crate::verified::merge::classify_three_way;

/// Kind of merge conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictKind {
    /// Both sides modified the same file to different content.
    BothModified,
    /// One side modified, the other deleted.
    ModifyDelete,
    /// Both sides added the same path with different content.
    BothAdded,
}

/// A conflict detected during three-way merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeConflict {
    /// Full path from repo root.
    pub path: String,
    /// Kind of conflict.
    pub kind: ConflictKind,
    /// Base entry hash (None if file didn't exist in base).
    pub base: Option<[u8; 32]>,
    /// Ours entry hash (None if file was deleted in ours).
    pub ours: Option<[u8; 32]>,
    /// Theirs entry hash (None if file was deleted in theirs).
    pub theirs: Option<[u8; 32]>,
}

/// Result of a three-way tree merge.
#[derive(Debug, Clone)]
pub struct TreeMergeResult {
    /// The merged tree (Some if merge was clean, None if conflicts).
    pub tree: Option<TreeObject>,
    /// List of conflicts (empty if merge was clean).
    pub conflicts: Vec<MergeConflict>,
    /// Whether the conflict list was truncated.
    pub truncated: bool,
}

impl TreeMergeResult {
    /// Check if the merge was clean (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Create a clean merge result.
    pub fn clean(tree: TreeObject) -> Self {
        Self {
            tree: Some(tree),
            conflicts: Vec::new(),
            truncated: false,
        }
    }

    /// Create a conflicted merge result.
    pub fn conflicted(conflicts: Vec<MergeConflict>, truncated: bool) -> Self {
        Self {
            tree: None,
            conflicts,
            truncated,
        }
    }
}

/// Three-way merge of two trees against a common base.
///
/// For each entry name across the three trees, uses `classify_three_way`
/// to determine the merge action. Recurses into subdirectories.
///
/// On clean merge, writes the merged tree to the blob store and returns it.
/// On conflict, returns the list of conflicting entries.
///
/// # Arguments
///
/// * `store` - Git blob store for reading/writing trees
/// * `base` - Common ancestor tree
/// * `ours` - Our side of the merge
/// * `theirs` - Their side of the merge
pub async fn merge_trees<B: BlobStore>(
    store: &GitBlobStore<B>,
    base: &TreeObject,
    ours: &TreeObject,
    theirs: &TreeObject,
) -> ForgeResult<TreeMergeResult> {
    let mut conflicts = Vec::new();
    let mut truncated = false;

    let result = merge_trees_recursive(store, base, ours, theirs, "", 0, &mut conflicts, &mut truncated).await?;

    if conflicts.is_empty() {
        match result {
            Some(entries) => {
                let tree = TreeObject::new(entries);
                // Write the merged tree to the blob store
                store.create_tree(&tree.entries).await?;
                Ok(TreeMergeResult::clean(tree))
            }
            None => {
                // Should not happen if conflicts is empty, but be defensive
                Ok(TreeMergeResult::conflicted(conflicts, truncated))
            }
        }
    } else {
        Ok(TreeMergeResult::conflicted(conflicts, truncated))
    }
}

/// Boxed future type for recursive merge.
type MergeRecurseFut<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = ForgeResult<Option<Vec<TreeEntry>>>> + 'a>>;

/// Recursive three-way merge implementation.
///
/// Returns `Some(entries)` on clean merge, `None` on conflict.
/// Conflicts are accumulated in the `conflicts` vec.
#[allow(clippy::too_many_arguments)]
fn merge_trees_recursive<'a, B: BlobStore>(
    store: &'a GitBlobStore<B>,
    base: &'a TreeObject,
    ours: &'a TreeObject,
    theirs: &'a TreeObject,
    prefix: &'a str,
    depth: u32,
    conflicts: &'a mut Vec<MergeConflict>,
    truncated: &'a mut bool,
) -> MergeRecurseFut<'a> {
    Box::pin(async move {
        if depth > MAX_MERGE_DEPTH {
            return Err(ForgeError::MergeDepthExceeded {
                depth,
                max: MAX_MERGE_DEPTH,
            });
        }

        // Build maps: name -> (entry) for each tree
        let base_map = entry_map(&base.entries);
        let ours_map = entry_map(&ours.entries);
        let theirs_map = entry_map(&theirs.entries);

        // Collect all names across all three trees
        let mut all_names: Vec<&str> =
            base_map.keys().chain(ours_map.keys()).chain(theirs_map.keys()).copied().collect();
        all_names.sort_unstable();
        all_names.dedup();

        let mut merged_entries: Vec<TreeEntry> = Vec::new();
        let mut has_conflicts = false;

        for name in all_names {
            let b = base_map.get(name).copied();
            let o = ours_map.get(name).copied();
            let t = theirs_map.get(name).copied();

            let b_hash = b.map(|e| e.hash);
            let o_hash = o.map(|e| e.hash);
            let t_hash = t.map(|e| e.hash);

            let class = classify_three_way(b_hash, o_hash, t_hash);

            match class {
                ThreeWayClass::Unchanged => {
                    // Keep base entry (or ours — they're the same)
                    if let Some(entry) = o.or(b) {
                        merged_entries.push(entry.clone());
                    }
                }

                ThreeWayClass::TakeOurs => {
                    if let Some(entry) = o {
                        merged_entries.push(entry.clone());
                    }
                    // If o is None, it was deleted in ours — omit from result
                }

                ThreeWayClass::TakeTheirs => {
                    if let Some(entry) = t {
                        merged_entries.push(entry.clone());
                    }
                    // If t is None, it was deleted in theirs — omit from result
                }

                ThreeWayClass::Convergent => {
                    // Both made the same change. Take ours (or theirs — same hash).
                    if let Some(entry) = o.or(t) {
                        merged_entries.push(entry.clone());
                    }
                    // If both are None (convergent deletion), omit from result
                }

                ThreeWayClass::Conflict => {
                    // Check if both sides are directories — we can try recursive merge
                    let o_is_dir = o.is_some_and(|e| e.is_directory());
                    let t_is_dir = t.is_some_and(|e| e.is_directory());
                    let b_is_dir = b.is_some_and(|e| e.is_directory());

                    if o_is_dir && t_is_dir {
                        // Both sides are directories with different hashes — recurse
                        let base_subtree = if b_is_dir {
                            store.get_tree(&b.unwrap().hash()).await?
                        } else {
                            TreeObject::new(vec![])
                        };
                        let ours_subtree = store.get_tree(&o.unwrap().hash()).await?;
                        let theirs_subtree = store.get_tree(&t.unwrap().hash()).await?;

                        let sub_prefix = format_path(prefix, name);

                        let sub_result = merge_trees_recursive(
                            store,
                            &base_subtree,
                            &ours_subtree,
                            &theirs_subtree,
                            &sub_prefix,
                            depth + 1,
                            conflicts,
                            truncated,
                        )
                        .await?;

                        match sub_result {
                            Some(sub_entries) => {
                                let sub_tree = TreeObject::new(sub_entries);
                                let sub_hash = store.create_tree(&sub_tree.entries).await?;
                                merged_entries.push(TreeEntry::directory(name.to_string(), sub_hash));
                            }
                            None => {
                                has_conflicts = true;
                                // Conflicts already accumulated by recursive call
                            }
                        }
                    } else {
                        // File-level conflict
                        has_conflicts = true;
                        if !*truncated {
                            let kind = match (b, o, t) {
                                (None, Some(_), Some(_)) => ConflictKind::BothAdded,
                                (Some(_), None, Some(_)) | (Some(_), Some(_), None) => ConflictKind::ModifyDelete,
                                _ => ConflictKind::BothModified,
                            };

                            let path = format_path(prefix, name);
                            conflicts.push(MergeConflict {
                                path,
                                kind,
                                base: b_hash,
                                ours: o_hash,
                                theirs: t_hash,
                            });

                            if conflicts.len() as u32 >= MAX_MERGE_CONFLICTS {
                                *truncated = true;
                            }
                        }
                    }
                }
            }
        }

        if has_conflicts {
            Ok(None)
        } else {
            Ok(Some(merged_entries))
        }
    })
}

/// Build a name -> entry map from sorted entries.
fn entry_map(entries: &[TreeEntry]) -> BTreeMap<&str, &TreeEntry> {
    entries.iter().map(|e| (e.name.as_str(), e)).collect()
}

/// Build a path string by joining prefix and name.
fn format_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", prefix, name)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;

    use super::*;

    fn test_store() -> GitBlobStore<InMemoryBlobStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        GitBlobStore::new(blobs, secret_key, "test")
    }

    #[tokio::test]
    async fn test_merge_clean_non_overlapping() {
        let store = test_store();
        let h_a = store.store_blob(b"a_base".to_vec()).await.unwrap();
        let h_b = store.store_blob(b"b_base".to_vec()).await.unwrap();
        let h_a2 = store.store_blob(b"a_modified".to_vec()).await.unwrap();
        let h_b2 = store.store_blob(b"b_modified".to_vec()).await.unwrap();

        let base_tree = TreeObject::new(vec![TreeEntry::file("a.rs", h_a), TreeEntry::file("b.rs", h_b)]);
        let base_hash = store.create_tree(&base_tree.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours_tree = TreeObject::new(vec![
            TreeEntry::file("a.rs", h_a2), // ours modifies a
            TreeEntry::file("b.rs", h_b),  // ours keeps b
        ]);
        let ours_hash = store.create_tree(&ours_tree.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs_tree = TreeObject::new(vec![
            TreeEntry::file("a.rs", h_a),  // theirs keeps a
            TreeEntry::file("b.rs", h_b2), // theirs modifies b
        ]);
        let theirs_hash = store.create_tree(&theirs_tree.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean());

        let merged = result.tree.unwrap();
        assert_eq!(merged.entries.len(), 2);
        // a.rs should be ours' version, b.rs should be theirs' version
        let a_entry = merged.entries.iter().find(|e| e.name == "a.rs").unwrap();
        let b_entry = merged.entries.iter().find(|e| e.name == "b.rs").unwrap();
        assert_eq!(a_entry.hash, *h_a2.as_bytes());
        assert_eq!(b_entry.hash, *h_b2.as_bytes());
    }

    #[tokio::test]
    async fn test_merge_both_modified_conflict() {
        let store = test_store();
        let h0 = store.store_blob(b"base".to_vec()).await.unwrap();
        let h1 = store.store_blob(b"ours".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"theirs".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("config.toml", h0)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours = TreeObject::new(vec![TreeEntry::file("config.toml", h1)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![TreeEntry::file("config.toml", h2)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].path, "config.toml");
        assert_eq!(result.conflicts[0].kind, ConflictKind::BothModified);
    }

    #[tokio::test]
    async fn test_merge_convergent() {
        let store = test_store();
        let h0 = store.store_blob(b"base".to_vec()).await.unwrap();
        let h1 = store.store_blob(b"same".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("shared.rs", h0)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        // Both sides change to the same value
        let ours = TreeObject::new(vec![TreeEntry::file("shared.rs", h1)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![TreeEntry::file("shared.rs", h1)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean());
        let merged = result.tree.unwrap();
        let entry = merged.entries.iter().find(|e| e.name == "shared.rs").unwrap();
        assert_eq!(entry.hash, *h1.as_bytes());
    }

    #[tokio::test]
    async fn test_merge_add_one_side() {
        let store = test_store();
        let h_a = store.store_blob(b"a".to_vec()).await.unwrap();
        let h_new = store.store_blob(b"new".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("a.rs", h_a)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours = TreeObject::new(vec![TreeEntry::file("a.rs", h_a), TreeEntry::file("new.rs", h_new)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![TreeEntry::file("a.rs", h_a)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean());
        let merged = result.tree.unwrap();
        assert_eq!(merged.entries.len(), 2);
        assert!(merged.entries.iter().any(|e| e.name == "new.rs"));
    }

    #[tokio::test]
    async fn test_merge_modify_delete_conflict() {
        let store = test_store();
        let h0 = store.store_blob(b"base".to_vec()).await.unwrap();
        let h1 = store.store_blob(b"modified".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("lib.rs", h0)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        // Ours modifies, theirs deletes
        let ours = TreeObject::new(vec![TreeEntry::file("lib.rs", h1)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].kind, ConflictKind::ModifyDelete);
    }

    #[tokio::test]
    async fn test_merge_both_added_different_conflict() {
        let store = test_store();
        let h1 = store.store_blob(b"ours".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"theirs".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours = TreeObject::new(vec![TreeEntry::file("new.rs", h1)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![TreeEntry::file("new.rs", h2)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].kind, ConflictKind::BothAdded);
    }

    #[tokio::test]
    async fn test_merge_recursive_subdirectory() {
        let store = test_store();
        let h_a = store.store_blob(b"a".to_vec()).await.unwrap();
        let h_b = store.store_blob(b"b".to_vec()).await.unwrap();
        let h_a2 = store.store_blob(b"a_mod".to_vec()).await.unwrap();
        let h_b2 = store.store_blob(b"b_mod".to_vec()).await.unwrap();

        let base_src = store.create_tree(&[TreeEntry::file("a.rs", h_a), TreeEntry::file("b.rs", h_b)]).await.unwrap();
        let base = TreeObject::new(vec![TreeEntry::directory("src", base_src)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours_src = store.create_tree(&[TreeEntry::file("a.rs", h_a2), TreeEntry::file("b.rs", h_b)]).await.unwrap();
        let ours = TreeObject::new(vec![TreeEntry::directory("src", ours_src)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs_src =
            store.create_tree(&[TreeEntry::file("a.rs", h_a), TreeEntry::file("b.rs", h_b2)]).await.unwrap();
        let theirs = TreeObject::new(vec![TreeEntry::directory("src", theirs_src)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean(), "recursive merge should be clean: {:?}", result.conflicts);

        // Verify the merged src directory has our a.rs and their b.rs
        let merged = result.tree.unwrap();
        assert_eq!(merged.entries.len(), 1);
        assert_eq!(merged.entries[0].name, "src");

        let merged_src = store.get_tree(&merged.entries[0].hash()).await.unwrap();
        let a_entry = merged_src.entries.iter().find(|e| e.name == "a.rs").unwrap();
        let b_entry = merged_src.entries.iter().find(|e| e.name == "b.rs").unwrap();
        assert_eq!(a_entry.hash, *h_a2.as_bytes());
        assert_eq!(b_entry.hash, *h_b2.as_bytes());
    }

    #[tokio::test]
    async fn test_merge_unchanged_subtree_skipped() {
        let store = test_store();
        let h1 = store.store_blob(b"vendor".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"app_base".to_vec()).await.unwrap();
        let h3 = store.store_blob(b"app_ours".to_vec()).await.unwrap();

        let vendor = store.create_tree(&[TreeEntry::file("dep.rs", h1)]).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("app.rs", h2), TreeEntry::directory("vendor", vendor)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        let ours = TreeObject::new(vec![TreeEntry::file("app.rs", h3), TreeEntry::directory("vendor", vendor)]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        // Theirs also keeps vendor unchanged
        let theirs = TreeObject::new(vec![TreeEntry::file("app.rs", h2), TreeEntry::directory("vendor", vendor)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean());

        let merged = result.tree.unwrap();
        // vendor dir should be in the merged result with original hash
        let vendor_entry = merged.entries.iter().find(|e| e.name == "vendor").unwrap();
        assert_eq!(vendor_entry.hash, *vendor.as_bytes());
    }

    #[tokio::test]
    async fn test_merge_delete_one_side_unchanged_other() {
        let store = test_store();
        let h0 = store.store_blob(b"old".to_vec()).await.unwrap();

        let base = TreeObject::new(vec![TreeEntry::file("old.rs", h0)]);
        let base_hash = store.create_tree(&base.entries).await.unwrap();
        let base = store.get_tree(&base_hash).await.unwrap();

        // Ours deletes it, theirs keeps it unchanged
        let ours = TreeObject::new(vec![]);
        let ours_hash = store.create_tree(&ours.entries).await.unwrap();
        let ours = store.get_tree(&ours_hash).await.unwrap();

        let theirs = TreeObject::new(vec![TreeEntry::file("old.rs", h0)]);
        let theirs_hash = store.create_tree(&theirs.entries).await.unwrap();
        let theirs = store.get_tree(&theirs_hash).await.unwrap();

        let result = merge_trees(&store, &base, &ours, &theirs).await.unwrap();
        assert!(result.is_clean());

        let merged = result.tree.unwrap();
        // File should be deleted (ours removed it, theirs didn't change it)
        assert!(merged.entries.is_empty());
    }
}
