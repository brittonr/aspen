//! RPC protocol types for Raft consensus over Iroh transport.
//!
//! This module defines the RPC protocol for Raft consensus communication
//! using IRPC over Iroh QUIC connections. The AppTypeConfig here is identical
//! to the one in aspen-raft/src/types.rs to avoid circular dependencies.
//!
//! **IMPORTANT**: The AppTypeConfig declaration below MUST be kept in sync
//! with the one in aspen-raft/src/types.rs. Any changes to the type parameters
//! must be applied to both locations.

use aspen_raft_types::AppRequest;
use aspen_raft_types::AppResponse;
use aspen_raft_types::NodeId;
use aspen_raft_types::RaftMemberInfo;
use irpc::channel::oneshot;
use irpc::rpc_requests;
use openraft::error::RaftError;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::SnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::type_config::alias::VoteOf;
use serde::Deserialize;
use serde::Serialize;

/// Legacy authentication context for backward compatibility.
///
/// # Security Warning
///
/// **DO NOT USE FOR PRODUCTION SECURITY.** This implementation only performs
/// a basic non-empty check on the HMAC field - it does NOT cryptographically
/// verify the response. Any non-empty response will be accepted as valid.
///
/// For proper HMAC-SHA256 authentication, use `aspen_raft::auth::AuthContext`
/// which provides:
/// - Cryptographic HMAC-SHA256 verification
/// - Constant-time comparison to prevent timing attacks
/// - Proper challenge expiration checking
///
/// This type is retained only for API compatibility during migration.
///
/// # Migration Path
///
/// 1. Move `aspen_raft::auth::AuthContext` to a shared crate (e.g., `aspen-auth`)
/// 2. Update all consumers to use the cryptographic implementation
/// 3. Remove this stub
#[deprecated(
    since = "0.2.0",
    note = "INSECURE: Use aspen_raft::auth::AuthContext for cryptographic verification"
)]
#[derive(Debug, Clone)]
pub struct AuthContext {
    _cluster_cookie: String,
}

/// Authentication challenge from server.
///
/// Used in challenge-response authentication. The server sends this challenge
/// and the client must respond with a valid HMAC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub nonce: Vec<u8>,
    pub timestamp: u64,
}

#[allow(deprecated)]
impl AuthContext {
    pub fn new(cluster_cookie: String) -> Self {
        tracing::warn!(
            target: "aspen_transport::security",
            "using INSECURE legacy AuthContext - migrate to aspen_raft::auth::AuthContext"
        );
        Self {
            _cluster_cookie: cluster_cookie,
        }
    }

    pub fn generate_challenge(&self) -> AuthChallenge {
        let mut nonce = vec![0u8; 32];
        use rand::RngCore;
        rand::rng().fill_bytes(&mut nonce);

        AuthChallenge {
            nonce,
            timestamp: aspen_core::utils::current_time_ms(),
        }
    }

    /// **INSECURE**: Only checks for non-empty fields.
    ///
    /// This does NOT perform cryptographic verification. Any non-empty
    /// response will be accepted. Use `aspen_raft::auth::AuthContext` for
    /// proper HMAC-SHA256 verification.
    pub fn verify_response(&self, _challenge: &AuthChallenge, response: &AuthResponse) -> bool {
        // WARNING: This is NOT secure - it only checks for non-empty fields
        // A proper implementation would use HMAC-SHA256 with constant-time comparison
        // See aspen_raft::auth::AuthContext for the secure implementation
        !response.hmac.is_empty() && !response.client_nonce.is_empty()
    }
}

/// Authentication response from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub hmac: Vec<u8>,
    pub client_nonce: Vec<u8>,
}

/// Authentication result from server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResult {
    Success,
    Failed,
}

// Declare AppTypeConfig locally to avoid circular dependencies.
// aspen-transport cannot depend on aspen-raft (would create a cycle).
// This is structurally identical to aspen-raft/src/types.rs::AppTypeConfig.
//
// SAFETY: The transmutes between this AppTypeConfig and aspen_raft::AppTypeConfig
// are safe because both declarations use identical type parameters from aspen-raft-types.
// This is verified at compile time by the types in the RPC messages.
openraft::declare_raft_types!(
    pub AppTypeConfig: D = AppRequest, R = AppResponse, NodeId = NodeId, Node = RaftMemberInfo,
);

/// Vote request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    pub request: VoteRequest<AppTypeConfig>,
}

/// AppendEntries request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesRequest {
    pub request: AppendEntriesRequest<AppTypeConfig>,
}

/// Full snapshot request wrapper for IRPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftSnapshotRequest {
    pub vote: VoteOf<AppTypeConfig>,
    pub snapshot_meta: openraft::SnapshotMeta<AppTypeConfig>,
    pub snapshot_data: Vec<u8>,
}

/// IRPC service protocol for Raft RPC operations.
#[rpc_requests(message = RaftRpcMessage)]
#[derive(Debug, Serialize, Deserialize)]
pub enum RaftRpcProtocol {
    #[rpc(tx = oneshot::Sender<Result<VoteResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    Vote(RaftVoteRequest),

    #[rpc(tx = oneshot::Sender<Result<AppendEntriesResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    AppendEntries(RaftAppendEntriesRequest),

    #[rpc(tx = oneshot::Sender<Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>>)]
    InstallSnapshot(RaftSnapshotRequest),
}

/// Wire protocol response types that are serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftRpcResponse {
    Vote(VoteResponse<AppTypeConfig>),
    AppendEntries(AppendEntriesResponse<AppTypeConfig>),
    InstallSnapshot(Result<SnapshotResponse<AppTypeConfig>, RaftError<AppTypeConfig>>),
    FatalError(RaftFatalErrorKind),
}

/// Classification of fatal Raft errors for wire protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftFatalErrorKind {
    Panicked,
    Stopped,
    StorageError,
}

impl RaftFatalErrorKind {
    pub fn from_fatal<C: openraft::RaftTypeConfig>(fatal: &openraft::error::Fatal<C>) -> Self {
        match fatal {
            openraft::error::Fatal::Panicked => Self::Panicked,
            openraft::error::Fatal::Stopped => Self::Stopped,
            _ => Self::StorageError,
        }
    }
}

/// Sharding constants and functions for prefixing RPC messages.
pub const SHARD_PREFIX_SIZE: usize = 4;

pub fn encode_shard_prefix(shard_id: u32) -> [u8; SHARD_PREFIX_SIZE] {
    shard_id.to_be_bytes()
}

pub fn try_decode_shard_prefix(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < SHARD_PREFIX_SIZE {
        return None;
    }
    let prefix: [u8; SHARD_PREFIX_SIZE] = bytes[..SHARD_PREFIX_SIZE].try_into().ok()?;
    Some(u32::from_be_bytes(prefix))
}
