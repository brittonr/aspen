//! The only live minting boundary for collaboration authority decisions.
//! Tokens, policy bodies, DIDs and signatures are borrowed and never projected.

use super::*;

pub const AUTHORITY_CONTRACT: &str = "collaboration-scope";
pub const AUTHORITY_COHORT: &str = "89675cd4f585f837323c049e4a25f7b94c903038";

pub struct HolderCredential<'a> {
    pub token: &'a str,
    pub issuer: &'a str,
    pub holder: &'a str,
    pub nonce: basalt::ucan::HolderInvocationNonce,
    pub signature: &'a basalt::ucan::SignerSignature,
}

pub struct PolicyInput<'a> {
    pub reference: molten_core::nominal::PolicyRef,
    pub json: &'a str,
}

pub struct DecisionInput<'a> {
    pub request: &'a RequestRef,
    pub subject: &'a SubjectRef,
    pub resource: &'a molten_core::nominal::ResourceRef,
    pub ability: Ability,
    pub currentness: &'a Currentness,
    pub consent: Option<DecisionRef>,
    pub policies: &'a [PolicyInput<'a>],
}

pub fn subject_ref(issuer: &str, holder: &str) -> crate::error::Result<SubjectRef> {
    if issuer.len() > 1024 || holder.len() > 1024 {
        return Err(denied(Denial::Bound));
    }
    let value = crate::preserves_rail::record("collaboration-subject-v1", vec![
        crate::preserves_rail::string(issuer),
        crate::preserves_rail::string(holder),
    ]);
    SubjectRef::new(crate::preserves_rail::canonical_hash(&value)?).map_err(|_| denied(Denial::Domain))
}

pub fn policy_ref(json: &str) -> crate::error::Result<molten_core::nominal::PolicyRef> {
    if json.len() > 65536 {
        return Err(denied(Denial::Bound));
    }
    let value = crate::preserves_rail::record("collaboration-policy-v1", vec![crate::preserves_rail::string(json)]);
    molten_core::nominal::PolicyRef::new(crate::preserves_rail::canonical_hash(&value)?)
        .map_err(|_| denied(Denial::Domain))
}

/// The canonical request digest already binds scope, state, policies, audience,
/// ability, target, generation, logical time, revocation and consent observation.
pub fn operation_binding(request: &RequestRef) -> crate::error::Result<basalt::ucan::OperationBinding> {
    let profile = crate::preserves_rail::record("collaboration-operation-profile-v1", vec![
        crate::preserves_rail::string(SCHEMA),
        crate::preserves_rail::string(AUTHORITY_COHORT),
    ]);
    let binding =
        crate::preserves_rail::record("collaboration-operation-binding-v1", vec![crate::preserves_rail::string(
            request.as_str(),
        )]);
    Ok(basalt::ucan::OperationBinding::new(
        basalt::ucan::OperationProfileId::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&profile)?).as_bytes(),
        ),
        basalt::ucan::RequestBindingDigest::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&binding)?).as_bytes(),
        ),
    ))
}

struct ExactProfile(basalt::ucan::OperationProfileId);
impl basalt::ucan::OperationProfileAdmission for ExactProfile {
    fn admit_operation_profile(
        &self,
        profile: &basalt::ucan::OperationProfileId,
    ) -> Result<(), basalt::ucan::OperationProfileAdmissionError> {
        if profile == &self.0 {
            Ok(())
        } else {
            Err(basalt::ucan::OperationProfileAdmissionError::UnknownProfile)
        }
    }
}

// r[impl molten.collaboration_scope.authority_snapshot]
pub fn verify_decisions(
    input: &DecisionInput<'_>,
    credential: &HolderCredential<'_>,
    services: &basalt::ucan_enforcement::UcanEnforcementBoundaries<'_>,
) -> crate::error::Result<Vec<AuthorityDecision>> {
    if input.policies.is_empty()
        || input.policies.len() > 6
        || credential.token.len() > 65536
        || subject_ref(credential.issuer, credential.holder)? != *input.subject
    {
        return Err(denied(Denial::Authority));
    }
    let verified = basalt::ucan::verify_compact_token_str_with_resolvers_and_revocations(
        credential.token,
        services.verification_time,
        services.key_resolver,
        services.proof_store,
        services.revocations,
        services.verification_limits,
    )
    .map_err(|_| denied(Denial::Authority))?;
    if verified.issuer().as_str() != credential.issuer || verified.audience().as_str() != credential.holder {
        return Err(denied(Denial::Authority));
    }
    let resource = format!("molten://collaboration/{}", input.resource.as_str());
    let matched = match verified.authorize_with_policies(&resource, input.ability.as_str(), services.caveat_policies) {
        basalt::ucan::AuthorizationDecision::Allowed { matched_capability } => matched_capability,
        basalt::ucan::AuthorizationDecision::Denied { .. } => return Err(denied(Denial::Authority)),
    };
    for policy in input.policies {
        if policy_ref(policy.json)? != policy.reference {
            return Err(denied(Denial::Stale));
        }
        let parsed = basalt::parse_policy_json(policy.json).map_err(|_| denied(Denial::Authority))?;
        let request = basalt::EnforcementRequest::new(AUTHORITY_CONTRACT, &resource, input.ability.as_str())
            .with_capabilities(vec![basalt::CapabilityGrant::new(
                matched.resource.clone(),
                matched.ability.clone(),
            )]);
        if !basalt::enforce(&parsed, &request).map_err(|_| denied(Denial::Authority))?.is_allowed() {
            return Err(denied(Denial::Authority));
        }
    }
    let verified_digest = verify_invocation(input.request, input.ability, &resource, credential, services)?;
    input.policies.iter().map(|policy| decision(input, &policy.reference, &verified_digest)).collect()
}

fn verify_invocation(
    request: &RequestRef,
    ability: Ability,
    resource: &str,
    credential: &HolderCredential<'_>,
    services: &basalt::ucan_enforcement::UcanEnforcementBoundaries<'_>,
) -> crate::error::Result<String> {
    let operation = operation_binding(request)?;
    let profiles = ExactProfile(operation.profile());
    let invocation = basalt::ucan::HolderInvocationRequest::new(
        credential.token,
        resource,
        ability.as_str(),
        operation,
        credential.nonce,
        credential.signature,
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
    basalt::ucan::verify_holder_signed_invocation(&invocation, &token_context, &policy_context)
        .map(|authorized| format!("blake3:{}", blake3::Hash::from(*authorized.request_digest().as_bytes())))
        .map_err(|_| denied(Denial::Replay))
}

fn decision(
    input: &DecisionInput<'_>,
    policy: &molten_core::nominal::PolicyRef,
    verified_digest: &str,
) -> crate::error::Result<AuthorityDecision> {
    let binding = crate::preserves_rail::record("collaboration-verified-authority-v1", vec![
        crate::preserves_rail::string(input.request.as_str()),
        crate::preserves_rail::string(input.subject.as_str()),
        crate::preserves_rail::string(policy.as_str()),
        crate::preserves_rail::string(input.ability.as_str()),
        currentness_value(input.currentness),
        crate::preserves_rail::string(AUTHORITY_COHORT),
    ]);
    let basalt = crate::preserves_rail::record("collaboration-basalt-decision-v1", vec![binding.clone()]);
    let ucan = crate::preserves_rail::record("collaboration-ucan-decision-v1", vec![
        binding,
        crate::preserves_rail::string(verified_digest),
    ]);
    Ok(AuthorityDecision {
        request: input.request.clone(),
        subject: input.subject.clone(),
        policy: policy.clone(),
        ability: input.ability,
        basalt: DecisionRef::new(crate::preserves_rail::canonical_hash(&basalt)?)
            .map_err(|_| denied(Denial::Domain))?,
        ucan: DecisionRef::new(crate::preserves_rail::canonical_hash(&ucan)?).map_err(|_| denied(Denial::Domain))?,
        consent: input.consent.clone(),
        currentness: input.currentness.clone(),
        is_allowed: true,
    })
}
