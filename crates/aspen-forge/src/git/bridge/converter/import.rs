//! Git -> Forge import conversion.
//!
//! Handles translating standard Git objects (SHA-1 based) into Aspen Forge
//! `SignedObject<GitObject>` (BLAKE3 based).

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
use crate::git::object::TreeEntry;
use crate::git::object::TreeObject;
use crate::identity::RepoId;
use crate::types::SignedObject;

impl<K: KeyValueStore + ?Sized> GitObjectConverter<K> {
    /// Import a git blob to Forge format.
    ///
    /// Blobs are simple: content is copied directly.
    /// Returns (GitObject, SHA-1, BLAKE3).
    pub fn import_blob(&self, content: &[u8]) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        // Compute SHA-1 of git format
        let sha1 = compute_sha1("blob", content);

        // Create Forge object
        let blob = GitObject::Blob(BlobObject::new(content.to_vec()));
        let signed = SignedObject::new(blob, &self.secret_key, &self.hlc)?;
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
        let sha1 = compute_sha1("tree", git_tree_content);

        // Parse git tree format and translate hashes
        let entries = self.parse_git_tree_content(repo_id, git_tree_content).await?;

        // Create Forge object
        let tree = GitObject::Tree(TreeObject::new(entries));
        let signed = SignedObject::new(tree, &self.secret_key, &self.hlc)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Parse git tree content and translate entry hashes.
    ///
    /// Git tree format: sequence of `<mode> <name>\0<20-byte-sha1>`
    ///
    /// Note: Gitlinks (mode 160000, submodules) are skipped because they reference
    /// commits in external repositories that cannot be translated to BLAKE3.
    async fn parse_git_tree_content(&self, repo_id: &RepoId, content: &[u8]) -> BridgeResult<Vec<TreeEntry>> {
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < content.len() {
            // Parse mode (octal number followed by space)
            let space_pos =
                content[pos..].iter().position(|&b| b == b' ').ok_or_else(|| BridgeError::MalformedTreeEntry {
                    message: "missing space after mode".to_string(),
                })?;

            let mode_str = std::str::from_utf8(&content[pos..pos + space_pos])?;
            let mode = u32::from_str_radix(mode_str, 8).map_err(|e| BridgeError::InvalidTreeMode {
                mode: format!("{mode_str}: {e}"),
            })?;
            pos += space_pos + 1;

            // Parse name (NUL-terminated)
            let nul_pos =
                content[pos..].iter().position(|&b| b == 0).ok_or_else(|| BridgeError::MalformedTreeEntry {
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

            // Skip gitlinks (mode 160000) - they reference commits in external repositories
            // The SHA-1 cannot be translated to BLAKE3 because the object doesn't exist locally
            if mode == 0o160000 {
                tracing::debug!(
                    name = %name,
                    sha1 = %sha1.to_hex(),
                    "skipping gitlink entry (submodule)"
                );
                continue;
            }

            // Translate SHA-1 to BLAKE3
            let (blake3, _obj_type) = self
                .mapping
                .get_blake3(repo_id, &sha1)
                .await?
                .ok_or_else(|| BridgeError::MappingNotFound { hash: sha1.to_hex() })?;

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
        let sha1 = compute_sha1("commit", git_commit_content);

        // Parse git commit format
        let content_str = std::str::from_utf8(git_commit_content)?;
        let commit_obj = self.parse_git_commit(repo_id, content_str).await?;

        // Create Forge object
        let commit = GitObject::Commit(commit_obj);
        let signed = SignedObject::new(commit, &self.secret_key, &self.hlc)?;
        let blake3 = signed.hash();

        Ok((signed, sha1, blake3))
    }

    /// Parse git commit content and translate references.
    async fn parse_git_commit(&self, repo_id: &RepoId, content: &str) -> BridgeResult<CommitObject> {
        let mut lines = content.lines().peekable();

        // Parse tree line
        let tree_line = lines.next().ok_or_else(|| BridgeError::MalformedCommit {
            message: "missing tree line".to_string(),
        })?;

        let tree_sha1_hex = tree_line.strip_prefix("tree ").ok_or_else(|| BridgeError::MalformedCommit {
            message: "invalid tree line".to_string(),
        })?;

        let tree_sha1 = Sha1Hash::from_hex(tree_sha1_hex)?;
        let (tree_blake3, _) =
            self.mapping.get_blake3(repo_id, &tree_sha1).await?.ok_or_else(|| BridgeError::MappingNotFound {
                hash: tree_sha1.to_hex(),
            })?;

        // Parse parent lines
        // Note: Skip parents that don't have mappings (external/not-yet-imported)
        let mut parents_blake3 = Vec::new();
        while let Some(line) = lines.peek() {
            if let Some(parent_hex) = line.strip_prefix("parent ") {
                let parent_sha1 = Sha1Hash::from_hex(parent_hex)?;
                match self.mapping.get_blake3(repo_id, &parent_sha1).await? {
                    Some((parent_blake3, _)) => {
                        parents_blake3.push(parent_blake3);
                    }
                    None => {
                        // Parent not yet imported or external - skip it
                        // This can happen during incremental imports or if the topological
                        // sort encountered an object that references a parent not in the batch.
                        tracing::debug!(
                            parent_sha1 = %parent_sha1.to_hex(),
                            "skipping parent commit without mapping"
                        );
                    }
                }
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

    /// Import a git tag to Forge format.
    pub async fn import_tag(
        &self,
        repo_id: &RepoId,
        git_tag_content: &[u8],
    ) -> BridgeResult<(SignedObject<GitObject>, Sha1Hash, blake3::Hash)> {
        let sha1 = compute_sha1("tag", git_tag_content);

        let content_str = std::str::from_utf8(git_tag_content)?;
        let tag_obj = self.parse_git_tag(repo_id, content_str).await?;

        let tag = GitObject::Tag(tag_obj);
        let signed = SignedObject::new(tag, &self.secret_key, &self.hlc)?;
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
        let target_sha1_hex = object_line.strip_prefix("object ").ok_or_else(|| BridgeError::MalformedObject {
            message: "invalid object line in tag".to_string(),
        })?;

        let target_sha1 = Sha1Hash::from_hex(target_sha1_hex)?;
        let (target_blake3, _obj_type) =
            self.mapping.get_blake3(repo_id, &target_sha1).await?.ok_or_else(|| BridgeError::MappingNotFound {
                hash: target_sha1.to_hex(),
            })?;

        // Parse type line
        let type_line = lines.next().ok_or_else(|| BridgeError::MalformedObject {
            message: "missing type line in tag".to_string(),
        })?;
        let type_str = type_line.strip_prefix("type ").ok_or_else(|| BridgeError::MalformedObject {
            message: "invalid type line in tag".to_string(),
        })?;

        let target_type = match type_str {
            "commit" => TagTargetType::Commit,
            "tree" => TagTargetType::Tree,
            "blob" => TagTargetType::Blob,
            "tag" => TagTargetType::Tag,
            _ => {
                return Err(BridgeError::UnknownObjectType {
                    type_str: type_str.to_string(),
                });
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
}
