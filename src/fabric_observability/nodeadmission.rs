//! Separately provisioned Linux observer admission; source evidence is never permission.

const MAX_PROOF_DEPTH: usize = 16;
const MAX_PROOF_COUNT: usize = 64;
const MAX_COMPACT_TOKEN_BYTES: usize = 65_536;
const MAX_AUDIENCE_BYTES: usize = 512;
const MAX_RESOURCE_BYTES: usize = 4_096;
const MAX_ABILITY_BYTES: usize = 256;

mod observerlease;
#[cfg(test)]
mod observertests;
mod observerwire;
pub use observerlease::ObserverAuthorityLease;
pub use observerlease::admit_node_observation;
pub use observerwire::decode_node_observer_credential;
pub use observerwire::decode_node_observer_trust;
pub use observerwire::encode_node_observer_credential;
pub use observerwire::encode_node_observer_trust;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeObservationSelection {
    pub source_validation_ref: String,
    pub checked_profile_path: String,
    pub checked_profile_ref: String,
    pub endpoint_directory: String,
    pub endpoint_leaf: String,
    pub endpoint_content_ref: String,
    pub epoch: u64,
    pub authorization_tick: u64,
    pub boot_id: String,
    pub onix_receipt_ref: String,
}

pub fn decode_node_observation_selection(bytes: &[u8]) -> crate::error::Result<NodeObservationSelection> {
    if bytes.len() > 16384 {
        return Err(denied());
    }
    let selection = serde_json::from_slice(bytes).map_err(|_| denied())?;
    selection_value(&selection)?;
    Ok(selection)
}

pub fn node_observation_selection_ref(selection: &NodeObservationSelection) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&selection_value(selection)?)
}

pub fn node_observation_request(
    selection: &NodeObservationSelection,
) -> crate::error::Result<crate::node_service_hosting::OperationRequest> {
    Ok(crate::node_service_hosting::OperationRequest {
        operation: "read".into(),
        subject: node_observation_resource(selection)?,
        logical_tick: selection.authorization_tick,
        state_ref: selection.source_validation_ref.clone(),
        input_ref: node_observation_selection_ref(selection)?,
    })
}

pub fn node_observation_resource(selection: &NodeObservationSelection) -> crate::error::Result<String> {
    Ok(format!("molten://linux-observation/{}", node_observation_selection_ref(selection)?))
}

pub fn node_observation_binding(
    selection: &NodeObservationSelection,
) -> crate::error::Result<basalt::ucan::OperationBinding> {
    let profile =
        crate::preserves_rail::record("linux-observation-reader-profile-v1", vec![crate::preserves_rail::string(
            "89675cd4f585f837323c049e4a25f7b94c903038",
        )]);
    let request = node_observation_request(selection)?;
    let value = crate::preserves_rail::record("linux-observation-request-v1", vec![
        crate::preserves_rail::string(request.operation),
        crate::preserves_rail::string(request.subject),
        crate::preserves_rail::u64_value(request.logical_tick),
        crate::preserves_rail::string(request.state_ref),
        crate::preserves_rail::string(request.input_ref),
    ]);
    Ok(basalt::ucan::OperationBinding::new(
        basalt::ucan::OperationProfileId::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&profile)?).as_bytes(),
        ),
        basalt::ucan::RequestBindingDigest::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&value)?).as_bytes(),
        ),
    ))
}

pub struct ObserverSigningInput<'a> {
    pub selection: &'a NodeObservationSelection,
    pub token: &'a str,
    pub audience: &'a basalt::ucan::AudienceDid,
    pub nonce: [u8; 32],
}

pub fn sign_node_observation_credential<S: basalt::ucan::TokenSigner>(
    input: ObserverSigningInput<'_>,
    signer: &S,
) -> crate::error::Result<crate::node_service_host::authority::wire::Credential> {
    let resource = node_observation_resource(input.selection)?;
    let signing = basalt::ucan::HolderInvocationSigningRequest::new(
        input.token,
        input.audience,
        &resource,
        "read",
        node_observation_binding(input.selection)?,
        basalt::ucan::HolderInvocationNonce::from_bytes(input.nonce),
    );
    let signature = basalt::ucan::sign_holder_invocation(&signing, signer, verification_limits(), holder_limits()?)
        .map_err(|_| denied())?;
    Ok(crate::node_service_host::authority::wire::Credential {
        token: input.token.into(),
        nonce: input.nonce,
        signature: signature.as_bytes().try_into().map_err(|_| denied())?,
        proofs: Vec::new(),
    })
}

fn verification_limits() -> basalt::ucan::VerificationLimits {
    basalt::ucan::VerificationLimits::new(MAX_PROOF_DEPTH, MAX_PROOF_COUNT, MAX_COMPACT_TOKEN_BYTES)
}

fn holder_limits() -> crate::error::Result<basalt::ucan::HolderInvocationLimits> {
    basalt::ucan::HolderInvocationLimits::try_new(MAX_AUDIENCE_BYTES, MAX_RESOURCE_BYTES, MAX_ABILITY_BYTES)
        .map_err(|_| denied())
}

pub fn node_observer_credential_path(
    selection: &NodeObservationSelection,
) -> crate::error::Result<crate::node_state::RelativePath> {
    let reference = node_observation_selection_ref(selection)?;
    crate::node_state::RelativePath::parse(&format!("linux-observation-credential-{}.preserves", &reference[7..]))
}

fn selection_value(s: &NodeObservationSelection) -> crate::error::Result<preserves::IOValue> {
    for reference in [
        &s.source_validation_ref,
        &s.checked_profile_ref,
        &s.endpoint_content_ref,
        &s.onix_receipt_ref,
    ] {
        crate::preserves_rail::ContentRef::parse(reference)?;
    }
    for path in [&s.checked_profile_path, &s.endpoint_directory, &s.endpoint_leaf] {
        if path.len() > 1024 || path.is_empty() || path.split('/').any(|p| p.is_empty() || p == "." || p == "..") {
            return Err(denied());
        }
        crate::node_state::RelativePath::parse(path)?;
    }
    if s.endpoint_leaf.contains('/')
        || s.boot_id.len() != 36
        || !s.boot_id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err(denied());
    }
    Ok(crate::preserves_rail::record("linux-observation-selection-v1", vec![
        crate::preserves_rail::string(&s.source_validation_ref),
        crate::preserves_rail::string(&s.checked_profile_path),
        crate::preserves_rail::string(&s.checked_profile_ref),
        crate::preserves_rail::string(&s.endpoint_directory),
        crate::preserves_rail::string(&s.endpoint_leaf),
        crate::preserves_rail::string(&s.endpoint_content_ref),
        crate::preserves_rail::u64_value(s.epoch),
        crate::preserves_rail::u64_value(s.authorization_tick),
        crate::preserves_rail::string(&s.boot_id),
        crate::preserves_rail::string(&s.onix_receipt_ref),
    ]))
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("Linux observer authority denied")
}
