mod invocation;
mod replay;
mod state;
mod verify;
pub mod wire;
pub use state::provision_revocations;

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
    replay: durable_authority_state_redb::ledger::ReplayRedbStore,
    revocations: durable_authority_state_redb::view::RevocationRedbStore,
    clock: C,
}

pub struct Admission {
    pub authentication: crate::world_head::WorldHeadAuthenticationResult,
    pub authority: molten_core::world_head::WorldHeadAuthorityObservation,
}

impl Admission {
    pub fn authentication_ref(&self) -> &str {
        self.authentication.observation.decision_ref.as_str()
    }
    pub fn authority_ref(&self) -> &str {
        self.authority.authority_ref.as_str()
    }
}

pub struct SigningInput<'a> {
    pub claim: &'a crate::world_head::CanonicalWorldHeadClaim,
    pub token: &'a str,
    pub audience: &'a basalt::ucan::AudienceDid,
    pub nonce: [u8; 32],
    pub artifact_signature: Vec<u8>,
}

pub fn sign_credential<S: basalt::ucan::TokenSigner>(
    input: SigningInput<'_>,
    signer: &S,
) -> crate::error::Result<wire::Credential> {
    let resource = resource(&input.claim.claim.branch_id);
    let signing = basalt::ucan::HolderInvocationSigningRequest::new(
        input.token,
        input.audience,
        &resource,
        "mutate",
        operation_binding(input.claim)?,
        basalt::ucan::HolderInvocationNonce::from_bytes(input.nonce),
    );
    let signature = basalt::ucan::sign_holder_invocation(&signing, signer, verification_limits(), holder_limits()?)
        .map_err(|_| denied())?;
    Ok(wire::Credential {
        token: input.token.to_owned(),
        nonce: input.nonce,
        signature: signature.as_bytes().to_vec(),
        proofs: Vec::new(),
        artifact_signature: input.artifact_signature,
    })
}

impl<C: super::WallClock> Current<C> {
    pub fn open(
        root: &crate::node_state::DirectoryRoot,
        replay: durable_authority_state_redb::ledger::ReplayRedbStore,
        revocations: durable_authority_state_redb::view::RevocationRedbStore,
        clock: C,
    ) -> crate::error::Result<Self> {
        let mut authority = Self {
            root: root.clone(),
            replay,
            revocations,
            clock,
        };
        authority.trust()?;
        Ok(authority)
    }

    pub fn witness_policy_ref(&mut self) -> crate::error::Result<String> {
        let (trust, _) = self.trust()?;
        crate::preserves_rail::ContentRef::parse(&trust.witness_policy_ref)?;
        Ok(trust.witness_policy_ref)
    }

    fn trust(&mut self) -> crate::error::Result<(wire::Trust, u64)> {
        let trust: wire::Trust = wire::load(&self.root, &wire::trust_path()?)?;
        let now = super::WallClock::unix_seconds(&mut self.clock).map_err(|_| denied())?;
        if now < trust.not_before || now >= trust.expires || trust.authority_generation == 0 {
            return Err(denied());
        }
        invocation::validate_keys(&trust)?;
        Ok((trust, now))
    }

    pub fn observe(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
    ) -> crate::error::Result<Admission> {
        let (trust, now) = self.trust()?;
        if trust.head_policy_ref != claim.claim.policy_ref.as_str()
            || observed.map_or(0, |state| state.generation) != claim.claim.expected_generation
            || observed.map(|state| &state.head) != claim.claim.expected_head.as_ref()
            || observed.is_some_and(|state| {
                state.branch_id != claim.claim.branch_id || state.policy_ref != claim.claim.policy_ref
            })
        {
            return Err(denied());
        }
        let credential: wire::Credential = wire::load(&self.root, &wire::credential_path(claim.claim_ref.as_str())?)?;
        let digest = invocation::authorize(self, claim, &trust, &credential, now)?;
        let authentication = verify::authenticate(claim, &trust, &credential)?;
        let authority_ref =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("world-head-authority-v1", vec![
                crate::preserves_rail::string(claim.claim_ref.as_str()),
                crate::preserves_rail::string(trust.identity()?),
                crate::preserves_rail::string(digest),
            ]))?;
        Ok(Admission {
            authentication,
            authority: molten_core::world_head::WorldHeadAuthorityObservation {
                authority_ref: molten_core::world_head::WorldHeadAuthorityRef::new(authority_ref)
                    .map_err(|_| denied())?,
                policy_ref: claim.claim.policy_ref.clone(),
                admitted: true,
                observed_generation: claim.claim.expected_generation,
            },
        })
    }
}

impl<C: super::WallClock> super::FreshAdmission for Current<C> {
    fn witness_policy_ref(&mut self) -> crate::error::Result<String> {
        Current::witness_policy_ref(self)
    }

    fn recheck(
        &mut self,
        claim: &crate::world_head::CanonicalWorldHeadClaim,
        observed: Option<&molten_core::world_head::WorldHeadState>,
    ) -> crate::error::Result<crate::world_head::WorldHeadFreshAdmission> {
        let admission = self.observe(claim, observed)?;
        Ok(crate::world_head::WorldHeadFreshAdmission {
            authentication_passed: admission.authentication.observation.passed,
            authority: admission.authority,
        })
    }
}

pub fn resource(branch: &molten_core::world_head::WorldBranchId) -> String {
    format!("molten://world-head/{branch}")
}

pub fn operation_binding(
    claim: &crate::world_head::CanonicalWorldHeadClaim,
) -> crate::error::Result<basalt::ucan::OperationBinding> {
    let profile = crate::preserves_rail::record("world-head-mutation-profile-v1", vec![crate::preserves_rail::string(
        "89675cd4f585f837323c049e4a25f7b94c903038",
    )]);
    Ok(basalt::ucan::OperationBinding::new(
        basalt::ucan::OperationProfileId::from_bytes(
            *blake3::hash(&crate::preserves_rail::canonical_bytes(&profile)?).as_bytes(),
        ),
        basalt::ucan::RequestBindingDigest::from_bytes(*blake3::hash(&claim.bytes).as_bytes()),
    ))
}

fn verification_limits() -> basalt::ucan::VerificationLimits {
    basalt::ucan::VerificationLimits::new(MAX_PROOF_DEPTH, MAX_PROOF_COUNT, MAX_COMPACT_TOKEN_BYTES)
}

fn holder_limits() -> crate::error::Result<basalt::ucan::HolderInvocationLimits> {
    basalt::ucan::HolderInvocationLimits::try_new(MAX_AUDIENCE_BYTES, MAX_RESOURCE_BYTES, MAX_ABILITY_BYTES)
        .map_err(|_| denied())
}

fn denied() -> crate::error::Failure {
    crate::error::Failure::invalid_harness("world head live authority denied")
}
