//! Object format conversion between standard Git and Aspen Forge.
//!
//! This module handles translation between:
//! - Standard Git format: `<type> <size>\0<content>` with SHA-1 hashing
//! - Aspen Forge format: `SignedObject<GitObject>` with BLAKE3 hashing
//!
//! The key challenge is that objects contain embedded hashes referencing
//! other objects. These must be rewritten during conversion.

mod export;
mod import;
mod parse;
pub(crate) mod sha1_hash;

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::hlc::HLC;

use super::error::BridgeError;
use super::error::BridgeResult;
use super::mapping::HashMappingStore;
use crate::identity::Author;

/// Converts between standard Git object format and Forge objects.
///
/// The converter handles:
/// - Blob conversion (direct content copy, different headers)
/// - Tree conversion (hash rewriting for each entry)
/// - Commit conversion (hash rewriting for tree and parents)
/// - Tag conversion (hash rewriting for target)
pub struct GitObjectConverter<K: KeyValueStore + ?Sized> {
    /// Hash mapping store for lookups during conversion.
    pub(crate) mapping: Arc<HashMappingStore<K>>,
    /// Secret key for signing Forge objects.
    pub(crate) secret_key: iroh::SecretKey,
    /// Hybrid logical clock for timestamping signed objects.
    pub(crate) hlc: HLC,
}

impl<K: KeyValueStore + ?Sized> GitObjectConverter<K> {
    /// Create a new object converter.
    pub fn new(mapping: Arc<HashMappingStore<K>>, secret_key: iroh::SecretKey, hlc: HLC) -> Self {
        Self {
            mapping,
            secret_key,
            hlc,
        }
    }

    /// Parse a git author/committer line.
    ///
    /// Format: `author Name <email> timestamp timezone`
    pub(crate) fn parse_git_author_line(&self, line: &str, prefix: &str) -> BridgeResult<Author> {
        let rest = line.strip_prefix(prefix).and_then(|s| s.strip_prefix(' ')).ok_or_else(|| {
            BridgeError::InvalidAuthorLine {
                message: format!("expected '{prefix} ' prefix"),
            }
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

    /// Format an Author as a git author/committer string.
    pub(crate) fn format_git_author(&self, author: &Author) -> String {
        let timestamp_secs = author.timestamp_ms / 1000;
        format!("{} <{}> {} {}", author.name, author.email, timestamp_secs, author.timezone)
    }
}

#[cfg(test)]
mod tests {
    use super::sha1_hash::compute_sha1;
    use super::*;

    #[test]
    fn test_compute_sha1_blob() {
        // Known SHA-1 for "hello\n"
        let content = b"hello\n";
        let sha1 = compute_sha1("blob", content);
        // Git: echo -n "hello\n" | git hash-object --stdin
        // = ce013625030ba8dba906f756967f9e9ca394464a
        assert_eq!(sha1.to_hex(), "ce013625030ba8dba906f756967f9e9ca394464a");
    }

    #[test]
    fn test_compute_sha1_empty_blob() {
        let content = b"";
        let sha1 = compute_sha1("blob", content);
        // Git: git hash-object -t blob /dev/null
        // = e69de29bb2d1d6434b8b29ae775ad8c2e48c5391
        assert_eq!(sha1.to_hex(), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }

    #[test]
    fn test_parse_git_object_type() {
        let git_bytes = b"blob 5\0hello";
        let (type_str, content) =
            GitObjectConverter::<aspen_testing::DeterministicKeyValueStore>::parse_git_object_type(git_bytes).unwrap();
        assert_eq!(type_str, "blob");
        assert_eq!(content, b"hello");
    }
}
