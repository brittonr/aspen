//! irpc protocol definition for the castore service.
//!
//! Defines the RPC messages for blob and directory operations.
//! Uses postcard serialization over QUIC streams via irpc.

use irpc::channel::mpsc;
use irpc::channel::oneshot;
use irpc::rpc_requests;
use serde::Deserialize;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Blob messages
// ---------------------------------------------------------------------------

/// Check if a blob exists by its BLAKE3 digest.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlobHas {
    /// 32-byte BLAKE3 digest.
    pub digest: [u8; 32],
}

/// Read a blob's contents by digest.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlobRead {
    /// 32-byte BLAKE3 digest.
    pub digest: [u8; 32],
}

/// Response for BlobRead — None means not found.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlobReadResponse {
    /// The blob data, or None if not found.
    pub data: Option<Vec<u8>>,
}

/// Write a blob. Returns its BLAKE3 digest.
#[derive(Debug, Serialize, Deserialize)]
pub struct BlobWrite {
    /// Raw blob data.
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Directory messages
// ---------------------------------------------------------------------------

/// Get a directory by its BLAKE3 digest.
/// Returns protobuf-encoded `snix_castore::proto::Directory`.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirGet {
    /// 32-byte BLAKE3 digest.
    pub digest: [u8; 32],
}

/// Response for DirGet — None means not found.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirGetResponse {
    /// Protobuf-encoded Directory, or None if not found.
    pub data: Option<Vec<u8>>,
}

/// Store a directory (protobuf-encoded). Returns its BLAKE3 digest.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirPut {
    /// Protobuf-encoded `snix_castore::proto::Directory`.
    pub data: Vec<u8>,
}

/// Walk a directory tree root-to-leaves. Server streams back
/// protobuf-encoded Directory messages.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirGetRecursive {
    /// 32-byte BLAKE3 root digest.
    pub digest: [u8; 32],
}

/// One item in the DirGetRecursive stream.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirGetRecursiveItem {
    /// Protobuf-encoded Directory, or an error string.
    pub result: Result<Vec<u8>, String>,
}

/// Batch-put directories (leaves-to-root order). Returns root digest.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirPutMultiple;

/// One directory in a DirPutMultiple client stream.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirPutItem {
    /// Protobuf-encoded Directory.
    pub data: Vec<u8>,
}

/// Result of DirPutMultiple — the root digest or an error.
#[derive(Debug, Serialize, Deserialize)]
pub struct DirPutMultipleResponse {
    pub result: Result<[u8; 32], String>,
}

// ---------------------------------------------------------------------------
// Service definition via irpc derive macro
// ---------------------------------------------------------------------------

#[rpc_requests(message = CastoreMessage)]
#[derive(Serialize, Deserialize, Debug)]
pub enum CastoreProtocol {
    // Blob ops
    #[rpc(tx = oneshot::Sender<bool>)]
    BlobHas(BlobHas),
    #[rpc(tx = oneshot::Sender<BlobReadResponse>)]
    BlobRead(BlobRead),
    #[rpc(tx = oneshot::Sender<[u8; 32]>)]
    BlobWrite(BlobWrite),

    // Directory ops
    #[rpc(tx = oneshot::Sender<DirGetResponse>)]
    DirGet(DirGet),
    #[rpc(tx = oneshot::Sender<[u8; 32]>)]
    DirPut(DirPut),
    #[rpc(tx = mpsc::Sender<DirGetRecursiveItem>)]
    DirGetRecursive(DirGetRecursive),
    #[rpc(tx = oneshot::Sender<DirPutMultipleResponse>, rx = mpsc::Receiver<DirPutItem>)]
    DirPutMultiple(DirPutMultiple),
}
