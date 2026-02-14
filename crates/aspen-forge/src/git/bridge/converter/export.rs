//! Forge -> Git export conversion.
//!
//! Handles translating Aspen Forge `SignedObject<GitObject>` (BLAKE3 based)
//! back into standard Git format (SHA-1 based).

use aspen_core::KeyValueStore;

use super::super::error::BridgeError;
use super::super::error::BridgeResult;
use super::super::sha1::Sha1Hash;
use super::GitObjectConverter;
use super::sha1_hash::compute_sha1;
use crate::git::object::BlobObject;
use crate::git::object::CommitObject;
use crate::git::object::GitObject;
use crate::git::object::TagObject;
use crate::git::object::TagTargetType;
use crate::git::object::TreeObject;
use crate::identity::RepoId;

impl<K: KeyValueStore + ?Sized> GitObjectConverter<K> {
    /// Export a Forge object to git format.
    ///
    /// Returns (git_content_bytes, SHA-1).
    /// The returned bytes are the content only, not including the header.
    pub async fn export_object(&self, repo_id: &RepoId, obj: &GitObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        match obj {
            GitObject::Blob(blob) => self.export_blob(blob),
            GitObject::Tree(tree) => self.export_tree(repo_id, tree).await,
            GitObject::Commit(commit) => self.export_commit(repo_id, commit).await,
            GitObject::Tag(tag) => self.export_tag(repo_id, tag).await,
        }
    }

    /// Export a blob to git format.
    fn export_blob(&self, blob: &BlobObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let sha1 = compute_sha1("blob", &blob.content);
        Ok((blob.content.clone(), sha1))
    }

    /// Export a tree to git format.
    async fn export_tree(&self, repo_id: &RepoId, tree: &TreeObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = Vec::new();

        for entry in &tree.entries {
            // Get SHA-1 for this entry's BLAKE3 hash
            let blake3 = blake3::Hash::from_bytes(entry.hash);
            let (sha1, _) =
                self.mapping.get_sha1(repo_id, &blake3).await?.ok_or_else(|| BridgeError::MappingNotFound {
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

        let sha1 = compute_sha1("tree", &content);
        Ok((content, sha1))
    }

    /// Export a commit to git format.
    async fn export_commit(&self, repo_id: &RepoId, commit: &CommitObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = String::new();

        // Tree line
        let tree_blake3 = blake3::Hash::from_bytes(commit.tree);
        let (tree_sha1, _) =
            self.mapping.get_sha1(repo_id, &tree_blake3).await?.ok_or_else(|| BridgeError::MappingNotFound {
                hash: hex::encode(commit.tree),
            })?;
        content.push_str(&format!("tree {}\n", tree_sha1.to_hex()));

        // Parent lines
        for parent_bytes in &commit.parents {
            let parent_blake3 = blake3::Hash::from_bytes(*parent_bytes);
            let (parent_sha1, _) =
                self.mapping.get_sha1(repo_id, &parent_blake3).await?.ok_or_else(|| BridgeError::MappingNotFound {
                    hash: hex::encode(parent_bytes),
                })?;
            content.push_str(&format!("parent {}\n", parent_sha1.to_hex()));
        }

        // Author line
        content.push_str(&format!("author {}\n", self.format_git_author(&commit.author)));

        // Committer line
        content.push_str(&format!("committer {}\n", self.format_git_author(&commit.committer)));

        // Blank line + message + trailing newline
        content.push('\n');
        content.push_str(&commit.message);
        // Git commit messages always end with a newline
        if !commit.message.ends_with('\n') {
            content.push('\n');
        }

        let content_bytes = content.into_bytes();
        let sha1 = compute_sha1("commit", &content_bytes);
        Ok((content_bytes, sha1))
    }

    /// Export a tag to git format.
    async fn export_tag(&self, repo_id: &RepoId, tag: &TagObject) -> BridgeResult<(Vec<u8>, Sha1Hash)> {
        let mut content = String::new();

        // Object line
        let target_blake3 = blake3::Hash::from_bytes(tag.target);
        let (target_sha1, _) =
            self.mapping.get_sha1(repo_id, &target_blake3).await?.ok_or_else(|| BridgeError::MappingNotFound {
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
        let sha1 = compute_sha1("tag", &content_bytes);
        Ok((content_bytes, sha1))
    }
}
