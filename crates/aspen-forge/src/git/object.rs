//! Git object types.

use serde::Deserialize;
use serde::Serialize;

use crate::identity::Author;

/// A Git object.
///
/// This enum represents the four types of objects in Git's object model,
/// adapted to use BLAKE3 hashes instead of SHA-1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitObject {
    /// A blob containing raw file content.
    Blob(BlobObject),

    /// A tree representing a directory.
    Tree(TreeObject),

    /// A commit pointing to a tree with metadata.
    Commit(CommitObject),

    /// A tag referencing another object.
    Tag(TagObject),
}

impl GitObject {
    /// Returns the object type as a string.
    pub fn object_type(&self) -> &'static str {
        match self {
            GitObject::Blob(_) => "blob",
            GitObject::Tree(_) => "tree",
            GitObject::Commit(_) => "commit",
            GitObject::Tag(_) => "tag",
        }
    }

    /// Returns the size of the object's content in bytes.
    pub fn size(&self) -> usize {
        match self {
            GitObject::Blob(b) => b.content.len(),
            GitObject::Tree(t) => {
                // Approximate: entries contribute variable size
                t.entries.iter().map(|e| e.name.len() + 32 + 4).sum()
            }
            GitObject::Commit(c) => c.message.len() + (c.parents.len() * 32) + 64,
            GitObject::Tag(t) => t.message.as_ref().map(|m| m.len()).unwrap_or(0) + 32 + 64,
        }
    }
}

/// A blob object containing raw file content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobObject {
    /// The raw file content.
    pub content: Vec<u8>,
}

impl BlobObject {
    /// Create a new blob from content.
    pub fn new(content: impl Into<Vec<u8>>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// A tree object representing a directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeObject {
    /// The entries in this tree, sorted by name.
    pub entries: Vec<TreeEntry>,
}

impl TreeObject {
    /// Create a new tree with the given entries.
    ///
    /// Entries will be sorted by name.
    pub fn new(mut entries: Vec<TreeEntry>) -> Self {
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Self { entries }
    }
}

/// An entry in a tree object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    /// File mode (e.g., 0o100644 for regular file, 0o040000 for directory).
    pub mode: u32,

    /// Entry name (file or directory name).
    pub name: String,

    /// BLAKE3 hash of the referenced object.
    pub hash: [u8; 32],
}

impl TreeEntry {
    /// Create a new tree entry for a regular file.
    pub fn file(name: impl Into<String>, hash: blake3::Hash) -> Self {
        Self {
            mode: 0o100644,
            name: name.into(),
            hash: *hash.as_bytes(),
        }
    }

    /// Create a new tree entry for an executable file.
    pub fn executable(name: impl Into<String>, hash: blake3::Hash) -> Self {
        Self {
            mode: 0o100755,
            name: name.into(),
            hash: *hash.as_bytes(),
        }
    }

    /// Create a new tree entry for a subdirectory.
    pub fn directory(name: impl Into<String>, hash: blake3::Hash) -> Self {
        Self {
            mode: 0o040000,
            name: name.into(),
            hash: *hash.as_bytes(),
        }
    }

    /// Create a new tree entry for a symbolic link.
    pub fn symlink(name: impl Into<String>, hash: blake3::Hash) -> Self {
        Self {
            mode: 0o120000,
            name: name.into(),
            hash: *hash.as_bytes(),
        }
    }

    /// Check if this entry is a directory.
    pub fn is_directory(&self) -> bool {
        self.mode == 0o040000
    }

    /// Check if this entry is a regular file.
    pub fn is_file(&self) -> bool {
        self.mode == 0o100644 || self.mode == 0o100755
    }

    /// Check if this entry is executable.
    pub fn is_executable(&self) -> bool {
        self.mode == 0o100755
    }

    /// Get the hash as a blake3::Hash.
    pub fn hash(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.hash)
    }
}

/// A commit object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitObject {
    /// BLAKE3 hash of the tree this commit points to.
    pub tree: [u8; 32],

    /// BLAKE3 hashes of parent commits.
    pub parents: Vec<[u8; 32]>,

    /// The commit author.
    pub author: Author,

    /// The committer (may differ from author for cherry-picks, rebases).
    pub committer: Author,

    /// The commit message.
    pub message: String,
}

impl CommitObject {
    /// Create a new commit.
    pub fn new(tree: blake3::Hash, parents: Vec<blake3::Hash>, author: Author, message: impl Into<String>) -> Self {
        Self {
            tree: *tree.as_bytes(),
            parents: parents.iter().map(|h| *h.as_bytes()).collect(),
            author: author.clone(),
            committer: author,
            message: message.into(),
        }
    }

    /// Get the tree hash.
    pub fn tree(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.tree)
    }

    /// Get the parent hashes.
    pub fn parents(&self) -> Vec<blake3::Hash> {
        self.parents.iter().map(|h| blake3::Hash::from_bytes(*h)).collect()
    }
}

/// A tag object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagObject {
    /// BLAKE3 hash of the tagged object.
    pub target: [u8; 32],

    /// Type of the tagged object.
    pub target_type: TagTargetType,

    /// Tag name.
    pub name: String,

    /// The tagger.
    pub tagger: Author,

    /// Optional tag message.
    pub message: Option<String>,
}

/// The type of object a tag points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagTargetType {
    Commit,
    Tree,
    Blob,
    Tag,
}

impl TagObject {
    /// Create a new tag pointing to a commit.
    pub fn new(target: blake3::Hash, name: impl Into<String>, tagger: Author, message: Option<String>) -> Self {
        Self {
            target: *target.as_bytes(),
            target_type: TagTargetType::Commit,
            name: name.into(),
            tagger,
            message,
        }
    }

    /// Get the target hash.
    pub fn target(&self) -> blake3::Hash {
        blake3::Hash::from_bytes(self.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_entry_modes() {
        let hash = blake3::hash(b"test");

        let file = TreeEntry::file("test.txt", hash);
        assert!(file.is_file());
        assert!(!file.is_directory());
        assert!(!file.is_executable());

        let exe = TreeEntry::executable("script.sh", hash);
        assert!(exe.is_file());
        assert!(exe.is_executable());

        let dir = TreeEntry::directory("src", hash);
        assert!(dir.is_directory());
        assert!(!dir.is_file());
    }

    #[test]
    fn test_tree_sorting() {
        let hash = blake3::hash(b"test");

        let entries = vec![
            TreeEntry::file("z.txt", hash),
            TreeEntry::file("a.txt", hash),
            TreeEntry::directory("m", hash),
        ];

        let tree = TreeObject::new(entries);

        assert_eq!(tree.entries[0].name, "a.txt");
        assert_eq!(tree.entries[1].name, "m");
        assert_eq!(tree.entries[2].name, "z.txt");
    }

    #[test]
    fn test_commit_parents() {
        use iroh::PublicKey;

        let tree = blake3::hash(b"tree");
        let parent1 = blake3::hash(b"parent1");
        let parent2 = blake3::hash(b"parent2");

        let author = Author {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            public_key: None,
            timestamp_ms: 0,
            timezone: "+0000".to_string(),
        };

        let commit = CommitObject::new(tree, vec![parent1, parent2], author, "Merge commit");

        assert_eq!(commit.tree(), tree);
        assert_eq!(commit.parents().len(), 2);
        assert_eq!(commit.parents()[0], parent1);
        assert_eq!(commit.parents()[1], parent2);
    }
}
