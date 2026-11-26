/// Authenticates an exact unsigned request and the Raft term it is fenced to. The service must
/// durably consume the nonce before effects, and a write executes only on the leader of
/// `fence_term`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedInvocation {
    pub client_scope_ref: String,
    pub request_digest: String,
    pub nonce: [u8; 32],
    /// Raft term whose leader may execute a signed write. Reads are not fenced.
    pub fence_term: u64,
    pub signature: Vec<u8>,
}

/// Signature purpose for client invocations; v2 binds `fence_term` inside the signed payload.
const INVOCATION_PURPOSE: &str = "live-client-invocation-v2";

pub fn sign_request<T: serde::Serialize>(
    key: &iroh::SecretKey,
    grant: &super::VerifiedGrant,
    unsigned_request: &T,
    nonce: [u8; 32],
    fence_term: u64,
) -> crate::error::Result<SignedInvocation> {
    if key.public().to_string() != grant.grant().client_public_key || nonce == [0; 32] {
        return Err(super::denied("client request key or nonce not admitted"));
    }
    let client_scope_ref = grant.grant().client_scope_ref.clone();
    let request_digest = super::canonical::identity("unsigned-live-client-request", unsigned_request)?;
    let signature =
        super::canonical::sign(INVOCATION_PURPOSE, &(&client_scope_ref, &request_digest, nonce, fence_term), key)?;
    Ok(SignedInvocation {
        client_scope_ref,
        request_digest,
        nonce,
        fence_term,
        signature,
    })
}

pub fn verify_request<T: serde::Serialize>(
    grant: &super::VerifiedGrant,
    unsigned_request: &T,
    authorization: &SignedInvocation,
) -> crate::error::Result<()> {
    if authorization.client_scope_ref != grant.grant().client_scope_ref
        || authorization.nonce == [0; 32]
        || authorization.request_digest != super::canonical::identity("unsigned-live-client-request", unsigned_request)?
    {
        return Err(super::denied("client invocation grant, nonce, or exact request mismatch"));
    }
    super::canonical::verify(
        INVOCATION_PURPOSE,
        &(
            &authorization.client_scope_ref,
            &authorization.request_digest,
            authorization.nonce,
            authorization.fence_term,
        ),
        &grant.grant().client_public_key,
        &authorization.signature,
    )
}

/// Durably record an authenticated invocation's `(client scope, nonce)` replay marker in `root`
/// before any effect. The marker is a node-state file, independent of Raft log compaction; a second
/// consumption on the same node is denied.
// r[impl molten.consensus.reliability_replay_and_failstop]
pub fn consume_nonce(
    root: &crate::node_state::DirectoryRoot,
    authorization: &SignedInvocation,
) -> crate::error::Result<()> {
    let identity = crate::preserves_rail::record("live-client-replay-v1", vec![
        crate::preserves_rail::string(&authorization.client_scope_ref),
        preserves::IOValue::bytes(authorization.nonce),
    ]);
    let reference = crate::preserves_rail::canonical_hash(&identity)?;
    let digest = reference
        .strip_prefix("blake3:")
        .ok_or_else(|| super::denied("client replay marker reference is not a BLAKE3 reference"))?;
    let path = crate::node_state::RelativePath::parse(&format!("coordination-client-{digest}.preserves"))?;
    root.write_new(&path, &crate::preserves_rail::canonical_bytes(&identity)?)
}

/// The executing leader admits a signed write only in the Raft term the client fenced it to.
///
/// Every write of term `t` is admitted by the unique leader of `t`, which consumes the nonce with
/// [`consume_nonce`] before proposing. A replay in the same term meets that durable marker; a
/// replay after failover meets a later term. Both are denied before any effect, whatever the Raft
/// request window has compacted. A legitimate retry is re-signed for the current term with a fresh
/// nonce.
// r[impl molten.consensus.reliability_replay_and_failstop]
pub fn require_write_fence(authorization: &SignedInvocation, current_term: u64) -> crate::error::Result<()> {
    if authorization.fence_term != current_term {
        return Err(super::denied(format!(
            "signed write is fenced to Raft term {} but the leader term is {current_term}",
            authorization.fence_term
        )));
    }
    Ok(())
}
