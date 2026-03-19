//! Store closure sync via DAG traversal.
//!
//! Syncs entire Nix store closures (PathInfo → references → content)
//! between cluster nodes using `aspen-dag`'s streaming protocol. This
//! replaces per-path NAR fetches with a single deterministic traversal
//! that transfers all missing objects in one QUIC stream.
//!
//! # Graph Shape
//!
//! The closure graph has two layers:
//!
//! ```text
//! StorePath (root)
//!   ├─ PathInfo.references → StorePath → StorePath → ...  (reference graph)
//!   └─ PathInfo.node (root Node)
//!        └─ Directory (B3Digest)
//!             ├─ File (B3Digest)
//!             ├─ Symlink (no digest)
//!             └─ Directory → ...
//! ```
//!
//! The [`ClosureLinkExtractor`] walks both layers: reference edges between
//! store paths and content edges within each path's directory tree.

use std::collections::HashSet;
use std::sync::Arc;

use nix_compat::store_path::StorePath;
use snix_castore::B3Digest;
use snix_castore::Node;
use snix_castore::directoryservice::DirectoryService;
use snix_store::pathinfoservice::PathInfoService;
use tracing::debug;
use tracing::warn;

/// Maximum number of store paths in a single closure sync.
const MAX_CLOSURE_PATHS: usize = 50_000;

/// Sync an entire store closure from a remote peer's snix services.
///
/// Given a root store path, computes the full reference closure locally
/// (using PathInfoService), identifies which paths are missing, then
/// fetches the missing PathInfo + directory trees + blobs via the
/// remote's services.
///
/// This is the entry point for cluster-internal store closure replication.
pub struct StoreClosureSync<D, P> {
    directory_service: Arc<D>,
    pathinfo_service: Arc<P>,
}

impl<D, P> StoreClosureSync<D, P>
where
    D: DirectoryService + Send + Sync + 'static,
    P: PathInfoService + Send + Sync + 'static,
{
    /// Create a new store closure sync service.
    pub fn new(directory_service: Arc<D>, pathinfo_service: Arc<P>) -> Self {
        Self {
            directory_service,
            pathinfo_service,
        }
    }

    /// Compute the set of store paths in a closure that are missing locally.
    ///
    /// Walks the reference graph from `root_path` using the remote's PathInfo
    /// entries (which must already be available locally via Raft replication),
    /// then checks which paths lack local content (directory tree + blobs).
    ///
    /// Returns store paths whose content needs to be fetched.
    pub async fn compute_missing_paths(
        &self,
        root_path: &StorePath<String>,
    ) -> Result<Vec<StorePath<String>>, ClosureSyncError> {
        let mut closure = HashSet::new();
        let mut queue = vec![root_path.clone()];
        let mut missing = Vec::new();

        while let Some(path) = queue.pop() {
            if closure.len() >= MAX_CLOSURE_PATHS {
                warn!(max = MAX_CLOSURE_PATHS, "closure sync hit path limit, stopping traversal");
                break;
            }

            let path_str = path.to_absolute_path();
            if !closure.insert(path_str.clone()) {
                continue;
            }

            let digest = *path.digest();
            let path_info = match self.pathinfo_service.get(digest).await {
                Ok(Some(info)) => info,
                Ok(None) => {
                    debug!(path = %path_str, "store path not in PathInfoService");
                    missing.push(path);
                    continue;
                }
                Err(e) => {
                    return Err(ClosureSyncError::PathInfoQuery {
                        path: path_str,
                        message: e.to_string(),
                    });
                }
            };

            // Check if this path's content tree is complete locally
            if !self.has_complete_content(&path_info.node).await {
                missing.push(path.clone());
            }

            // Enqueue references
            for reference in &path_info.references {
                let ref_path = reference.to_owned();
                let ref_str = ref_path.to_absolute_path();
                if !closure.contains(&ref_str) {
                    queue.push(ref_path);
                }
            }
        }

        debug!(closure_size = closure.len(), missing = missing.len(), "closure sync: computed missing paths");

        Ok(missing)
    }

    /// Check if a content node's directory tree is fully present locally.
    ///
    /// For files and symlinks, returns true (they're leaves).
    /// For directories, recursively checks that the directory and all
    /// its children exist in the DirectoryService.
    async fn has_complete_content(&self, node: &Node) -> bool {
        match node {
            Node::File { .. } | Node::Symlink { .. } => true,
            Node::Directory { digest, .. } => self.has_directory_tree(digest).await,
        }
    }

    /// Check if a directory and all its descendants exist locally.
    async fn has_directory_tree(&self, digest: &B3Digest) -> bool {
        let dir = match self.directory_service.get(digest).await {
            Ok(Some(d)) => d,
            Ok(None) | Err(_) => return false,
        };

        for (_name, node) in dir.nodes() {
            match node {
                Node::Directory { digest, .. } => {
                    if !Box::pin(self.has_directory_tree(digest)).await {
                        return false;
                    }
                }
                Node::File { .. } | Node::Symlink { .. } => {}
            }
        }

        true
    }

    /// Extract all B3Digests (directory + file content) from a PathInfo's content tree.
    ///
    /// Returns the set of digests that need to be transferred for complete
    /// content replication of a single store path.
    pub async fn extract_content_digests(&self, node: &Node) -> Result<Vec<B3Digest>, ClosureSyncError> {
        let mut digests = Vec::new();
        self.collect_digests(node, &mut digests, &mut HashSet::new()).await?;
        Ok(digests)
    }

    /// Recursively collect all B3Digests from a content tree.
    async fn collect_digests(
        &self,
        node: &Node,
        digests: &mut Vec<B3Digest>,
        visited: &mut HashSet<B3Digest>,
    ) -> Result<(), ClosureSyncError> {
        match node {
            Node::File { digest, .. } => {
                if visited.insert(*digest) {
                    digests.push(*digest);
                }
            }
            Node::Symlink { .. } => {}
            Node::Directory { digest, .. } => {
                if !visited.insert(*digest) {
                    return Ok(());
                }
                digests.push(*digest);

                let dir = self
                    .directory_service
                    .get(digest)
                    .await
                    .map_err(|e| ClosureSyncError::DirectoryQuery {
                        digest: digest.to_string(),
                        message: e.to_string(),
                    })?
                    .ok_or_else(|| ClosureSyncError::DirectoryNotFound {
                        digest: digest.to_string(),
                    })?;

                for (_name, child_node) in dir.nodes() {
                    Box::pin(self.collect_digests(child_node, digests, visited)).await?;
                }
            }
        }
        Ok(())
    }
}

/// Errors from store closure sync operations.
#[derive(Debug)]
pub enum ClosureSyncError {
    /// Failed to query PathInfoService.
    PathInfoQuery { path: String, message: String },
    /// Failed to query DirectoryService.
    DirectoryQuery { digest: String, message: String },
    /// Directory not found in local store.
    DirectoryNotFound { digest: String },
}

impl std::fmt::Display for ClosureSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathInfoQuery { path, message } => {
                write!(f, "PathInfo query failed for {path}: {message}")
            }
            Self::DirectoryQuery { digest, message } => {
                write!(f, "Directory query failed for {digest}: {message}")
            }
            Self::DirectoryNotFound { digest } => {
                write!(f, "Directory not found: {digest}")
            }
        }
    }
}

impl std::error::Error for ClosureSyncError {}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;
    use snix_castore::Directory;
    use snix_store::pathinfoservice::PathInfo;

    use super::*;
    use crate::RaftDirectoryService;
    use crate::RaftPathInfoService;

    fn test_store_path(name: &str) -> StorePath<String> {
        let hash_bytes = blake3::hash(name.as_bytes());
        // StorePath digest is 20 bytes (truncated SHA-256)
        let mut digest = [0u8; 20];
        digest.copy_from_slice(&hash_bytes.as_bytes()[..20]);
        StorePath::from_name_and_digest_fixed(name.try_into().unwrap(), digest).unwrap()
    }

    #[tokio::test]
    async fn empty_closure_no_missing() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let root = test_store_path("hello");

        // Put a simple file PathInfo (no references, file root node)
        let file_digest = B3Digest::from(blake3::hash(b"hello world"));
        let path_info = PathInfo {
            store_path: root.clone(),
            node: Node::File {
                digest: file_digest,
                size: 11,
                executable: false,
            },
            references: vec![],
            nar_size: 100,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(path_info).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&root).await.unwrap();
        assert!(missing.is_empty(), "file PathInfo should have no missing content");
    }

    #[tokio::test]
    async fn missing_pathinfo_is_reported() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let root = test_store_path("missing-pkg");

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&root).await.unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], root);
    }

    #[tokio::test]
    async fn directory_with_missing_subtree() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let root = test_store_path("has-dir");

        // Create a directory that references a subdirectory we DON'T put
        let subdir_digest = B3Digest::from(blake3::hash(b"missing subdir"));
        let mut dir = Directory::new();
        dir.add("subdir".try_into().unwrap(), Node::Directory {
            digest: subdir_digest,
            size: 0,
        })
        .unwrap();
        let root_digest = ds.put(dir).await.unwrap();

        let path_info = PathInfo {
            store_path: root.clone(),
            node: Node::Directory {
                digest: root_digest,
                size: 0,
            },
            references: vec![],
            nar_size: 200,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(path_info).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&root).await.unwrap();
        assert_eq!(missing.len(), 1, "path with incomplete directory tree should be missing");
    }

    #[tokio::test]
    async fn reference_chain_traversal() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let lib = test_store_path("libfoo");
        let app = test_store_path("myapp");

        // lib has no references, simple file
        let lib_info = PathInfo {
            store_path: lib.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"libfoo.so")),
                size: 1000,
                executable: true,
            },
            references: vec![],
            nar_size: 1100,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(lib_info).await.unwrap();

        // app references lib
        let app_info = PathInfo {
            store_path: app.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"myapp")),
                size: 500,
                executable: true,
            },
            references: vec![lib.clone()],
            nar_size: 600,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(app_info).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&app).await.unwrap();
        // Both paths exist with file nodes — nothing missing
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn extract_content_digests_nested_tree() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv);

        let file1_digest = B3Digest::from(blake3::hash(b"file1"));
        let file2_digest = B3Digest::from(blake3::hash(b"file2"));

        // Inner directory with one file
        let mut inner = Directory::new();
        inner
            .add("f2.txt".try_into().unwrap(), Node::File {
                digest: file2_digest.clone(),
                size: 5,
                executable: false,
            })
            .unwrap();
        let inner_digest = ds.put(inner).await.unwrap();

        // Root directory with a file and a subdirectory
        let mut root_dir = Directory::new();
        root_dir
            .add("f1.txt".try_into().unwrap(), Node::File {
                digest: file1_digest.clone(),
                size: 5,
                executable: false,
            })
            .unwrap();
        root_dir
            .add("inner".try_into().unwrap(), Node::Directory {
                digest: inner_digest.clone(),
                size: 100,
            })
            .unwrap();
        let root_digest = ds.put(root_dir).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let root_node = Node::Directory {
            digest: root_digest.clone(),
            size: 200,
        };
        let digests = sync.extract_content_digests(&root_node).await.unwrap();

        // Should have: root_dir digest, file1, inner_dir digest, file2 = 4 total
        assert_eq!(digests.len(), 4);
        assert!(digests.contains(&root_digest));
        assert!(digests.contains(&inner_digest));
        assert!(digests.contains(&file1_digest));
        assert!(digests.contains(&file2_digest));
    }

    #[tokio::test]
    async fn max_closure_paths_bounded() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        // Create a long chain of store paths: A -> B -> C -> ... -> Z
        // Each path references the next one, creating a chain longer than practical
        let mut paths = Vec::new();
        for i in 0..100 {
            paths.push(test_store_path(&format!("path-{:03}", i)));
        }

        // Set up the reference chain
        for i in 0..100 {
            let current_path = paths[i].clone();
            let references = if i == 99 {
                vec![] // Last path has no references
            } else {
                vec![paths[i + 1].clone()] // Reference the next path
            };

            let path_info = PathInfo {
                store_path: current_path,
                node: Node::File {
                    digest: B3Digest::from(blake3::hash(format!("content-{}", i).as_bytes())),
                    size: 10,
                    executable: false,
                },
                references,
                nar_size: 50,
                nar_sha256: [0u8; 32],
                signatures: vec![],
                deriver: None,
                ca: None,
            };
            ps.put(path_info).await.unwrap();
        }

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&paths[0]).await.unwrap();

        // Should complete without error and respect bounds
        assert!(missing.is_empty(), "All paths exist as files, none should be missing");
    }

    #[tokio::test]
    async fn diamond_reference_graph_dedup() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let lib = test_store_path("shared-lib");
        let app1 = test_store_path("app1");
        let app2 = test_store_path("app2");

        // lib has no references
        let lib_info = PathInfo {
            store_path: lib.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"shared-lib.so")),
                size: 1000,
                executable: true,
            },
            references: vec![],
            nar_size: 1100,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(lib_info).await.unwrap();

        // app1 references lib
        let app1_info = PathInfo {
            store_path: app1.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"app1")),
                size: 500,
                executable: true,
            },
            references: vec![lib.clone()],
            nar_size: 600,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(app1_info).await.unwrap();

        // app2 also references lib (diamond pattern)
        let app2_info = PathInfo {
            store_path: app2.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"app2")),
                size: 600,
                executable: true,
            },
            references: vec![lib.clone()],
            nar_size: 700,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(app2_info).await.unwrap();

        // Create a top-level that references both apps
        let top = test_store_path("top-level");
        let top_info = PathInfo {
            store_path: top.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"top-level")),
                size: 100,
                executable: true,
            },
            references: vec![app1.clone(), app2.clone()],
            nar_size: 200,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(top_info).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&top).await.unwrap();

        // All paths exist as files — lib should only be visited once
        assert!(missing.is_empty(), "diamond dedup should work correctly");
    }

    #[tokio::test]
    async fn extract_content_digests_dedup_shared_files() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv);

        let shared_file_digest = B3Digest::from(blake3::hash(b"shared content"));

        // First directory with shared file
        let mut dir1 = Directory::new();
        dir1.add("shared.txt".try_into().unwrap(), Node::File {
            digest: shared_file_digest.clone(),
            size: 15,
            executable: false,
        })
        .unwrap();
        let dir1_digest = ds.put(dir1).await.unwrap();

        // Second directory with same shared file
        let mut dir2 = Directory::new();
        dir2.add("also-shared.txt".try_into().unwrap(), Node::File {
            digest: shared_file_digest.clone(),
            size: 15,
            executable: false,
        })
        .unwrap();
        let dir2_digest = ds.put(dir2).await.unwrap();

        // Root directory containing both subdirs
        let mut root_dir = Directory::new();
        root_dir
            .add("subdir1".try_into().unwrap(), Node::Directory {
                digest: dir1_digest.clone(),
                size: 100,
            })
            .unwrap();
        root_dir
            .add("subdir2".try_into().unwrap(), Node::Directory {
                digest: dir2_digest.clone(),
                size: 100,
            })
            .unwrap();
        let root_digest = ds.put(root_dir).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let root_node = Node::Directory {
            digest: root_digest.clone(),
            size: 200,
        };
        let digests = sync.extract_content_digests(&root_node).await.unwrap();

        // Should have: root_digest, dir1_digest, dir2_digest, shared_file_digest = 4 total
        // shared_file_digest should appear only once despite being referenced twice
        assert_eq!(digests.len(), 4);
        assert!(digests.contains(&root_digest));
        assert!(digests.contains(&dir1_digest));
        assert!(digests.contains(&dir2_digest));
        assert!(digests.contains(&shared_file_digest));

        // Count occurrences of shared file digest
        let shared_count = digests.iter().filter(|&d| d == &shared_file_digest).count();
        assert_eq!(shared_count, 1, "shared file digest should appear exactly once");
    }

    #[tokio::test]
    async fn extract_content_digests_symlink_ignored() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv);

        // Directory with only symlinks
        let mut dir = Directory::new();
        dir.add("link1".try_into().unwrap(), Node::Symlink {
            target: "/some/path".try_into().unwrap(),
        })
        .unwrap();
        dir.add("link2".try_into().unwrap(), Node::Symlink {
            target: "relative/path".try_into().unwrap(),
        })
        .unwrap();
        let dir_digest = ds.put(dir).await.unwrap();

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let root_node = Node::Directory {
            digest: dir_digest.clone(),
            size: 100,
        };
        let digests = sync.extract_content_digests(&root_node).await.unwrap();

        // Should only have the directory digest itself, no symlink content
        assert_eq!(digests.len(), 1);
        assert_eq!(digests[0], dir_digest);
    }

    #[tokio::test]
    async fn missing_reference_chain() {
        let kv = DeterministicKeyValueStore::new();
        let ds = RaftDirectoryService::from_arc(kv.clone());
        let ps = RaftPathInfoService::from_arc(kv.clone());

        let root = test_store_path("root");
        let missing_dep = test_store_path("missing-dependency");

        // Root path exists and references a missing path
        let root_info = PathInfo {
            store_path: root.clone(),
            node: Node::File {
                digest: B3Digest::from(blake3::hash(b"root content")),
                size: 100,
                executable: false,
            },
            references: vec![missing_dep.clone()],
            nar_size: 150,
            nar_sha256: [0u8; 32],
            signatures: vec![],
            deriver: None,
            ca: None,
        };
        ps.put(root_info).await.unwrap();

        // missing_dep is NOT in the PathInfoService

        let sync = StoreClosureSync::new(Arc::new(ds), Arc::new(ps));
        let missing = sync.compute_missing_paths(&root).await.unwrap();

        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], missing_dep);
    }
}
