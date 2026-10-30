/// Authenticates an exact unsigned request. The service must durably consume the nonce before
/// effects.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedInvocation {
    pub client_scope_ref: String,
    pub request_digest: String,
    pub nonce: [u8; 32],
    pub signature: Vec<u8>,
}

pub fn sign_request<T: serde::Serialize>(
    key: &iroh::SecretKey,
    grant: &super::VerifiedGrant,
    unsigned_request: &T,
    nonce: [u8; 32],
) -> crate::error::Result<SignedInvocation> {
    if key.public().to_string() != grant.grant().client_public_key || nonce == [0; 32] {
        return Err(super::denied("client request key or nonce not admitted"));
    }
    let client_scope_ref = grant.grant().client_scope_ref.clone();
    let request_digest = super::canonical::identity("unsigned-live-client-request", unsigned_request)?;
    let signature =
        super::canonical::sign("live-client-invocation", &(&client_scope_ref, &request_digest, nonce), key)?;
    Ok(SignedInvocation {
        client_scope_ref,
        request_digest,
        nonce,
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
        "live-client-invocation",
        &(&authorization.client_scope_ref, &authorization.request_digest, authorization.nonce),
        &grant.grant().client_public_key,
        &authorization.signature,
    )
}
