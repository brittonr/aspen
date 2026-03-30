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
    /// Entries are sorted using git's mode-aware comparison: directory entries
    /// (mode 040000) are compared as if their name has `/` appended. This
    /// matches git's `base_name_compare` and ensures byte-identical round-trip
    /// through import → Forge storage → export.
    pub fn new(mut entries: Vec<TreeEntry>) -> Self {
        entries.sort_by(git_tree_entry_cmp);
        Self { entries }
    }
}

/// Compare two tree entries using git's mode-aware sort order.
///
/// Git sorts tree entries by treating directory names as if they end with `/`.
/// For example, `foo` (dir, mode 040000) sorts as `"foo/"` while `foo.c`
/// (file) sorts as `"foo.c"`. Since `'.'` (0x2E) < `'/'` (0x2F), the file
/// `foo.c` sorts before the directory `foo`.
///
/// For entries where neither is a directory, or where names don't share a
/// prefix, this produces the same result as plain lexicographic comparison.
fn git_tree_entry_cmp(a: &TreeEntry, b: &TreeEntry) -> std::cmp::Ordering {
    let a_bytes = a.name.as_bytes();
    let b_bytes = b.name.as_bytes();
    let min_len = a_bytes.len().min(b_bytes.len());

    // Compare the common prefix
    match a_bytes[..min_len].cmp(&b_bytes[..min_len]) {
        std::cmp::Ordering::Equal => {}
        ord => return ord,
    }

    // Common prefix matches — compare the "virtual" next byte.
    // Directories get '/' appended, others get '\0' (sorts before anything).
    let a_next = if a_bytes.len() > min_len {
        a_bytes[min_len]
    } else if a.is_directory() {
        b'/'
    } else {
        0
    };
    let b_next = if b_bytes.len() > min_len {
        b_bytes[min_len]
    } else if b.is_directory() {
        b'/'
    } else {
        0
    };

    a_next.cmp(&b_next)
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

    /// Extra headers preserved verbatim for round-trip fidelity.
    ///
    /// Git commits can have additional headers between the committer line
    /// and the blank line that separates headers from the message body.
    /// Common examples: `gpgsig` (PGP signatures from GitHub), `mergetag`,
    /// `encoding`. Multi-line headers use leading-space continuation lines.
    ///
    /// Each entry is `(header_name, full_value)` where `full_value` includes
    /// continuation lines (with their leading spaces preserved).
    pub extra_headers: Vec<(String, String)>,
}

impl CommitObject {
    /// Create a new commit.
    pub fn new(tree: blake3::Hash, parents: Vec<blake3::Hash>, author: Author, message: impl Into<String>) -> Self {
        Self {
            tree: *tree.as_bytes(),
            parents: parents.iter().map(|h| *h.as_bytes()).collect(),
            author: author.clone(),
            committer: author,
            extra_headers: Vec::new(),
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

    /// Test git's mode-aware sort: directory `foo` (sorts as "foo/") vs
    /// file `foo.c`. Since '.' (0x2E) < '/' (0x2F), the file comes first.
    #[test]
    fn test_tree_sorting_git_mode_aware() {
        let hash = blake3::hash(b"test");

        let entries = vec![
            TreeEntry::directory("foo", hash),
            TreeEntry::file("foo.c", hash),
            TreeEntry::file("foo-bar", hash),
        ];

        let tree = TreeObject::new(entries);

        // Git sort: "foo-bar" ('-'=0x2D), "foo.c" ('.'=0x2E), "foo" (dir, '/'=0x2F)
        assert_eq!(tree.entries[0].name, "foo-bar");
        assert_eq!(tree.entries[1].name, "foo.c");
        assert_eq!(tree.entries[2].name, "foo");
    }

    /// Two entries with the same name but different types: directory vs file.
    /// The directory sorts after the file because '/' > '\0'.
    #[test]
    fn test_tree_sorting_same_name_different_mode() {
        let hash = blake3::hash(b"test");

        // This is unusual but valid in the sort algorithm
        let entries = vec![TreeEntry::directory("lib", hash), TreeEntry::file("lib", hash)];

        let tree = TreeObject::new(entries);

        // File "lib" (virtual next byte = 0) < dir "lib" (virtual next byte = '/')
        assert_eq!(tree.entries[0].mode, 0o100644); // file first
        assert_eq!(tree.entries[1].mode, 0o040000); // dir second
    }

    #[test]
    fn test_commit_parents() {
        let tree = blake3::hash(b"tree");
        let parent1 = blake3::hash(b"parent1");
        let parent2 = blake3::hash(b"parent2");

        let author = Author {
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            public_key: None,
            timestamp_ms: 0,
            timezone: "+0000".to_string(),
            npub: None,
        };

        let commit = CommitObject::new(tree, vec![parent1, parent2], author, "Merge commit");

        assert_eq!(commit.tree(), tree);
        assert_eq!(commit.parents().len(), 2);
        assert_eq!(commit.parents()[0], parent1);
        assert_eq!(commit.parents()[1], parent2);
    }
}
