use basalt::ucan::TokenSigner;

use super::*;

pub(crate) struct Signers {
    pub(super) issuer: basalt::ucan::Ed25519InMemorySigner,
    pub(super) holder: basalt::ucan::Ed25519InMemorySigner,
    pub(crate) audience: basalt::ucan::AudienceDid,
    pub(super) resolver: basalt::ucan::KeyResolutionContext,
}
#[derive(Clone, Copy)]
pub(crate) enum TokenFault {
    Signature,
    Expired,
    Revoked,
    WrongAbility,
}
pub(crate) struct StageCase {
    pub(crate) mutation: Mutation,
    pub(crate) fault: Option<TokenFault>,
}

impl Signers {
    pub(crate) fn new() -> Self {
        let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
        let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
        let audience = basalt::ucan::AudienceDid::from(holder.issuer().expect("holder"));
        let resolver = basalt::ucan::KeyResolutionContext::new(vec![
            issuer.verification_key().expect("issuer key"),
            holder.verification_key().expect("holder key"),
        ]);
        Self {
            issuer,
            holder,
            audience,
            resolver,
        }
    }
    pub(crate) fn issuer_ref(&self) -> String {
        self.issuer.issuer().expect("issuer").as_str().to_string()
    }
    pub(crate) fn stage(
        &self,
        application: &mut Application,
        current: CurrentState,
        mutation: Mutation,
        replay: &NonceLedger,
    ) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
        self.stage_case(application, current, StageCase { mutation, fault: None }, replay)
    }
    pub(crate) fn stage_case(
        &self,
        application: &mut Application,
        current: CurrentState,
        case: StageCase,
        replay: &NonceLedger,
    ) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
        let request = self.request(application, &current, case.mutation)?;
        let resource = format!("molten://collaboration/{}", mutation_resource(&request.mutation)?.as_str());
        let ability = request.mutation.ability().as_str();
        let grant = if matches!(case.fault, Some(TokenFault::WrongAbility)) {
            "resource/read"
        } else {
            ability
        };
        let token = self.issue(&resource, grant)?;
        let (nonce, signature) = self.sign(&token, &request.reference, &resource, ability)?;
        let bad_signature = basalt::ucan::SignerSignature::from_bytes([0; 64]);
        let signature = if matches!(case.fault, Some(TokenFault::Signature)) {
            &bad_signature
        } else {
            &signature
        };
        let issuer = self.issuer_ref();
        let policies = [PolicyInput {
            reference: policy_ref(POLICY)?,
            json: POLICY,
        }];
        let credential = HolderCredential {
            token: token.as_str(),
            issuer: &issuer,
            holder: self.audience.as_str(),
            nonce,
            signature,
        };
        let revocations = if matches!(case.fault, Some(TokenFault::Revoked)) {
            basalt::ucan::RevocationList::from_references(vec![basalt::ucan::proof_reference(&token)])
        } else {
            basalt::ucan::RevocationList::empty()
        };
        let now = if matches!(case.fault, Some(TokenFault::Expired)) {
            250
        } else {
            150
        };
        let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
            verification_time: basalt::ucan::VerificationTime::from_unix_seconds(now),
            key_resolver: &self.resolver,
            proof_store: &EMPTY_PROOFS,
            revocations: &revocations,
            caveat_policies: &NoCaveats,
            replay,
            verification_limits: basalt::ucan::VerificationLimits::default().with_clock_skew_tolerance_seconds(0),
            holder_limits: basalt::ucan::HolderInvocationLimits::default(),
        };
        application.stage_mutation(MutationInput {
            request: &request,
            current,
            policies: &policies,
            credential: &credential,
            services: &services,
        })
    }
    fn request(
        &self,
        application: &Application,
        current: &CurrentState,
        mutation: Mutation,
    ) -> crate::error::Result<MutationRequest> {
        let mut request = MutationRequest {
            operation: molten_core::nominal::OperationRef::new(format!(
                "operation-{}",
                current.currentness.committed_index
            ))
            .map_err(|_| denied(Denial::Domain))?,
            reference: RequestRef::new(reference("pending-request")).map_err(|_| denied(Denial::Domain))?,
            before: application.state_ref().clone(),
            actor: subject_ref(&self.issuer_ref(), self.audience.as_str())?,
            currentness: current.currentness.clone(),
            mutation,
        };
        request.reference = mutation_request_ref(&request)?;
        Ok(request)
    }
    fn issue(&self, resource: &str, ability: &str) -> crate::error::Result<basalt::ucan::CompactToken> {
        let capabilities = basalt::ucan::CapabilitySet::new(vec![
            basalt::ucan::CapabilityDocument::new(resource.to_string(), ability.to_string())
                .map_err(|_| denied(Denial::Authority))?,
        ])
        .map_err(|_| denied(Denial::Authority))?;
        basalt::ucan::issue_token_with_signer(
            &self.issuer,
            &self.audience,
            &capabilities,
            &basalt::ucan::ProofReferences::empty(),
            basalt::ucan::TokenTimeBounds::new(100, 200).map_err(|_| denied(Denial::Authority))?,
        )
        .map_err(|_| denied(Denial::Authority))
    }
    fn sign(
        &self,
        token: &basalt::ucan::CompactToken,
        request: &RequestRef,
        resource: &str,
        ability: &str,
    ) -> crate::error::Result<(basalt::ucan::HolderInvocationNonce, basalt::ucan::SignerSignature)> {
        let nonce =
            basalt::ucan::HolderInvocationNonce::from_bytes(*blake3::hash(request.as_str().as_bytes()).as_bytes());
        let signature = basalt::ucan::sign_holder_invocation(
            &basalt::ucan::HolderInvocationSigningRequest::new(
                token.as_str(),
                &self.audience,
                resource,
                ability,
                operation_binding(request)?,
                nonce,
            ),
            &self.holder,
            basalt::ucan::VerificationLimits::default(),
            basalt::ucan::HolderInvocationLimits::default(),
        )
        .map_err(|_| denied(Denial::Authority))?;
        Ok((nonce, signature))
    }
}
