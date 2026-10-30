//! Current operator-owned Basalt/UCAN admission; no bundle-provided trust anchors.
pub(crate) mod boundaries;
#[cfg(test)]
mod tests;
pub mod wire;

/// Reviewed Basalt UCAN verification bounds, stated explicitly rather than taken from library
/// defaults.
const MAX_PROOF_DEPTH: usize = 16;
const MAX_PROOF_COUNT: usize = 64;
const MAX_COMPACT_TOKEN_BYTES: usize = 65_536;
const MAX_AUDIENCE_BYTES: usize = 512;
const MAX_RESOURCE_BYTES: usize = 4_096;
const MAX_ABILITY_BYTES: usize = 256;

/// Installed operator trust observed through the shell-selected wall clock `C`.
pub struct Current<C> {
    root: crate::node_state::DirectoryRoot,
    bundle_root: String,
    clock: C,
}

pub struct SigningInput<'a> {
    pub bundle_root: &'a str,
    pub request: &'a crate::node_service_hosting::OperationRequest,
    pub token: &'a str,
    pub audience: &'a basalt::ucan::AudienceDid,
    pub nonce: [u8; 32],
}

/// Produces an exact holder assertion, not a grant or a trust anchor.
pub fn sign_credential<S: basalt::ucan::TokenSigner>(
    input: SigningInput<'_>,
    signer: &S,
) -> crate::error::Result<wire::Credential> {
    let resource = resource(input.bundle_root);
    let signing = basalt::ucan::HolderInvocationSigningRequest::new(
        input.token,
        input.audience,
        &resource,
        &input.request.operation,
        operation_binding(input.bundle_root, input.request)?,
        basalt::ucan::HolderInvocationNonce::from_bytes(input.nonce),
    );
    let signature = basalt::ucan::sign_holder_invocation(&signing, signer, verification_limits(), holder_limits()?)
        .map_err(|_| denied())?;
    Ok(wire::Credential {
        token: input.token.to_owned(),
        nonce: input.nonce,
        signature: signature.as_bytes().try_into().map_err(|_| denied())?,
        proofs: Vec::new(),
    })
}

impl<C: crate::world_witness::WallClock> Current<C> {
    /// `root` is an independently provisioned operator authority capability.
    /// Opening validates trust without creating namespaces or replay state.
    pub fn open(root: &crate::node_state::DirectoryRoot, bundle_root: &str, clock: C) -> crate::error::Result<Self> {
        crate::preserves_rail::ContentRef::parse(bundle_root)?;
        let mut authority = Self {
            root: root.clone(),
            bundle_root: bundle_root.to_owned(),
            clock,
        };
        authority.trust()?;
        Ok(authority)
    }

    fn trust(&mut self) -> crate::error::Result<(wire::Trust, u64)> {
        let path = crate::node_state::RelativePath::parse("service-host-trust")?;
        let trust = wire::decode_trust(&self.root.read(&path, 262144)?)?;
        let now = crate::world_witness::WallClock::unix_seconds(&mut self.clock).map_err(|_| denied())?;
        if now < trust.not_before || now >= trust.expires {
            return Err(denied());
        }
        boundaries::validate_keys(&trust)?;
        Ok((trust, now))
    }
}

impl<C: crate::world_witness::WallClock> crate::node_service_hosting::AuthorityPort for Current<C> {
    fn admit(&mut self, request: &crate::node_service_hosting::OperationRequest) -> crate::error::Result<String> {
        let (trust, now) = self.trust()?;
        let request_ref = request_ref(&self.bundle_root, request)?;
        let credential = wire::decode_credential(&self.root.read(&credential_path(&request_ref)?, 2228224)?)?;
        let resource = resource(&self.bundle_root);
        let operation = operation_binding(&self.bundle_root, request)?;
        let verified_digest = boundaries::verify(&trust, &credential, boundaries::Verification {
            root: &self.root,
            now,
            resource: &resource,
            ability: &request.operation,
            operation,
            contract: "node-service-host",
        })?;
        let value = crate::preserves_rail::record("node-service-host-authorized-v1", vec![
            crate::preserves_rail::string(&request_ref),
            crate::preserves_rail::string(&self.bundle_root),
            crate::preserves_rail::string(crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "node-service-host-policy-v1",
                vec![crate::preserves_rail::string(&trust.policy)],
            ))?),
            crate::preserves_rail::string(verified_digest),
        ]);
        crate::preserves_rail::canonical_hash(&value)
    }
}

/// Exact producer request digest, including the admitted bundle closure root.
pub fn request_ref(
    bundle_root: &str,
    request: &crate::node_service_hosting::OperationRequest,
) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&request_value(bundle_root, request)?)
}

pub fn operation_binding(
    bundle_root: &str,
    request: &crate::node_service_hosting::OperationRequest,
) -> crate::error::Result<basalt::ucan::OperationBinding> {
    let profile = crate::preserves_rail::record("node-service-host-profile-v1", vec![crate::preserves_rail::string(
        "89675cd4f585f837323c049e4a25f7b94c903038",
    )]);
    let bytes = crate::preserves_rail::canonical_bytes(&request_value(bundle_root, request)?)?;
    Ok(basalt::ucan::OperationBinding::new(
        basalt::ucan::OperationProfileId::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&profile)?).as_bytes(),
        ),
        basalt::ucan::RequestBindingDigest::from_bytes(*blake3::hash(&bytes).as_bytes()),
    ))
}

pub fn resource(bundle_root: &str) -> String {
    format!("molten://node-service-host/{bundle_root}")
}

pub fn credential_path(request_ref: &str) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::preserves_rail::ContentRef::parse(request_ref)?;
    crate::node_state::RelativePath::parse(&format!("service-host-credential-{}.preserves", &request_ref[7..]))
}

fn request_value(
    bundle_root: &str,
    request: &crate::node_service_hosting::OperationRequest,
) -> crate::error::Result<preserves::IOValue> {
    crate::preserves_rail::ContentRef::parse(bundle_root)?;
    crate::preserves_rail::ContentRef::parse(&request.state_ref)?;
    crate::preserves_rail::ContentRef::parse(&request.input_ref)?;
    if request.operation.is_empty()
        || request.operation.len() > 128
        || request.subject.is_empty()
        || request.subject.len() > 1024
    {
        return Err(denied());
    }
    Ok(crate::preserves_rail::record("node-service-host-request-v1", vec![
        crate::preserves_rail::string(bundle_root),
        crate::preserves_rail::string(&request.operation),
        crate::preserves_rail::string(&request.subject),
        crate::preserves_rail::u64_value(request.logical_tick),
        crate::preserves_rail::string(&request.state_ref),
        crate::preserves_rail::string(&request.input_ref),
    ]))
}

pub(crate) fn verification_limits() -> basalt::ucan::VerificationLimits {
    basalt::ucan::VerificationLimits::new(MAX_PROOF_DEPTH, MAX_PROOF_COUNT, MAX_COMPACT_TOKEN_BYTES)
}

pub(crate) fn holder_limits() -> crate::error::Result<basalt::ucan::HolderInvocationLimits> {
    basalt::ucan::HolderInvocationLimits::try_new(MAX_AUDIENCE_BYTES, MAX_RESOURCE_BYTES, MAX_ABILITY_BYTES)
        .map_err(|_| denied())
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("service host current authority denied")
}
