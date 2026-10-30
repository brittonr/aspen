use super::*;

impl Signers {
    pub(crate) fn snapshot(
        &self,
        application: &Application,
        current: CurrentState,
        replay: &NonceLedger,
    ) -> crate::error::Result<ScopeAdmissionSnapshot> {
        let scope = application.state().scopes.get(&space_ref()).ok_or_else(|| denied(Denial::Missing))?;
        let binding = application.state().bindings.values().next().ok_or_else(|| denied(Denial::Missing))?;
        let mut request = AudienceRequest {
            reference: RequestRef::new(reference("pending-audience")).map_err(|_| denied(Denial::Domain))?,
            scope: scope.reference.clone(),
            scope_revision: scope.revision,
            membership_epoch: scope.membership_epoch,
            binding: binding.reference.clone(),
            resource: binding.resource.clone(),
            audience: [scope.owner.clone()].into_iter().collect(),
            ability: Ability::Read,
            target: None,
            target_revision: None,
            target_membership_epoch: None,
            scope_policy: scope.policy.clone(),
            resource_policy: binding.policy.clone(),
            organization_floor: scope.organization_floor.clone(),
            consent: None,
            currentness: current.currentness.clone(),
        };
        request.reference = audience_request_ref(&request)?;
        let resource = format!("molten://collaboration/{}", request.resource.as_str());
        let capabilities = basalt::ucan::CapabilitySet::new(vec![
            basalt::ucan::CapabilityDocument::new(resource.clone(), "resource/read".to_string())
                .map_err(|_| denied(Denial::Authority))?,
        ])
        .map_err(|_| denied(Denial::Authority))?;
        let token = basalt::ucan::issue_token_with_signer(
            &self.issuer,
            &self.audience,
            &capabilities,
            &basalt::ucan::ProofReferences::empty(),
            basalt::ucan::TokenTimeBounds::new(100, 200).map_err(|_| denied(Denial::Authority))?,
        )
        .map_err(|_| denied(Denial::Authority))?;
        let nonce = basalt::ucan::HolderInvocationNonce::from_bytes(
            *blake3::hash(request.reference.as_str().as_bytes()).as_bytes(),
        );
        let signature = basalt::ucan::sign_holder_invocation(
            &basalt::ucan::HolderInvocationSigningRequest::new(
                token.as_str(),
                &self.audience,
                &resource,
                "resource/read",
                operation_binding(&request.reference)?,
                nonce,
            ),
            &self.holder,
            basalt::ucan::VerificationLimits::default(),
            basalt::ucan::HolderInvocationLimits::default(),
        )
        .map_err(|_| denied(Denial::Authority))?;
        let issuer = self.issuer_ref();
        let policies = [PolicyInput {
            reference: policy_ref(POLICY)?,
            json: POLICY,
        }];
        let credentials = [HolderCredential {
            token: token.as_str(),
            issuer: &issuer,
            holder: self.audience.as_str(),
            nonce,
            signature: &signature,
        }];
        let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
            verification_time: basalt::ucan::VerificationTime::from_unix_seconds(150),
            key_resolver: &self.resolver,
            proof_store: &EMPTY_PROOFS,
            revocations: &basalt::ucan::NoRevocations,
            caveat_policies: &NoCaveats,
            replay,
            verification_limits: basalt::ucan::VerificationLimits::default(),
            holder_limits: basalt::ucan::HolderInvocationLimits::default(),
        };
        application.admit_snapshot(SnapshotInput {
            request: &request,
            current,
            policies: &policies,
            credentials: &credentials,
            services: &services,
        })
    }
}
