//! Link extractors for Forge DAG types.
//!
//! These implement the `aspen_dag::LinkExtractor` trait so that the generic
//! DAG traversal engine can walk Forge-specific object graphs without
//! knowing about Git or COB internals.

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_dag::LinkExtractor;
use aspen_dag::TraversalResult;
use aspen_dag::error::TraversalError;

use crate::CobChange;
use crate::GitObject;
use crate::types::SignedObject;

// ============================================================================
// GitLinkExtractor
// ============================================================================

/// Extracts child hashes from Forge Git objects.
///
/// Given a BLAKE3 hash, reads the `SignedObject<GitObject>` from the blob
/// store and returns child references:
///
/// - **Commit**: tree hash + parent commit hashes
/// - **Tree**: all entry hashes (blobs, subtrees, submodules)
/// - **Tag**: target object hash
/// - **Blob**: empty (blobs are leaves)
pub struct GitLinkExtractor<B> {
    blobs: Arc<B>,
}

impl<B> GitLinkExtractor<B> {
    /// Create a new Git link extractor backed by the given blob store.
    pub fn new(blobs: Arc<B>) -> Self {
        Self { blobs }
    }
}

impl<B: BlobRead + Send + Sync + 'static> LinkExtractor for GitLinkExtractor<B> {
    type Hash = blake3::Hash;

    fn extract_links<D>(&self, hash: &blake3::Hash, _db: &D) -> TraversalResult<Vec<blake3::Hash>> {
        // We need to block on the async blob read. The LinkExtractor trait is
        // sync because it's called inside the traversal loop. Use a
        // synchronous approach: try_read from a cached/in-memory store,
        // or fall back to empty links if the data isn't available yet.
        //
        // In the sender case, all data is local. In the receiver case,
        // data is inserted via db_mut() before the next call to next(),
        // so the link extractor is only called after data is available.
        //
        // For now, we provide a from_bytes path that doesn't need async.
        // The caller is responsible for ensuring the data is in the store.

        // Use the tokio runtime handle to block on the async read.
        // This is safe because LinkExtractor is called from within an
        // async context (inside DagTraversal::next() which is async).
        let blobs = Arc::clone(&self.blobs);
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());

        let data = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { blobs.get_bytes(&iroh_hash).await })
        });

        let bytes = match data {
            Ok(Some(b)) => b,
            Ok(None) => {
                // Data not yet available — this is expected on the receiver
                // side when the traversal encounters a node whose children
                // haven't been received yet. Return empty to stop descending.
                return Ok(vec![]);
            }
            Err(e) => {
                return Err(TraversalError::LinkExtraction {
                    message: format!("failed to read git object {}: {}", hash.to_hex(), e),
                });
            }
        };

        let signed: SignedObject<GitObject> =
            SignedObject::from_bytes(&bytes).map_err(|e| TraversalError::LinkExtraction {
                message: format!("failed to deserialize git object {}: {}", hash.to_hex(), e),
            })?;

        Ok(extract_git_references(&signed.payload))
    }
}

/// Extract child hashes from a Git object (pure function).
///
/// This is the same logic as `SyncService::extract_git_references` but
/// factored out so it can be used by both the old sync code and the new
/// DAG traversal.
fn extract_git_references(object: &GitObject) -> Vec<blake3::Hash> {
    match object {
        GitObject::Commit(c) => {
            let mut refs = Vec::with_capacity(1 + c.parents.len());
            refs.push(c.tree());
            refs.extend(c.parents());
            refs
        }
        GitObject::Tree(t) => t.entries.iter().map(|e| e.hash()).collect(),
        GitObject::Tag(t) => vec![t.target()],
        GitObject::Blob(_) => vec![],
    }
}

// ============================================================================
// CobLinkExtractor
// ============================================================================

/// Extracts parent hashes from Forge COB changes.
///
/// Given a BLAKE3 hash, reads the `SignedObject<CobChange>` from the blob
/// store and returns parent change hashes. Root changes have no parents.
pub struct CobLinkExtractor<B> {
    blobs: Arc<B>,
}

impl<B> CobLinkExtractor<B> {
    /// Create a new COB link extractor backed by the given blob store.
    pub fn new(blobs: Arc<B>) -> Self {
        Self { blobs }
    }
}

impl<B: BlobRead + Send + Sync + 'static> LinkExtractor for CobLinkExtractor<B> {
    type Hash = blake3::Hash;

    fn extract_links<D>(&self, hash: &blake3::Hash, _db: &D) -> TraversalResult<Vec<blake3::Hash>> {
        let blobs = Arc::clone(&self.blobs);
        let iroh_hash = iroh_blobs::Hash::from_bytes(*hash.as_bytes());

        let data = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { blobs.get_bytes(&iroh_hash).await })
        });

        let bytes = match data {
            Ok(Some(b)) => b,
            Ok(None) => return Ok(vec![]),
            Err(e) => {
                return Err(TraversalError::LinkExtraction {
                    message: format!("failed to read COB change {}: {}", hash.to_hex(), e),
                });
            }
        };

        let signed: SignedObject<CobChange> =
            SignedObject::from_bytes(&bytes).map_err(|e| TraversalError::LinkExtraction {
                message: format!("failed to deserialize COB change {}: {}", hash.to_hex(), e),
            })?;

        Ok(signed.payload.parents())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Use the git module's private types via the public re-exports.
    // We construct GitObject variants using the private constructors
    // accessible from within the crate.
    use crate::git::object::*;

    fn test_author() -> crate::identity::Author {
        crate::identity::Author {
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
            public_key: None,
            timestamp_ms: 0,
            timezone: "+0000".to_string(),
            npub: None,
        }
    }

    #[test]
    fn extract_git_refs_commit() {
        let tree = blake3::hash(b"tree");
        let parent1 = blake3::hash(b"parent1");
        let parent2 = blake3::hash(b"parent2");
        let commit = CommitObject::new(tree, vec![parent1, parent2], test_author(), "msg");
        let refs = extract_git_references(&GitObject::Commit(commit));
        assert_eq!(refs, vec![tree, parent1, parent2]);
    }

    #[test]
    fn extract_git_refs_tree() {
        let h1 = blake3::hash(b"file1");
        let h2 = blake3::hash(b"file2");
        let tree = TreeObject::new(vec![TreeEntry::file("a.txt", h1), TreeEntry::file("b.txt", h2)]);
        let refs = extract_git_references(&GitObject::Tree(tree));
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], h1);
        assert_eq!(refs[1], h2);
    }

    #[test]
    fn extract_git_refs_blob() {
        let blob = BlobObject::new(b"content");
        let refs = extract_git_references(&GitObject::Blob(blob));
        assert!(refs.is_empty());
    }

    #[test]
    fn extract_git_refs_tag() {
        let target = blake3::hash(b"commit");
        let tag = TagObject::new(target, "v1.0", test_author(), None);
        let refs = extract_git_references(&GitObject::Tag(tag));
        assert_eq!(refs, vec![target]);
    }
}
