static PROOFS: basalt::ucan::ProofCollection = basalt::ucan::ProofCollection::empty();

// The pinned verifier bounds, stated explicitly rather than inherited from its defaults.
const MAX_PROOF_DEPTH: usize = 16;
const MAX_PROOF_COUNT: usize = 64;
const MAX_COMPACT_TOKEN_BYTES: usize = 65_536;
const MAX_AUDIENCE_BYTES: usize = 512;
const MAX_RESOURCE_BYTES: usize = 4_096;
const MAX_ABILITY_BYTES: usize = 256;

pub(super) fn stage(
    application: &mut crate::collaboration_scope::Application,
    request: &crate::collaboration_scope::MutationRequest,
    current: crate::collaboration_scope::CurrentState,
    inputs: (&super::config::Trust, &super::config::Credential, &crate::collaboration_scope::NonceLedger),
) -> crate::error::Result<crate::fabric_consistency::raft::Command> {
    let (trust, credential, replay) = inputs;
    let policies = trust
        .policies
        .iter()
        .map(|json| {
            Ok(crate::collaboration_scope::PolicyInput {
                reference: crate::collaboration_scope::policy_ref(json)?,
                json,
            })
        })
        .collect::<crate::error::Result<Vec<_>>>()?;
    let credential = crate::collaboration_scope::HolderCredential {
        token: &credential.token,
        issuer: &trust.issuer,
        holder: &trust.holder,
        nonce: credential.nonce,
        signature: &credential.signature,
    };
    let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
        verification_time: basalt::ucan::VerificationTime::from_unix_seconds(trust.unix_seconds),
        key_resolver: &trust.keys,
        proof_store: &PROOFS,
        revocations: &trust.revocations,
        caveat_policies: &basalt::ucan::token::DenyUnknownCaveats,
        replay,
        verification_limits: basalt::ucan::VerificationLimits::new(
            MAX_PROOF_DEPTH,
            MAX_PROOF_COUNT,
            MAX_COMPACT_TOKEN_BYTES,
        )
        .with_clock_skew_tolerance_seconds(0),
        holder_limits: basalt::ucan::HolderInvocationLimits::try_new(
            MAX_AUDIENCE_BYTES,
            MAX_RESOURCE_BYTES,
            MAX_ABILITY_BYTES,
        )
        .map_err(|_| crate::collaboration_scope::denied(crate::collaboration_scope::Denial::Bound))?,
    };
    application.stage_mutation(crate::collaboration_scope::MutationInput {
        request,
        current,
        policies: &policies,
        credential: &credential,
        services: &services,
    })
}
