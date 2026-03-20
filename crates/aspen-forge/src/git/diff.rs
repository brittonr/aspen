//! Tree and commit diff engine.
//!
//! Compares two `TreeObject`s using a sorted two-pointer walk to produce
//! structured `DiffEntry` records. Recurses into subdirectories, with
//! early-out when subtree hashes match.

use aspen_blob::prelude::*;
use serde::Deserialize;
use serde::Serialize;

use super::object::TreeEntry;
use super::object::TreeObject;
use super::store::GitBlobStore;
use crate::constants::MAX_DIFF_BLOB_SIZE;
use crate::constants::MAX_DIFF_ENTRIES;
use crate::error::ForgeResult;

/// Kind of diff entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    /// File was added.
    Added,
    /// File was removed.
    Removed,
    /// File was modified (content or mode changed).
    Modified,
}

/// A single entry in a diff result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Full path from repo root (e.g., "src/main.rs").
    pub path: String,
    /// Kind of change.
    pub kind: DiffKind,
    /// Old file mode (None for Added).
    pub old_mode: Option<u32>,
    /// New file mode (None for Removed).
    pub new_mode: Option<u32>,
    /// Old BLAKE3 hash (None for Added).
    pub old_hash: Option<[u8; 32]>,
    /// New BLAKE3 hash (None for Removed).
    pub new_hash: Option<[u8; 32]>,
    /// Old file content (populated on request, None for large blobs or Added).
    #[serde(skip)]
    pub old_content: Option<Vec<u8>>,
    /// New file content (populated on request, None for large blobs or Removed).
    #[serde(skip)]
    pub new_content: Option<Vec<u8>>,
}

/// Options for diff computation.
#[derive(Debug, Clone, Default)]
pub struct DiffOptions {
    /// Whether to load blob content for modified/added/removed files.
    pub include_content: bool,
}

/// Result of a diff operation.
#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    /// Changed entries, sorted by path.
    pub entries: Vec<DiffEntry>,
    /// Whether the result was truncated due to MAX_DIFF_ENTRIES.
    pub truncated: bool,
}

/// Compute a structured diff between two trees.
///
/// Uses a sorted two-pointer walk. Recurses into subdirectories.
/// Skips entire subtrees when their root hashes match.
///
/// # Arguments
///
/// * `store` - Git blob store for fetching subtrees and blobs
/// * `old_tree` - Left side of the diff
/// * `new_tree` - Right side of the diff
/// * `opts` - Diff options (content loading, etc.)
pub async fn diff_trees<B: BlobStore>(
    store: &GitBlobStore<B>,
    old_tree: &TreeObject,
    new_tree: &TreeObject,
    opts: &DiffOptions,
) -> ForgeResult<DiffResult> {
    let mut entries = Vec::new();
    let mut truncated = false;

    diff_trees_recursive(store, old_tree, new_tree, "", opts, &mut entries, &mut truncated).await?;

    // Sort by path for deterministic output
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(DiffResult { entries, truncated })
}

/// Compute a diff between two commits.
///
/// Resolves each commit to its root tree and delegates to `diff_trees`.
pub async fn diff_commits<B: BlobStore>(
    store: &GitBlobStore<B>,
    old_commit: &blake3::Hash,
    new_commit: &blake3::Hash,
    opts: &DiffOptions,
) -> ForgeResult<DiffResult> {
    let old = store.get_commit(old_commit).await?;
    let new = store.get_commit(new_commit).await?;

    let old_tree = store.get_tree(&old.tree()).await?;
    let new_tree = store.get_tree(&new.tree()).await?;

    diff_trees(store, &old_tree, &new_tree, opts).await
}

/// Recursive tree diff implementation.
///
/// Tiger Style: bounded by MAX_DIFF_ENTRIES to prevent unbounded output.
fn diff_trees_recursive<'a, B: BlobStore>(
    store: &'a GitBlobStore<B>,
    old_tree: &'a TreeObject,
    new_tree: &'a TreeObject,
    prefix: &'a str,
    opts: &'a DiffOptions,
    entries: &'a mut Vec<DiffEntry>,
    truncated: &'a mut bool,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ForgeResult<()>> + 'a>> {
    Box::pin(async move {
        let old_entries = &old_tree.entries;
        let new_entries = &new_tree.entries;

        let mut oi = 0usize;
        let mut ni = 0usize;

        while oi < old_entries.len() || ni < new_entries.len() {
            // Check truncation limit
            if entries.len() as u32 >= MAX_DIFF_ENTRIES {
                *truncated = true;
                return Ok(());
            }

            match (old_entries.get(oi), new_entries.get(ni)) {
                (Some(old), Some(new)) => {
                    match old.name.cmp(&new.name) {
                        std::cmp::Ordering::Less => {
                            // Entry only in old — removed
                            collect_removed(store, old, prefix, opts, entries, truncated).await?;
                            oi += 1;
                        }
                        std::cmp::Ordering::Greater => {
                            // Entry only in new — added
                            collect_added(store, new, prefix, opts, entries, truncated).await?;
                            ni += 1;
                        }
                        std::cmp::Ordering::Equal => {
                            // Same name in both — check for changes
                            if old.hash != new.hash || old.mode != new.mode {
                                diff_matching_entries(store, old, new, prefix, opts, entries, truncated).await?;
                            }
                            // If hash and mode are the same, skip (unchanged)
                            oi += 1;
                            ni += 1;
                        }
                    }
                }
                (Some(old), None) => {
                    collect_removed(store, old, prefix, opts, entries, truncated).await?;
                    oi += 1;
                }
                (None, Some(new)) => {
                    collect_added(store, new, prefix, opts, entries, truncated).await?;
                    ni += 1;
                }
                (None, None) => break,
            }
        }

        Ok(())
    }) // Box::pin
}

/// Handle two entries with the same name but different content/mode.
async fn diff_matching_entries<B: BlobStore>(
    store: &GitBlobStore<B>,
    old: &TreeEntry,
    new: &TreeEntry,
    prefix: &str,
    opts: &DiffOptions,
    entries: &mut Vec<DiffEntry>,
    truncated: &mut bool,
) -> ForgeResult<()> {
    let both_dirs = old.is_directory() && new.is_directory();
    let old_dir_new_file = old.is_directory() && !new.is_directory();
    let old_file_new_dir = !old.is_directory() && new.is_directory();

    if both_dirs {
        // Recurse into subdirectories
        let old_subtree = store.get_tree(&old.hash()).await?;
        let new_subtree = store.get_tree(&new.hash()).await?;
        let sub_prefix = format_path(prefix, &old.name);
        diff_trees_recursive(store, &old_subtree, &new_subtree, &sub_prefix, opts, entries, truncated).await?;
    } else if old_dir_new_file || old_file_new_dir {
        // Type changed (dir → file or file → dir): treat as remove + add
        collect_removed(store, old, prefix, opts, entries, truncated).await?;
        if !*truncated {
            collect_added(store, new, prefix, opts, entries, truncated).await?;
        }
    } else {
        // Both files, content or mode changed
        let path = format_path(prefix, &old.name);
        let (old_content, new_content) = if opts.include_content {
            load_content_pair(store, old, new).await?
        } else {
            (None, None)
        };

        entries.push(DiffEntry {
            path,
            kind: DiffKind::Modified,
            old_mode: Some(old.mode),
            new_mode: Some(new.mode),
            old_hash: Some(old.hash),
            new_hash: Some(new.hash),
            old_content,
            new_content,
        });
    }

    Ok(())
}

/// Collect all entries under a removed tree entry (file or directory).
async fn collect_removed<B: BlobStore>(
    store: &GitBlobStore<B>,
    entry: &TreeEntry,
    prefix: &str,
    opts: &DiffOptions,
    entries: &mut Vec<DiffEntry>,
    truncated: &mut bool,
) -> ForgeResult<()> {
    if *truncated {
        return Ok(());
    }

    if entry.is_directory() {
        // Recurse into directory and mark all contents as removed
        let subtree = store.get_tree(&entry.hash()).await?;
        let sub_prefix = format_path(prefix, &entry.name);
        for child in &subtree.entries {
            if entries.len() as u32 >= MAX_DIFF_ENTRIES {
                *truncated = true;
                return Ok(());
            }
            // Use Box::pin for recursive async
            Box::pin(collect_removed(store, child, &sub_prefix, opts, entries, truncated)).await?;
        }
    } else {
        let path = format_path(prefix, &entry.name);
        let old_content = if opts.include_content {
            load_content_if_small(store, entry).await?
        } else {
            None
        };

        entries.push(DiffEntry {
            path,
            kind: DiffKind::Removed,
            old_mode: Some(entry.mode),
            new_mode: None,
            old_hash: Some(entry.hash),
            new_hash: None,
            old_content,
            new_content: None,
        });
    }

    Ok(())
}

/// Collect all entries under an added tree entry (file or directory).
async fn collect_added<B: BlobStore>(
    store: &GitBlobStore<B>,
    entry: &TreeEntry,
    prefix: &str,
    opts: &DiffOptions,
    entries: &mut Vec<DiffEntry>,
    truncated: &mut bool,
) -> ForgeResult<()> {
    if *truncated {
        return Ok(());
    }

    if entry.is_directory() {
        let subtree = store.get_tree(&entry.hash()).await?;
        let sub_prefix = format_path(prefix, &entry.name);
        for child in &subtree.entries {
            if entries.len() as u32 >= MAX_DIFF_ENTRIES {
                *truncated = true;
                return Ok(());
            }
            Box::pin(collect_added(store, child, &sub_prefix, opts, entries, truncated)).await?;
        }
    } else {
        let path = format_path(prefix, &entry.name);
        let new_content = if opts.include_content {
            load_content_if_small(store, entry).await?
        } else {
            None
        };

        entries.push(DiffEntry {
            path,
            kind: DiffKind::Added,
            old_mode: None,
            new_mode: Some(entry.mode),
            old_hash: None,
            new_hash: Some(entry.hash),
            old_content: None,
            new_content,
        });
    }

    Ok(())
}

/// Load content for both sides of a modified file, respecting size limits.
async fn load_content_pair<B: BlobStore>(
    store: &GitBlobStore<B>,
    old: &TreeEntry,
    new: &TreeEntry,
) -> ForgeResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
    let old_content = load_content_if_small(store, old).await?;
    let new_content = load_content_if_small(store, new).await?;
    Ok((old_content, new_content))
}

/// Load blob content if it's smaller than MAX_DIFF_BLOB_SIZE.
async fn load_content_if_small<B: BlobStore>(
    store: &GitBlobStore<B>,
    entry: &TreeEntry,
) -> ForgeResult<Option<Vec<u8>>> {
    if entry.is_directory() {
        return Ok(None);
    }

    let content = store.get_blob(&entry.hash()).await?;
    if content.len() as u64 > MAX_DIFF_BLOB_SIZE {
        Ok(None)
    } else {
        Ok(Some(content))
    }
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
    async fn test_diff_identical_trees() {
        let store = test_store();
        let hash = store.store_blob(b"content".to_vec()).await.unwrap();
        let tree = store.create_tree(&[TreeEntry::file("a.txt", hash)]).await.unwrap();
        let tree_obj = store.get_tree(&tree).await.unwrap();

        let result = diff_trees(&store, &tree_obj, &tree_obj, &DiffOptions::default()).await.unwrap();
        assert!(result.entries.is_empty());
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn test_diff_added_file() {
        let store = test_store();
        let h1 = store.store_blob(b"readme".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"license".to_vec()).await.unwrap();

        let old = store.create_tree(&[TreeEntry::file("README.md", h1)]).await.unwrap();
        let new = store
            .create_tree(&[TreeEntry::file("LICENSE", h2), TreeEntry::file("README.md", h1)])
            .await
            .unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "LICENSE");
        assert_eq!(result.entries[0].kind, DiffKind::Added);
    }

    #[tokio::test]
    async fn test_diff_removed_file() {
        let store = test_store();
        let h1 = store.store_blob(b"readme".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"license".to_vec()).await.unwrap();

        let old = store
            .create_tree(&[TreeEntry::file("LICENSE", h2), TreeEntry::file("README.md", h1)])
            .await
            .unwrap();
        let new = store.create_tree(&[TreeEntry::file("README.md", h1)]).await.unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "LICENSE");
        assert_eq!(result.entries[0].kind, DiffKind::Removed);
    }

    #[tokio::test]
    async fn test_diff_modified_file() {
        let store = test_store();
        let h1 = store.store_blob(b"v1".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"v2".to_vec()).await.unwrap();

        let old = store.create_tree(&[TreeEntry::file("main.rs", h1)]).await.unwrap();
        let new = store.create_tree(&[TreeEntry::file("main.rs", h2)]).await.unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "main.rs");
        assert_eq!(result.entries[0].kind, DiffKind::Modified);
        assert_eq!(result.entries[0].old_hash, Some(*h1.as_bytes()));
        assert_eq!(result.entries[0].new_hash, Some(*h2.as_bytes()));
    }

    #[tokio::test]
    async fn test_diff_nested_directory() {
        let store = test_store();
        let h1 = store.store_blob(b"lib".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"util".to_vec()).await.unwrap();

        let old_src = store.create_tree(&[TreeEntry::file("lib.rs", h1)]).await.unwrap();
        let new_src =
            store.create_tree(&[TreeEntry::file("lib.rs", h1), TreeEntry::file("util.rs", h2)]).await.unwrap();

        let old = store.create_tree(&[TreeEntry::directory("src", old_src)]).await.unwrap();
        let new = store.create_tree(&[TreeEntry::directory("src", new_src)]).await.unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "src/util.rs");
        assert_eq!(result.entries[0].kind, DiffKind::Added);
    }

    #[tokio::test]
    async fn test_diff_unchanged_subtree_skipped() {
        let store = test_store();
        let h1 = store.store_blob(b"vendor_file".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"app_v1".to_vec()).await.unwrap();
        let h3 = store.store_blob(b"app_v2".to_vec()).await.unwrap();

        let vendor = store.create_tree(&[TreeEntry::file("dep.rs", h1)]).await.unwrap();

        let old = store
            .create_tree(&[TreeEntry::file("app.rs", h2), TreeEntry::directory("vendor", vendor)])
            .await
            .unwrap();
        let new = store
            .create_tree(&[TreeEntry::file("app.rs", h3), TreeEntry::directory("vendor", vendor)])
            .await
            .unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        // Only app.rs changed, vendor/ is skipped because same hash
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "app.rs");
    }

    #[tokio::test]
    async fn test_diff_mode_change() {
        let store = test_store();
        let h1 = store.store_blob(b"#!/bin/bash\necho hi".to_vec()).await.unwrap();

        let old = store.create_tree(&[TreeEntry::file("deploy.sh", h1)]).await.unwrap();
        let new = store.create_tree(&[TreeEntry::executable("deploy.sh", h1)]).await.unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "deploy.sh");
        assert_eq!(result.entries[0].kind, DiffKind::Modified);
        assert_eq!(result.entries[0].old_mode, Some(0o100644));
        assert_eq!(result.entries[0].new_mode, Some(0o100755));
    }

    #[tokio::test]
    async fn test_diff_with_content() {
        let store = test_store();
        let h1 = store.store_blob(b"old content".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"new content".to_vec()).await.unwrap();

        let old = store.create_tree(&[TreeEntry::file("f.txt", h1)]).await.unwrap();
        let new = store.create_tree(&[TreeEntry::file("f.txt", h2)]).await.unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions { include_content: true }).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].old_content.as_deref(), Some(b"old content".as_slice()));
        assert_eq!(result.entries[0].new_content.as_deref(), Some(b"new content".as_slice()));
    }

    #[tokio::test]
    async fn test_diff_sorted_by_path() {
        let store = test_store();
        let h1 = store.store_blob(b"a".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"b".to_vec()).await.unwrap();
        let h3 = store.store_blob(b"c".to_vec()).await.unwrap();

        let old = store.create_tree(&[]).await.unwrap();
        let new = store
            .create_tree(&[
                TreeEntry::file("README.md", h1),
                TreeEntry::file("src/a.rs", h2),
                TreeEntry::file("src/z.rs", h3),
            ])
            .await
            .unwrap();

        let old_tree = store.get_tree(&old).await.unwrap();
        let new_tree = store.get_tree(&new).await.unwrap();

        let result = diff_trees(&store, &old_tree, &new_tree, &DiffOptions::default()).await.unwrap();
        let paths: Vec<&str> = result.entries.iter().map(|e| e.path.as_str()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }

    #[tokio::test]
    async fn test_diff_commits() {
        let store = test_store();
        let h1 = store.store_blob(b"v1".to_vec()).await.unwrap();
        let h2 = store.store_blob(b"v2".to_vec()).await.unwrap();

        let t1 = store.create_tree(&[TreeEntry::file("f.txt", h1)]).await.unwrap();
        let t2 = store.create_tree(&[TreeEntry::file("f.txt", h2)]).await.unwrap();

        let c1 = store.commit(t1, vec![], "commit 1").await.unwrap();
        let c2 = store.commit(t2, vec![c1], "commit 2").await.unwrap();

        let result = diff_commits(&store, &c1, &c2, &DiffOptions::default()).await.unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].path, "f.txt");
        assert_eq!(result.entries[0].kind, DiffKind::Modified);
    }
}
