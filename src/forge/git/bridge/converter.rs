//! Object format conversion between standard Git and Aspen Forge.
//!
//! This module handles translation between:
//! - Standard Git format: `<type> <size>\0<content>` with SHA-1 hashing
//! - Aspen Forge format: `SignedObject<GitObject>` with BLAKE3 hashing
//!
//! The key challenge is that objects contain embedded hashes referencing
//! other objects. These must be rewritten during conversion.

use std::sync::Arc;

use sha1::{Digest, Sha1};

use crate::api::KeyValueStore;
use crate::forge::git::object::{
    BlobObject, CommitObject, GitObject, TagObject, TagTargetType, TreeEntry, TreeObject,
};
use crate::forge::identity::{Author, RepoId};
use crate::forge::types::SignedObject;

use super::error::{BridgeError, BridgeResult};
use super::mapping::{GitObjectType, HashMappingStore};
use super::sha1::Sha1Hash;

/// Converts between standard Git object format and Forge objects.
///
/// The converter handles:
/// - Blob conversion (direct content copy, different headers)
/// - Tree conversion (hash rewriting for each entry)
/// - Commit conversion (hash rewriting for tree and parents)
/// - Tag conversion (hash rewriting for target)
pub struct GitObjectConverter<K: KeyValueStore + ?Sized> {
    /// Hash mapping store for lookups during conversion.
    mapping: Arc<HashMappingStore<K>>,
    /// Secret key for signing Forge objects.
    secret_key: iroh::SecretKey,
}

impl<K: KeyValueStore + ?Sized> GitObjectConverter<K> {
    /// Create a new object converter.
    pub fn new(mapping: Arc<HashMappingStore<K>>, secret_key: iroh::SecretKey) -> Self {
        Self { mapping, secret_key }
    }

    // ========================================================================
    // SHA-1 Computation (Git format)
    // ========================================================================

    /// Compute SHA-1 hash of git object bytes.
    ///
    /// Git format: `<type> <size>\0<content>`
    fn compute_sha1(object_type: &str, content: &[u8]) -> Sha1Hash {
        let header = format!("{} {}\0", object_type, content.len());
        let mut hasher = Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(content);
        let result = hasher.finalize();
        Sha1Hash::from_bytes(result.into())
    }

    /// Compute SHA-1 of a full git object (including header).
    fn compute_sha1_full(git_bytes: &[u8]) -> Sha1Hash {
        let mut hasher = Sha1::new();
        hasher.update(git_bytes);
        let result = hasher.finalize();
        Sha1Hash::from_bytes(result.into())
    }

    // ========================================================================
    // Git -> Forge (Import)
    // ========================================================================

    /// Import a git blob to Forge format.
    ///
    /// Blobs are simple: content is copied directly.
    /// Returns (GitObject, SHA-1, BLAKE3).
    pub fn import_blob(
        &self,
        content: &[u8],
    ) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        // Compute SHA-1 of git format
        let sha1 = Self::compute_sha1("blob", content);

        // Create Forge object
        let blob = GitObject::Blob(BlobObject::new(content.to_vec()));
        let signed = SignedObject::new(blob, &self.secret_key)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Import a git tree to Forge format.
    ///
    /// Tree entries contain SHA-1 hashes that must be translated to BLAKE3.
    /// All referenced objects must already have mappings.
    pub async fn import_tree(
        &self,
        repo_id: &RepoId,
        git_tree_content: &[u8],
    ) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        // Compute SHA-1 of git format
        let sha1 = Self::compute_sha1("tree", git_tree_content);

        // Parse git tree format and translate hashes
        let entries = self
            .parse_git_tree_content(repo_id, git_tree_content)
            .await?;

        // Create Forge object
        let tree = GitObject::Tree(TreeObject::new(entries));
        let signed = SignedObject::new(tree, &self.secret_key)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Parse git tree content and translate entry hashes.
    ///
    /// Git tree format: sequence of `<mode> <name>\0<20-byte-sha1>`
    async fn parse_git_tree_content(
        &self,
        repo_id: &RepoId,
        content: &[u8],
    ) -> BridgeResult<Vec<TreeEntry>> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < content.len() {
            // Parse mode (octal number followed by space)
            let space_pos = content[pos..]
                .iter()
                .position(|&b| b == b' ')
                .ok_or_else(|| BridgeError::MalformedTreeEntry {
                    message: "missing space after mode".to_string(),
                })?;

            let mode_str = std::str::from_utf8(&content[pos..pos + space_pos])?;
            let mode = u32::from_str_radix(mode_str, 8).map_err(|e| {
                BridgeError::InvalidTreeMode {
                    mode: format!("{mode_str}: {e}"),
                }
            })?;
            pos += space_pos + 1;

            // Parse name (NUL-terminated)
            let nul_pos = content[pos..]
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| BridgeError::MalformedTreeEntry {
                    message: "missing NUL after name".to_string(),
                })?;

            let name = String::from_utf8(content[pos..pos + nul_pos].to_vec())?;
            pos += nul_pos + 1;

            // Parse SHA-1 hash (20 bytes binary)
            if pos + 20 > content.len() {
                return Err(BridgeError::MalformedTreeEntry {
                    message: "truncated SHA-1 hash".to_string(),
                });
            }
            let sha1 = Sha1Hash::from_slice(&content[pos..pos + 20])?;
            pos += 20;

            // Translate SHA-1 to BLAKE3
            let (blake3, _obj_type) = self
                .mapping
                .get_blake3(repo_id, &sha1)
                .await?
                .ok_or_else(|| BridgeError::MappingNotFound {
                    hash: sha1.to_hex(),
                })?;

            entries.push(TreeEntry {
                mode,
                name,
                hash: *blake3.as_bytes(),
            });
        }

        Ok(entries)
    }

    /// Import a git commit to Forge format.
    ///
    /// Commit references (tree, parents) must be translated from SHA-1 to BLAKE3.
    pub async fn import_commit(
        &self,
        repo_id: &RepoId,
        git_commit_content: &[u8],
    ) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        // Compute SHA-1 of git format
        let sha1 = Self::compute_sha1("commit", git_commit_content);

        // Parse git commit format
        let content_str = std::str::from_utf8(git_commit_content)?;
        let commit_obj = self.parse_git_commit(repo_id, content_str).await?;

        // Create Forge object
        let commit = GitObject::Commit(commit_obj);
        let signed = SignedObject::new(commit, &self.secret_key)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Parse git commit content and translate references.
    async fn parse_git_commit(
        &self,
        repo_id: &RepoId,
        content: &str,
    ) -> BridgeResult<CommitObject> {
        let mut lines = content.lines().peekable();

        // Parse tree line
        let tree_line = lines.next().ok_or_else(|| BridgeError::MalformedCommit {
            message: "missing tree line".to_string(),
        })?;

        let tree_sha1_hex = tree_line
            .strip_prefix("tree ")
            .ok_or_else(|| BridgeError::MalformedCommit {
                message: "invalid tree line".to_string(),
            })?;

        let tree_sha1 = Sha1Hash::from_hex(tree_sha1_hex)?;
        let (tree_blake3, _) = self
            .mapping
            .get_blake3(repo_id, &tree_sha1)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: tree_sha1.to_hex(),
            })?;

        // Parse parent lines
        let mut parents_blake3 = Vec::new();
        while let Some(line) = lines.peek() {
            if let Some(parent_hex) = line.strip_prefix("parent ") {
                let parent_sha1 = Sha1Hash::from_hex(parent_hex)?;
                let (parent_blake3, _) = self
                    .mapping
                    .get_blake3(repo_id, &parent_sha1)
                    .await?
                    .ok_or_else(|| BridgeError::MappingNotFound {
                        hash: parent_sha1.to_hex(),
                    })?;
                parents_blake3.push(parent_blake3);
                lines.next();
            } else {
                break;
            }
        }

        // Parse author line
        let author_line = lines.next().ok_or_else(|| BridgeError::MalformedCommit {
            message: "missing author line".to_string(),
        })?;
        let author = self.parse_git_author_line(author_line, "author")?;

        // Parse committer line
        let committer_line = lines.next().ok_or_else(|| BridgeError::MalformedCommit {
            message: "missing committer line".to_string(),
        })?;
        let committer = self.parse_git_author_line(committer_line, "committer")?;

        // Skip blank line
        lines.next();

        // Rest is the commit message
        let message: String = lines.collect::<Vec<_>>().join("\n");

        Ok(CommitObject {
            tree: *tree_blake3.as_bytes(),
            parents: parents_blake3.iter().map(|h| *h.as_bytes()).collect(),
            author,
            committer,
            message,
        })
    }

    /// Parse a git author/committer line.
    ///
    /// Format: `author Name <email> timestamp timezone`
    fn parse_git_author_line(&self, line: &str, prefix: &str) -> BridgeResult<Author> {
        let rest = line
            .strip_prefix(prefix)
            .and_then(|s| s.strip_prefix(' '))
            .ok_or_else(|| BridgeError::InvalidAuthorLine {
                message: format!("expected '{prefix} ' prefix"),
            })?;

        // Find email in angle brackets
        let email_start = rest.find('<').ok_or_else(|| BridgeError::InvalidAuthorLine {
            message: "missing '<' before email".to_string(),
        })?;
        let email_end = rest.find('>').ok_or_else(|| BridgeError::InvalidAuthorLine {
            message: "missing '>' after email".to_string(),
        })?;

        let name = rest[..email_start].trim().to_string();
        let email = rest[email_start + 1..email_end].to_string();

        // Parse timestamp and timezone after email
        let after_email = rest[email_end + 1..].trim();
        let (timestamp_ms, timezone) = if let Some(space_pos) = after_email.find(' ') {
            let timestamp_secs: i64 = after_email[..space_pos].parse().unwrap_or(0);
            let tz = after_email[space_pos + 1..].to_string();
            ((timestamp_secs * 1000) as u64, tz)
        } else {
            let timestamp_secs: i64 = after_email.parse().unwrap_or(0);
            ((timestamp_secs * 1000) as u64, "+0000".to_string())
        };

        Ok(Author::with_timezone(name, email, timestamp_ms, timezone))
    }

    /// Import a git tag to Forge format.
    pub async fn import_tag(
        &self,
        repo_id: &RepoId,
        git_tag_content: &[u8],
    ) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        let sha1 = Self::compute_sha1("tag", git_tag_content);

        let content_str = std::str::from_utf8(git_tag_content)?;
        let tag_obj = self.parse_git_tag(repo_id, content_str).await?;

        let tag = GitObject::Tag(tag_obj);
        let signed = SignedObject::new(tag, &self.secret_key)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Parse git tag content.
    async fn parse_git_tag(&self, repo_id: &RepoId, content: &str) -> BridgeResult<TagObject> {
        let mut lines = content.lines().peekable();

        // Parse object line
        let object_line = lines.next().ok_or_else(|| BridgeError::MalformedObject {
            message: "missing object line in tag".to_string(),
        })?;
        let target_sha1_hex = object_line
            .strip_prefix("object ")
            .ok_or_else(|| BridgeError::MalformedObject {
                message: "invalid object line in tag".to_string(),
            })?;

        let target_sha1 = Sha1Hash::from_hex(target_sha1_hex)?;
        let (target_blake3, _obj_type) = self
            .mapping
            .get_blake3(repo_id, &target_sha1)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: target_sha1.to_hex(),
            })?;

        // Parse type line
        let type_line = lines.next().ok_or_else(|| BridgeError::MalformedObject {
            message: "missing type line in tag".to_string(),
        })?;
        let type_str = type_line.strip_prefix("type ").ok_or_else(|| {
            BridgeError::MalformedObject {
                message: "invalid type line in tag".to_string(),
            }
        })?;

        let target_type = match type_str {
            "commit" => TagTargetType::Commit,
            "tree" => TagTargetType::Tree,
            "blob" => TagTargetType::Blob,
            "tag" => TagTargetType::Tag,
            _ => {
                return Err(BridgeError::UnknownObjectType {
                    type_str: type_str.to_string(),
                })
            }
        };

        // Parse tag name
        let tag_line = lines.next().ok_or_else(|| BridgeError::MalformedObject {
            message: "missing tag line".to_string(),
        })?;
        let name = tag_line
            .strip_prefix("tag ")
            .ok_or_else(|| BridgeError::MalformedObject {
                message: "invalid tag line".to_string(),
            })?
            .to_string();

        // Parse tagger
        let tagger_line = lines.next().ok_or_else(|| BridgeError::MalformedObject {
            message: "missing tagger line".to_string(),
        })?;
        let tagger = self.parse_git_author_line(tagger_line, "tagger")?;

        // Skip blank line
        lines.next();

        // Rest is the tag message
        let message_lines: Vec<_> = lines.collect();
        let message = if message_lines.is_empty() {
            None
        } else {
            Some(message_lines.join("\n"))
        };

        Ok(TagObject {
            target: *target_blake3.as_bytes(),
            target_type,
            name,
            tagger,
            message,
        })
    }

    // ========================================================================
    // Forge -> Git (Export)
    // ========================================================================

    /// Export a Forge object to git format.
    ///
    /// Returns (git_content_bytes, SHA-1).
    /// The returned bytes are the content only, not including the header.
    pub async fn export_object(
        &self,
        repo_id: &RepoId,
        obj: &GitObject,
    ) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        match obj {
            GitObject::Blob(blob) => self.export_blob(blob),
            GitObject::Tree(tree) => self.export_tree(repo_id, tree).await,
            GitObject::Commit(commit) => self.export_commit(repo_id, commit).await,
            GitObject::Tag(tag) => self.export_tag(repo_id, tag).await,
        }
    }

    /// Export a blob to git format.
    fn export_blob(&self, blob: &BlobObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let sha1 = Self::compute_sha1("blob", &blob.content);
        Ok((blob.content.clone(), sha1))
    }

    /// Export a tree to git format.
    async fn export_tree(
        &self,
        repo_id: &RepoId,
        tree: &TreeObject,
    ) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = Vec::new();

        for entry in &tree.entries {
            // Get SHA-1 for this entry's BLAKE3 hash
            let blake3 = blake3::Hash::from_bytes(entry.hash);
            let (sha1, _) = self
                .mapping
                .get_sha1(repo_id, &blake3)
                .await?
                .ok_or_else(|| BridgeError::MappingNotFound {
                    hash: hex::encode(entry.hash),
                })?;

            // Git tree entry: `<mode> <name>\0<20-byte-sha1>`
            let mode_str = format!("{:o}", entry.mode);
            content.extend_from_slice(mode_str.as_bytes());
            content.push(b' ');
            content.extend_from_slice(entry.name.as_bytes());
            content.push(0);
            content.extend_from_slice(sha1.as_bytes());
        }

        let sha1 = Self::compute_sha1("tree", &content);
        Ok((content, sha1))
    }

    /// Export a commit to git format.
    async fn export_commit(
        &self,
        repo_id: &RepoId,
        commit: &CommitObject,
    ) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = String::new();

        // Tree line
        let tree_blake3 = blake3::Hash::from_bytes(commit.tree);
        let (tree_sha1, _) = self
            .mapping
            .get_sha1(repo_id, &tree_blake3)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: hex::encode(commit.tree),
            })?;
        content.push_str(&format!("tree {}\n", tree_sha1.to_hex()));

        // Parent lines
        for parent_bytes in &commit.parents {
            let parent_blake3 = blake3::Hash::from_bytes(*parent_bytes);
            let (parent_sha1, _) = self
                .mapping
                .get_sha1(repo_id, &parent_blake3)
                .await?
                .ok_or_else(|| BridgeError::MappingNotFound {
                    hash: hex::encode(parent_bytes),
                })?;
            content.push_str(&format!("parent {}\n", parent_sha1.to_hex()));
        }

        // Author line
        content.push_str(&format!("author {}\n", self.format_git_author(&commit.author)));

        // Committer line
        content.push_str(&format!(
            "committer {}\n",
            self.format_git_author(&commit.committer)
        ));

        // Blank line + message + trailing newline
        content.push('\n');
        content.push_str(&commit.message);
        // Git commit messages always end with a newline
        if !commit.message.ends_with('\n') {
            content.push('\n');
        }

        let content_bytes = content.into_bytes();
        let sha1 = Self::compute_sha1("commit", &content_bytes);
        Ok((content_bytes, sha1))
    }

    /// Export a tag to git format.
    async fn export_tag(
        &self,
        repo_id: &RepoId,
        tag: &TagObject,
    ) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = String::new();

        // Object line
        let target_blake3 = blake3::Hash::from_bytes(tag.target);
        let (target_sha1, _) = self
            .mapping
            .get_sha1(repo_id, &target_blake3)
            .await?
            .ok_or_else(|| BridgeError::MappingNotFound {
                hash: hex::encode(tag.target),
            })?;
        content.push_str(&format!("object {}\n", target_sha1.to_hex()));

        // Type line
        let type_str = match tag.target_type {
            TagTargetType::Commit => "commit",
            TagTargetType::Tree => "tree",
            TagTargetType::Blob => "blob",
            TagTargetType::Tag => "tag",
        };
        content.push_str(&format!("type {}\n", type_str));

        // Tag name
        content.push_str(&format!("tag {}\n", tag.name));

        // Tagger
        content.push_str(&format!("tagger {}\n", self.format_git_author(&tag.tagger)));

        // Message (if present)
        if let Some(msg) = &tag.message {
            content.push('\n');
            content.push_str(msg);
            // Git tag messages always end with a newline
            if !msg.ends_with('\n') {
                content.push('\n');
            }
        }

        let content_bytes = content.into_bytes();
        let sha1 = Self::compute_sha1("tag", &content_bytes);
        Ok((content_bytes, sha1))
    }

    /// Format an Author as a git author/committer string.
    fn format_git_author(&self, author: &Author) -> String {
        let timestamp_secs = author.timestamp_ms / 1000;
        format!(
            "{} <{}> {} {}",
            author.name, author.email, timestamp_secs, author.timezone
        )
    }

    // ========================================================================
    // Utility Methods
    // ========================================================================

    /// Get the object type from git object bytes.
    pub fn parse_git_object_type(git_bytes: &[u8]) -> BridgeResult<(&str, &[u8])> {
        // Find space after type
        let space_pos = git_bytes
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| BridgeError::MalformedObject {
                message: "missing space in git object header".to_string(),
            })?;

        let type_str = std::str::from_utf8(&git_bytes[..space_pos])?;

        // Find NUL after size
        let nul_pos = git_bytes
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| BridgeError::MalformedObject {
                message: "missing NUL in git object header".to_string(),
            })?;

        let content = &git_bytes[nul_pos + 1..];

        Ok((type_str, content))
    }

    /// Parse a full git object (with header) and return type and content.
    pub fn split_git_object(git_bytes: &[u8]) -> BridgeResult<(GitObjectType, &[u8])> {
        let (type_str, content) = Self::parse_git_object_type(git_bytes)?;

        let obj_type = GitObjectType::from_str(type_str).ok_or_else(|| {
            BridgeError::UnknownObjectType {
                type_str: type_str.to_string(),
            }
        })?;

        Ok((obj_type, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha1_blob() {
        // Known SHA-1 for "hello\n"
        let content = b"hello\n";
        let sha1 = GitObjectConverter::<crate::api::DeterministicKeyValueStore>::compute_sha1(
            "blob", content,
        );
        // Git: echo -n "hello\n" | git hash-object --stdin
        // = ce013625030ba8dba906f756967f9e9ca394464a
        assert_eq!(sha1.to_hex(), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn test_compute_sha1_empty_blob() {
        let content = b"";
        let sha1 = GitObjectConverter::<crate::api::DeterministicKeyValueStore>::compute_sha1(
            "blob", content,
        );
        // Git: git hash-object -t blob /dev/null
        // = e69de29bb2d1d6434b8b29ae775ad8c2e48c5391
        assert_eq!(sha1.to_hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }

    #[test]
    fn test_parse_git_object_type() {
        let git_bytes = b"blob 5\0hello";
        let (type_str, content) =
            GitObjectConverter::<crate::api::DeterministicKeyValueStore>::parse_git_object_type(
                git_bytes,
            )
            .unwrap();
        assert_eq!(type_str, "blob");
        assert_eq!(content, b"hello");
    }
}
