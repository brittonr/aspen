//! DAG sync integration for snix store closures.
//!
//! Provides [`DirectoryLinkExtractor`] for walking the snix content-addressed
//! directory tree using the `aspen-dag` traversal framework.
//!
//! # Graph Shape
//!
//! ```text
//! PathInfo.entry (root Node)
//!   └─ Directory (B3Digest)
//!        ├─ File (B3Digest) ← leaf
//!        ├─ Symlink ← leaf (no digest)
//!        └─ Directory (B3Digest) ← recurse
//!             ├─ File ...
//!             └─ ...
//! ```
//!
//! The extractor takes a `B3Digest`, reads the `Directory` from the
//! `DirectoryService`, and returns child digests (subdirectory digests
//! and file content digests).

use std::sync::Arc;

use aspen_dag::LinkExtractor;
use aspen_dag::TraversalResult;
use aspen_dag::error::TraversalError;
use snix_castore::B3Digest;
use snix_castore::Node;
use snix_castore::directoryservice::DirectoryService;

/// Node type classification for snix DAG nodes.
///
/// Maps to `TraversalFilter` type tags in the wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SnixNodeType {
    /// A directory node (structural, has children).
    Directory = 0,
    /// A file node (leaf, content blob).
    File = 1,
    /// A symlink node (leaf, no content hash).
    Symlink = 2,
}

impl SnixNodeType {
    /// Type tag as u32 for the wire protocol.
    pub fn as_tag(self) -> u32 {
        self as u32
    }
}

/// Extracts child digests from snix `Directory` nodes.
///
/// Given a `B3Digest`, reads the `Directory` from the provided
/// `DirectoryService` and returns digests of all children:
///
/// - **Directory children**: their `B3Digest` (recursive)
/// - **File children**: their content `B3Digest` (leaf)
/// - **Symlink children**: ignored (no content hash)
pub struct DirectoryLinkExtractor<D> {
    dir_service: Arc<D>,
}

impl<D> DirectoryLinkExtractor<D> {
    /// Create a new directory link extractor.
    pub fn new(dir_service: Arc<D>) -> Self {
        Self { dir_service }
    }
}

impl<D: DirectoryService + Send + Sync + 'static> LinkExtractor for DirectoryLinkExtractor<D> {
    type Hash = B3Digest;

    fn extract_links<Db>(&self, digest: &B3Digest, _db: &Db) -> TraversalResult<Vec<B3Digest>> {
        let dir_service = Arc::clone(&self.dir_service);
        let digest = *digest;

        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { dir_service.get(&digest).await })
        });

        let dir = match result {
            Ok(Some(d)) => d,
            Ok(None) => return Ok(vec![]),
            Err(e) => {
                return Err(TraversalError::LinkExtraction {
                    message: format!("failed to read directory {}: {e}", digest),
                });
            }
        };

        let mut children = Vec::new();
        for (_name, node) in dir.nodes() {
            match node {
                Node::Directory { digest, .. } => children.push(*digest),
                Node::File { digest, .. } => children.push(*digest),
                Node::Symlink { .. } => {} // No content hash
            }
        }

        Ok(children)
    }
}

/// Extract child digests from a `Directory` without I/O (pure function).
///
/// Useful when you already have the `Directory` in hand and just need
/// its children's digests.
pub fn extract_directory_children(dir: &snix_castore::Directory) -> Vec<B3Digest> {
    let mut children = Vec::new();
    for (_name, node) in dir.nodes() {
        match node {
            Node::Directory { digest, .. } => children.push(*digest),
            Node::File { digest, .. } => children.push(*digest),
            Node::Symlink { .. } => {}
        }
    }
    children
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snix_node_type_tags() {
        assert_eq!(SnixNodeType::Directory.as_tag(), 0);
        assert_eq!(SnixNodeType::File.as_tag(), 1);
        assert_eq!(SnixNodeType::Symlink.as_tag(), 2);
    }

    #[test]
    fn extract_children_empty_directory() {
        let dir = snix_castore::Directory::new();
        let children = extract_directory_children(&dir);
        assert!(children.is_empty());
    }

    #[test]
    fn extract_children_with_files_and_dirs() {
        let file_digest = B3Digest::from(blake3::hash(b"file content"));
        let subdir_digest = B3Digest::from(blake3::hash(b"subdir"));

        let mut dir = snix_castore::Directory::new();
        dir.add("hello.txt".try_into().unwrap(), Node::File {
            digest: file_digest.clone(),
            size: 42,
            executable: false,
        })
        .unwrap();
        dir.add("subdir".try_into().unwrap(), Node::Directory {
            digest: subdir_digest.clone(),
            size: 100,
        })
        .unwrap();
        dir.add("link".try_into().unwrap(), Node::Symlink {
            target: "hello.txt".try_into().unwrap(),
        })
        .unwrap();

        let children = extract_directory_children(&dir);
        // 2 children: file + subdir. Symlink has no digest.
        assert_eq!(children.len(), 2);
        assert!(children.contains(&file_digest));
        assert!(children.contains(&subdir_digest));
    }

    #[test]
    fn extract_children_ordering() {
        // Directory.nodes() returns entries sorted by name.
        let d1 = B3Digest::from(blake3::hash(b"aaa"));
        let d2 = B3Digest::from(blake3::hash(b"bbb"));

        let mut dir = snix_castore::Directory::new();
        dir.add("zzz".try_into().unwrap(), Node::File {
            digest: d1.clone(),
            size: 1,
            executable: false,
        })
        .unwrap();
        dir.add("aaa".try_into().unwrap(), Node::File {
            digest: d2.clone(),
            size: 2,
            executable: false,
        })
        .unwrap();

        let children = extract_directory_children(&dir);
        assert_eq!(children.len(), 2);
        // Sorted by name: aaa first, zzz second.
        assert_eq!(children[0], d2);
        assert_eq!(children[1], d1);
    }
}
