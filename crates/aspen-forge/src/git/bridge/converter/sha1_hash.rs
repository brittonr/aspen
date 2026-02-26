//! SHA-1 computation helpers for Git object hashing.

use sha1::Digest;
use sha1::Sha1;

use super::super::sha1::Sha1Hash;

/// Compute SHA-1 hash of git object bytes.
///
/// Git format: `<type> <size>\0<content>`
pub(crate) fn compute_sha1(object_type: &str, content: &[u8]) -> Sha1Hash {
    let header = format!("{} {}\0", object_type, content.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(content);
    let result = hasher.finalize();
    Sha1Hash::from_bytes(result.into())
}

/// Compute SHA-1 of a full git object (including header).
#[allow(dead_code)]
pub(crate) fn compute_sha1_full(git_bytes: &[u8]) -> Sha1Hash {
    let mut hasher = Sha1::new();
    hasher.update(git_bytes);
    let result = hasher.finalize();
    Sha1Hash::from_bytes(result.into())
}
