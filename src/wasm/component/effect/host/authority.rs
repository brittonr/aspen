//! Product mapping over the pinned Basalt policy and UCAN verifier APIs.

use basalt::ucan::OperationProfileAdmission;

pub const BLOB_CONTRACT: &str = "component-blob-get";
pub const BLOB_ABILITY: &str = "blob/get";
pub const BLOB_RESOURCE_PREFIX: &str = "molten://blob/";

pub struct Credential<'a> {
    pub compact_token: &'a str,
    pub nonce: basalt::ucan::HolderInvocationNonce,
    pub holder_signature: &'a basalt::ucan::SignerSignature,
}

pub struct Input<'a> {
    pub policy_json: &'a str,
    pub trusted_issuer: &'a str,
    pub expected_holder: &'a str,
    pub credential: Credential<'a>,
    pub services: &'a basalt::ucan_enforcement::UcanEnforcementBoundaries<'a>,
}

pub fn policy_ref(policy_json: &str) -> super::Result<String> {
    if u64::try_from(policy_json.len())
        .map_err(|_| super::super::invalid("component host policy length is unrepresentable"))?
        > super::super::HOST_PAYLOAD_BYTES
    {
        return Err(super::super::invalid("component host policy exceeds the byte ceiling"));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("component-host-policy-v1", vec![
        crate::preserves_rail::string(policy_json),
    ]))
    .map_err(super::super::convert)
}

pub fn credential_ref(compact_token: &str, trusted_issuer: &str, expected_holder: &str) -> super::Result<String> {
    if u64::try_from(compact_token.len())
        .map_err(|_| super::super::invalid("component host token length is unrepresentable"))?
        > super::super::HOST_PAYLOAD_BYTES
        || trusted_issuer.len() > 1_024
        || expected_holder.len() > 1_024
    {
        return Err(super::super::invalid("component host credential or identity exceeds its ceiling"));
    }
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("component-host-authority-v1", vec![
        crate::preserves_rail::string("onix-compact-ed25519-v1"),
        crate::preserves_rail::string(trusted_issuer),
        crate::preserves_rail::string(expected_holder),
        crate::preserves_rail::string(compact_token),
    ]))
    .map_err(super::super::convert)
}

pub fn blob_resource(request: &super::super::HostRequest) -> super::Result<String> {
    let fields =
        crate::preserves_rail::simple_record_fields(request.payload(), "blob-get", 1).map_err(super::super::convert)?;
    let reference = crate::preserves_rail::required_content_ref(&fields[0], "component blob resource")
        .map_err(super::super::convert)?;
    Ok(format!("{BLOB_RESOURCE_PREFIX}{reference}"))
}

/// Signing helper. This binds assertions only; it does not grant authority.
pub fn holder_binding(
    request: &super::super::HostRequest,
    scope: &super::context::Scope,
) -> super::Result<basalt::ucan::OperationBinding> {
    super::context::validate_scope(scope)?;
    let profile = crate::preserves_rail::record("component-host-operation-profile-v1", vec![
        crate::preserves_rail::string(super::super::HOST_PROFILE_ID),
        crate::preserves_rail::string(super::super::HOST_WIT_SOURCE_REF),
        crate::preserves_rail::string(BLOB_CONTRACT),
        crate::preserves_rail::string(BLOB_ABILITY),
        crate::preserves_rail::string(BLOB_RESOURCE_PREFIX),
    ]);
    let binding = crate::preserves_rail::record("component-host-invocation-v1", vec![
        super::context::scope_value(scope),
        crate::preserves_rail::string(request.request_ref()),
    ]);
    let profile = crate::preserves_rail::canonical_bytes(&profile).map_err(super::super::convert)?;
    let binding = crate::preserves_rail::canonical_bytes(&binding).map_err(super::super::convert)?;
    Ok(basalt::ucan::OperationBinding::new(
        basalt::ucan::OperationProfileId::from_bytes(*blake3::hash(&profile).as_bytes()),
        basalt::ucan::RequestBindingDigest::from_bytes(*blake3::hash(&binding).as_bytes()),
    ))
}

struct ExactProfile(basalt::ucan::OperationProfileId);

impl OperationProfileAdmission for ExactProfile {
    fn admit_operation_profile(
        &self,
        profile: &basalt::ucan::OperationProfileId,
    ) -> std::result::Result<(), basalt::ucan::OperationProfileAdmissionError> {
        if profile == &self.0 {
            Ok(())
        } else {
            Err(basalt::ucan::OperationProfileAdmissionError::UnknownProfile)
        }
    }
}

// r[impl molten.wasm_component_adoption.hostcalls]
pub(super) fn admit(
    request: &super::super::HostRequest,
    scope: &super::context::Scope,
    input: &Input<'_>,
) -> super::Result<preserves::IOValue> {
    let resource = blob_resource(request)?;
    let policy =
        basalt::parse_policy_json(input.policy_json).map_err(|_| denied("component host policy is malformed"))?;
    let verified = verified_grant(input, &resource)?;
    let enforcement = basalt::EnforcementRequest::new(BLOB_CONTRACT, &resource, BLOB_ABILITY)
        .with_capabilities(vec![basalt::CapabilityGrant::new(verified.resource, verified.ability)]);
    if !basalt::enforce(&policy, &enforcement)
        .map_err(|_| denied("component host Basalt enforcement failed"))?
        .is_allowed()
    {
        return Err(denied("component host Basalt policy denied blob retrieval"));
    }
    let operation = holder_binding(request, scope)?;
    let profiles = ExactProfile(operation.profile());
    let services = input.services;
    let invocation = basalt::ucan::HolderInvocationRequest::new(
        input.credential.compact_token,
        &resource,
        BLOB_ABILITY,
        operation,
        input.credential.nonce,
        input.credential.holder_signature,
    );
    let token_context = basalt::ucan::HolderTokenVerificationContext::new(
        services.verification_time,
        services.key_resolver,
        services.proof_store,
        services.revocations,
        services.verification_limits,
    );
    let policy_context = basalt::ucan::HolderInvocationPolicyContext::new(
        services.caveat_policies,
        &profiles,
        services.replay,
        services.holder_limits,
    );
    let policy_ref = policy_ref(input.policy_json)?;
    let authority_ref = credential_ref(input.credential.compact_token, input.trusted_issuer, input.expected_holder)?;
    let authorized = basalt::ucan::verify_holder_signed_invocation(&invocation, &token_context, &policy_context)
        .map_err(|_| denied("component host holder signature or replay admission denied"))?;
    Ok(crate::preserves_rail::record("component-host-authorized-v1", vec![
        crate::preserves_rail::string(request.request_ref()),
        super::context::scope_value(scope),
        crate::preserves_rail::string(policy_ref),
        crate::preserves_rail::string(authority_ref),
        crate::preserves_rail::string(format!(
            "blake3:{}",
            blake3::Hash::from(*authorized.request_digest().as_bytes())
        )),
    ]))
}

fn verified_grant(input: &Input<'_>, resource: &str) -> super::Result<basalt::ucan::CapabilityDocument> {
    let services = input.services;
    let verified = basalt::ucan::verify_compact_token_str_with_resolvers_and_revocations(
        input.credential.compact_token,
        services.verification_time,
        services.key_resolver,
        services.proof_store,
        services.revocations,
        services.verification_limits,
    )
    .map_err(|_| denied("component host UCAN token verification denied"))?;
    if verified.issuer().as_str() != input.trusted_issuer || verified.audience().as_str() != input.expected_holder {
        return Err(denied("component host UCAN issuer or holder differs from current admission"));
    }
    match verified.authorize_with_policies(resource, BLOB_ABILITY, services.caveat_policies) {
        basalt::ucan::AuthorizationDecision::Allowed { matched_capability } => Ok(matched_capability),
        basalt::ucan::AuthorizationDecision::Denied { .. } => Err(denied("component host UCAN scope denied")),
    }
}

fn denied(message: &'static str) -> super::super::super::model::Denial {
    super::super::super::model::Denial::classified(super::super::super::model::DenialClass::AuthorityDenial, message)
}
