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
