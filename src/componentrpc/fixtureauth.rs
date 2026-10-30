//! Fixture-only real signing and replay admission; never production credentials.
use basalt::ucan;
use basalt::ucan::HolderInvocationReplayAdmission;
use basalt::ucan::TokenSigner;

pub const POLICY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/component-rpc/policy.json"));
pub(crate) struct Replay(pub(crate) std::cell::Cell<bool>);
impl HolderInvocationReplayAdmission for Replay {
    fn admit_holder_invocation(
        &self,
        _: &ucan::HolderInvocationReplayCandidate<'_>,
    ) -> std::result::Result<(), ucan::HolderInvocationReplayError> {
        if self.0.replace(true) {
            Err(ucan::HolderInvocationReplayError::Duplicate)
        } else {
            Ok(())
        }
    }
}
pub(crate) struct NoCaveats;
impl ucan::CaveatPolicySet for NoCaveats {
    fn policy_for(&self, _: &ucan::CaveatIdentifier) -> Option<&dyn ucan::CaveatPolicy> {
        None
    }
}
pub(crate) static PROOFS: ucan::ProofCollection = ucan::ProofCollection::empty();

pub fn lease(scope: &super::bridge::Scope, nonce: u8) -> super::Result<super::authority::Lease> {
    issue(scope, nonce, Mode::Allowed)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Allowed,
    WrongGrant,
    WrongPeer,
    InvalidSignature,
    Duplicate,
    PolicyDenied,
    MissingToken,
    Expired,
    OverBudget,
}

pub fn negatives(scope: &super::bridge::Scope) -> super::Result<Vec<&'static str>> {
    let mut cases = Vec::new();
    for (mode, label) in [
        (Mode::WrongGrant, "ucan-wrong-grant"),
        (Mode::WrongPeer, "unauthorized-peer-before-effect"),
        (Mode::InvalidSignature, "ucan-invalid-holder-signature"),
        (Mode::Duplicate, "ucan-duplicate-nonce"),
        (Mode::PolicyDenied, "current-basalt-policy-deny"),
        (Mode::MissingToken, "missing-valid-token"),
        (Mode::Expired, "expired-current-authority"),
        (Mode::OverBudget, "overbudget-before-effect"),
    ] {
        if issue(scope, 9, mode).is_ok() {
            return Err(format!("real UCAN negative fixture was admitted: {label}"));
        }
        cases.push(label);
    }
    Ok(cases)
}

fn issue(scope: &super::bridge::Scope, nonce: u8, mode: Mode) -> super::Result<super::authority::Lease> {
    let original_resource = super::authority::resource(scope);
    let policy = if mode == Mode::PolicyDenied {
        std::borrow::Cow::Owned(POLICY.replace("component-rpc/invoke", "component-rpc/other"))
    } else {
        std::borrow::Cow::Borrowed(POLICY)
    };
    let changed = match mode {
        Mode::WrongPeer => {
            let mut changed = scope.clone();
            changed.peer = super::reference(b"ungranted-peer");
            Some(changed)
        }
        Mode::PolicyDenied => {
            let mut changed = scope.clone();
            changed.policy = super::authority::policy_ref(&policy)?;
            Some(changed)
        }
        Mode::OverBudget => {
            let mut changed = scope.clone();
            changed.resources = super::reference(b"excess-budget");
            Some(changed)
        }
        _ => None,
    };
    let scope = changed.as_ref().unwrap_or(scope);
    let issuer = ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
    let holder = ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
    let audience = ucan::AudienceDid::from(holder.issuer().map_err(super::failure)?);
    let issuer_id = issuer.issuer().map_err(super::failure)?;
    let keys = ucan::KeyResolutionContext::new(vec![
        issuer.verification_key().map_err(super::failure)?,
        holder.verification_key().map_err(super::failure)?,
    ]);
    let resource = super::authority::resource(scope);
    let granted = if mode == Mode::WrongGrant {
        "urn:molten:component-rpc:other-peer".to_string()
    } else {
        original_resource
    };
    let capabilities = ucan::CapabilitySet::new(vec![
        ucan::CapabilityDocument::new(granted, super::authority::ABILITY.to_string()).map_err(super::failure)?,
    ])
    .map_err(super::failure)?;
    let token = ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &ucan::ProofReferences::empty(),
        ucan::TokenTimeBounds::new(100, 200).map_err(super::failure)?,
    )
    .map_err(super::failure)?;
    let nonce = ucan::HolderInvocationNonce::from_bytes([nonce; 32]);
    let signature = ucan::sign_holder_invocation(
        &ucan::HolderInvocationSigningRequest::new(
            token.as_str(),
            &audience,
            &resource,
            super::authority::ABILITY,
            super::authority::binding(scope)?,
            nonce,
        ),
        &holder,
        ucan::VerificationLimits::default(),
        ucan::HolderInvocationLimits::default(),
    )
    .map_err(super::failure)?;
    let signature = if mode == Mode::InvalidSignature {
        ucan::SignerSignature::from_bytes([0; 64])
    } else {
        signature
    };
    let replay = Replay(std::cell::Cell::new(false));
    let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
        verification_time: ucan::VerificationTime::from_unix_seconds(if mode == Mode::Expired { 201 } else { 150 }),
        key_resolver: &keys,
        proof_store: &PROOFS,
        revocations: &ucan::NoRevocations,
        caveat_policies: &NoCaveats,
        replay: &replay,
        verification_limits: ucan::VerificationLimits::default().with_clock_skew_tolerance_seconds(0),
        holder_limits: ucan::HolderInvocationLimits::default(),
    };
    let input = super::authority::Input {
        policy_json: &policy,
        credential: super::authority::Credential {
            token: if mode == Mode::MissingToken { "" } else { token.as_str() },
            trusted_issuer: issuer_id.as_str(),
            expected_holder: audience.as_str(),
            nonce,
            signature: &signature,
        },
        services: &services,
        tick: 10,
        generation: 1,
    };
    let lease = super::authority::admit(scope, &input)?;
    if mode == Mode::Duplicate {
        super::authority::admit(scope, &input)
    } else {
        Ok(lease)
    }
}
