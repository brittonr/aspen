//! Current Basalt/UCAN authorization. Possession of an artifact never mints this lease.
use basalt::ucan;
use basalt::ucan::OperationProfileAdmission;

use crate::codec;

pub const ABILITY: &str = "component-rpc/invoke";
pub const CONTRACT: &str = "component-rpc-unary-v1";

pub struct Credential<'a> {
    pub token: &'a str,
    pub trusted_issuer: &'a str,
    pub expected_holder: &'a str,
    pub nonce: ucan::HolderInvocationNonce,
    pub signature: &'a ucan::SignerSignature,
}
pub struct Input<'a> {
    pub policy_json: &'a str,
    pub credential: Credential<'a>,
    pub services: &'a basalt::ucan_enforcement::UcanEnforcementBoundaries<'a>,
    pub tick: u64,
    pub generation: u64,
}

#[derive(Debug)]
pub struct Lease {
    scope: super::bridge::Scope,
    receipt: String,
}
impl Lease {
    pub fn scope(&self) -> &super::bridge::Scope {
        &self.scope
    }
    pub fn receipt(&self) -> &str {
        &self.receipt
    }
    pub fn check(&self, tick: u64, generation: u64) -> super::Result<()> {
        self.scope.validate(tick)?;
        if generation != self.scope.generation {
            return Err("authority generation changed".into());
        }
        Ok(())
    }
}

pub fn resource(scope: &super::bridge::Scope) -> String {
    format!(
        "urn:molten:component-rpc:{}:{}:{}:{}:{}:{}",
        scope.callee,
        scope.wit,
        scope.peer,
        scope.caller_peer,
        super::INTERFACE,
        super::FUNCTION
    )
}
pub fn policy_ref(json: &str) -> super::Result<String> {
    if json.len() > 65_536 {
        return Err("policy exceeds byte ceiling".into());
    }
    super::hash(&codec::record("component-rpc-policy-v1", vec![codec::string(json)]))
}
pub fn binding(scope: &super::bridge::Scope) -> super::Result<ucan::OperationBinding> {
    let profile = codec::record("component-rpc-operation-v1", vec![
        codec::string(super::PROFILE),
        codec::string(super::REVISION),
        codec::string(ABILITY),
    ]);
    let bytes = codec::canonical_bytes(&scope.value()).map_err(super::failure)?;
    let profile = codec::canonical_bytes(&profile).map_err(super::failure)?;
    Ok(ucan::OperationBinding::new(
        ucan::OperationProfileId::from_bytes(*blake3::hash(&profile).as_bytes()),
        ucan::RequestBindingDigest::from_bytes(*blake3::hash(&bytes).as_bytes()),
    ))
}
struct ExactProfile(ucan::OperationProfileId);
impl OperationProfileAdmission for ExactProfile {
    fn admit_operation_profile(
        &self,
        profile: &ucan::OperationProfileId,
    ) -> std::result::Result<(), ucan::OperationProfileAdmissionError> {
        if profile == &self.0 {
            Ok(())
        } else {
            Err(ucan::OperationProfileAdmissionError::UnknownProfile)
        }
    }
}

// r[impl molten.component_rpc.authority]
pub fn admit(scope: &super::bridge::Scope, input: &Input<'_>) -> super::Result<Lease> {
    scope.validate(input.tick)?;
    if input.generation != scope.generation || policy_ref(input.policy_json)? != scope.policy {
        return Err("current policy or generation differs from invocation".into());
    }
    let receipt = authorize(
        Operation {
            contract: CONTRACT,
            ability: ABILITY,
            resource: resource(scope),
            binding: binding(scope)?,
            value: scope.value(),
        },
        input,
    )?;
    Ok(Lease {
        scope: scope.clone(),
        receipt,
    })
}

pub(crate) struct Operation {
    pub contract: &'static str,
    pub ability: &'static str,
    pub resource: String,
    pub binding: ucan::OperationBinding,
    pub value: preserves::IOValue,
}

pub(crate) fn authorize(operation: Operation, input: &Input<'_>) -> super::Result<String> {
    let grant = grant(input, &operation.resource, operation.ability)?;
    let policy = basalt::parse_policy_json(input.policy_json).map_err(super::failure)?;
    let enforcement = basalt::EnforcementRequest::new(operation.contract, &operation.resource, operation.ability)
        .with_capabilities(vec![basalt::CapabilityGrant::new(grant.resource, grant.ability)]);
    if !basalt::enforce(&policy, &enforcement).map_err(super::failure)?.is_allowed() {
        return Err("Basalt policy denied operation".into());
    }
    let profiles = ExactProfile(operation.binding.profile());
    let services = input.services;
    let request = ucan::HolderInvocationRequest::new(
        input.credential.token,
        &operation.resource,
        operation.ability,
        operation.binding,
        input.credential.nonce,
        input.credential.signature,
    );
    let token = ucan::HolderTokenVerificationContext::new(
        services.verification_time,
        services.key_resolver,
        services.proof_store,
        services.revocations,
        services.verification_limits,
    );
    let policy = ucan::HolderInvocationPolicyContext::new(
        services.caveat_policies,
        &profiles,
        services.replay,
        services.holder_limits,
    );
    let verified = ucan::verify_holder_signed_invocation(&request, &token, &policy).map_err(super::failure)?;
    super::hash(&codec::record("component-rpc-authorized-v1", vec![
        operation.value,
        codec::string(operation.contract),
        codec::string(operation.ability),
        codec::string(format!("blake3:{}", blake3::Hash::from(*verified.request_digest().as_bytes()))),
    ]))
}

fn grant(input: &Input<'_>, resource: &str, ability: &str) -> super::Result<ucan::CapabilityDocument> {
    if input.credential.token.len() > 65_536 {
        return Err("UCAN exceeds byte ceiling".into());
    }
    let services = input.services;
    let verified = ucan::verify_compact_token_str_with_resolvers_and_revocations(
        input.credential.token,
        services.verification_time,
        services.key_resolver,
        services.proof_store,
        services.revocations,
        services.verification_limits,
    )
    .map_err(super::failure)?;
    if verified.issuer().as_str() != input.credential.trusted_issuer
        || verified.audience().as_str() != input.credential.expected_holder
    {
        return Err("UCAN issuer or holder mismatched".into());
    }
    match verified.authorize_with_policies(resource, ability, services.caveat_policies) {
        ucan::AuthorizationDecision::Allowed { matched_capability } => Ok(matched_capability),
        ucan::AuthorizationDecision::Denied { .. } => Err("UCAN scope denied".into()),
    }
}
