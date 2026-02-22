//! WASM-compatible storage types for the forge plugin.
//!
//! These avoid iroh dependencies by storing keys and hashes as hex strings.
//! All types serialize/deserialize with serde_json for blob storage.

use serde::Deserialize;
use serde::Serialize;

/// Signed wrapper for forge objects.
///
/// Replaces the native `aspen-forge` `SignedObject<T>` with a self-contained
/// version that uses hex strings instead of iroh types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedObject<T> {
    pub payload: T,
    /// Ed25519 public key of the signer (hex-encoded, 64 chars).
    pub author: String,
    /// HLC timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Ed25519 signature (hex-encoded, 128 chars).
    pub signature: String,
}

/// Repository identity stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoIdentity {
    pub name: String,
    pub description: Option<String>,
    pub default_branch: String,
    /// Delegate public keys (hex-encoded).
    pub delegates: Vec<String>,
    pub threshold: u32,
    pub created_at_ms: u64,
}

/// Git object variants stored in the blob store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitObject {
    Blob(BlobObject),
    Tree(TreeObject),
    Commit(Box<CommitObject>),
}

/// Raw blob content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobObject {
    pub content: Vec<u8>,
}

/// Tree (directory listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeObject {
    pub entries: Vec<TreeEntry>,
}

/// Single entry in a tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// File mode (e.g., 0o100644 for regular file).
    pub mode: u32,
    /// Entry name.
    pub name: String,
    /// BLAKE3 hash of the referenced object (hex-encoded).
    pub hash: String,
}

/// Commit object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitObject {
    /// Tree hash this commit points to (hex-encoded).
    pub tree: String,
    /// Parent commit hashes (hex-encoded).
    pub parents: Vec<String>,
    /// Author information.
    pub author: Author,
    /// Committer information.
    pub committer: Author,
    /// Commit message.
    pub message: String,
}

/// Author/committer metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
    /// Public key (hex-encoded).
    pub public_key: Option<String>,
    pub timestamp_ms: u64,
    pub timezone: String,
}

// ============================================================================
// Issue & Patch types (KV-backed COBs)
// ============================================================================

/// Issue stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueData {
    pub title: String,
    pub body: String,
    /// "open" or "closed"
    pub state: String,
    pub close_reason: Option<String>,
    pub labels: Vec<String>,
    pub comments: Vec<CommentData>,
    /// Hex-encoded public keys.
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    /// Hex-encoded public key of the creator.
    pub author: String,
}

/// Patch stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchData {
    pub title: String,
    pub description: String,
    /// "open", "merged", or "closed"
    pub state: String,
    pub close_reason: Option<String>,
    /// Hex-encoded BLAKE3 hash.
    pub base: String,
    /// Hex-encoded BLAKE3 hash (current head).
    pub head: String,
    pub labels: Vec<String>,
    pub comments: Vec<CommentData>,
    pub revisions: Vec<RevisionData>,
    pub approvals: Vec<ApprovalData>,
    /// Hex-encoded public keys.
    pub assignees: Vec<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    /// Hex-encoded public key of the creator.
    pub author: String,
}

/// Comment on an issue or patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentData {
    /// Hex-encoded BLAKE3 hash identifying this comment.
    pub hash: String,
    /// Hex-encoded public key of the author.
    pub author: String,
    pub body: String,
    pub timestamp_ms: u64,
}

/// Patch revision (head update).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionData {
    /// Hex-encoded BLAKE3 hash identifying this revision.
    pub hash: String,
    /// Hex-encoded BLAKE3 hash of the new head commit.
    pub head: String,
    pub message: Option<String>,
    /// Hex-encoded public key of the author.
    pub author: String,
    pub timestamp_ms: u64,
}

/// Patch approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalData {
    /// Hex-encoded public key of the approver.
    pub author: String,
    /// Hex-encoded BLAKE3 hash of the approved commit.
    pub commit: String,
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signed_object_roundtrip() {
        let obj = SignedObject {
            payload: "test payload".to_string(),
            author: "a".repeat(64),
            timestamp_ms: 1234567890,
            signature: "b".repeat(128),
        };
        let json = serde_json::to_vec(&obj).expect("serialize");
        let decoded: SignedObject<String> = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(decoded.payload, "test payload");
        assert_eq!(decoded.timestamp_ms, 1234567890);
    }

    #[test]
    fn test_repo_identity_roundtrip() {
        let identity = RepoIdentity {
            name: "test-repo".to_string(),
            description: Some("A test repository".to_string()),
            default_branch: "main".to_string(),
            delegates: vec!["delegate1".to_string()],
            threshold: 1,
            created_at_ms: 1000,
        };
        let json = serde_json::to_vec(&identity).expect("serialize");
        let decoded: RepoIdentity = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(decoded.name, "test-repo");
        assert_eq!(decoded.default_branch, "main");
    }

    #[test]
    fn test_git_object_blob_roundtrip() {
        let blob = GitObject::Blob(BlobObject {
            content: b"hello world".to_vec(),
        });
        let json = serde_json::to_vec(&blob).expect("serialize");
        let decoded: GitObject = serde_json::from_slice(&json).expect("deserialize");
        match decoded {
            GitObject::Blob(b) => assert_eq!(b.content, b"hello world"),
            _ => panic!("expected Blob"),
        }
    }

    #[test]
    fn test_git_object_tree_roundtrip() {
        let tree = GitObject::Tree(TreeObject {
            entries: vec![TreeEntry {
                mode: 0o100644,
                name: "file.txt".to_string(),
                hash: "abc123".to_string(),
            }],
        });
        let json = serde_json::to_vec(&tree).expect("serialize");
        let decoded: GitObject = serde_json::from_slice(&json).expect("deserialize");
        match decoded {
            GitObject::Tree(t) => {
                assert_eq!(t.entries.len(), 1);
                assert_eq!(t.entries[0].name, "file.txt");
            }
            _ => panic!("expected Tree"),
        }
    }

    #[test]
    fn test_git_object_commit_roundtrip() {
        let author = Author {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            public_key: Some("key123".to_string()),
            timestamp_ms: 1000,
            timezone: "+0000".to_string(),
        };
        let commit = GitObject::Commit(Box::new(CommitObject {
            tree: "treehash".to_string(),
            parents: vec!["parent1".to_string()],
            author: author.clone(),
            committer: author,
            message: "Initial commit".to_string(),
        }));
        let json = serde_json::to_vec(&commit).expect("serialize");
        let decoded: GitObject = serde_json::from_slice(&json).expect("deserialize");
        match decoded {
            GitObject::Commit(c) => {
                assert_eq!(c.message, "Initial commit");
                assert_eq!(c.tree, "treehash");
                assert_eq!(c.parents, vec!["parent1".to_string()]);
            }
            _ => panic!("expected Commit"),
        }
    }

    #[test]
    fn test_author_without_public_key() {
        let author = Author {
            name: "Anonymous".to_string(),
            email: "anon@example.com".to_string(),
            public_key: None,
            timestamp_ms: 0,
            timezone: "+0000".to_string(),
        };
        let json = serde_json::to_vec(&author).expect("serialize");
        let decoded: Author = serde_json::from_slice(&json).expect("deserialize");
        assert!(decoded.public_key.is_none());
    }

    #[test]
    fn test_tree_entry_modes() {
        // Regular file
        let file_entry = TreeEntry {
            mode: 0o100644,
            name: "regular.txt".to_string(),
            hash: "hash1".to_string(),
        };
        // Executable
        let exec_entry = TreeEntry {
            mode: 0o100755,
            name: "script.sh".to_string(),
            hash: "hash2".to_string(),
        };
        // Directory (tree)
        let dir_entry = TreeEntry {
            mode: 0o040000,
            name: "subdir".to_string(),
            hash: "hash3".to_string(),
        };

        let tree = TreeObject {
            entries: vec![file_entry, exec_entry, dir_entry],
        };
        let json = serde_json::to_vec(&tree).expect("serialize");
        let decoded: TreeObject = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(decoded.entries[0].mode, 0o100644);
        assert_eq!(decoded.entries[1].mode, 0o100755);
        assert_eq!(decoded.entries[2].mode, 0o040000);
    }

    #[test]
    fn test_empty_blob() {
        let blob = GitObject::Blob(BlobObject { content: vec![] });
        let json = serde_json::to_vec(&blob).expect("serialize");
        let decoded: GitObject = serde_json::from_slice(&json).expect("deserialize");
        match decoded {
            GitObject::Blob(b) => assert!(b.content.is_empty()),
            _ => panic!("expected Blob"),
        }
    }

    #[test]
    fn test_repo_identity_no_description() {
        let identity = RepoIdentity {
            name: "minimal".to_string(),
            description: None,
            default_branch: "master".to_string(),
            delegates: vec![],
            threshold: 0,
            created_at_ms: 0,
        };
        let json = serde_json::to_vec(&identity).expect("serialize");
        let decoded: RepoIdentity = serde_json::from_slice(&json).expect("deserialize");
        assert!(decoded.description.is_none());
        assert!(decoded.delegates.is_empty());
    }
}
