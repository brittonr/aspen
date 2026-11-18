//! Fixture-only real signing and replay admission; never production credentials.
use basalt::ucan::HolderInvocationReplayAdmission;
use basalt::ucan::TokenSigner;

pub const POLICY: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/component-rpc/policy.json"));
const MAX_PROOF_DEPTH: usize = 16;
const MAX_PROOF_COUNT: usize = 64;
const MAX_COMPACT_TOKEN_BYTES: usize = 64 * 1024;
const MAX_AUDIENCE_BYTES: usize = 512;
const MAX_RESOURCE_BYTES: usize = 4 * 1024;
const MAX_ABILITY_BYTES: usize = 256;

pub(crate) struct Replay(pub(crate) std::cell::Cell<bool>);
impl HolderInvocationReplayAdmission for Replay {
    fn admit_holder_invocation(
        &self,
        _: &basalt::ucan::HolderInvocationReplayCandidate<'_>,
    ) -> std::result::Result<(), basalt::ucan::HolderInvocationReplayError> {
        if self.0.replace(true) {
            Err(basalt::ucan::HolderInvocationReplayError::Duplicate)
        } else {
            Ok(())
        }
    }
}
pub(crate) struct NoCaveats;
impl basalt::ucan::CaveatPolicySet for NoCaveats {
    fn policy_for(&self, _: &basalt::ucan::CaveatIdentifier) -> Option<&dyn basalt::ucan::CaveatPolicy> {
        None
    }
}
pub(crate) static PROOFS: basalt::ucan::ProofCollection = basalt::ucan::ProofCollection::empty();

/// Token verification bounds for the fixture credentials, stated rather than inherited.
pub(crate) fn verification_limits() -> basalt::ucan::VerificationLimits {
    basalt::ucan::VerificationLimits::new(MAX_PROOF_DEPTH, MAX_PROOF_COUNT, MAX_COMPACT_TOKEN_BYTES)
}

/// Holder-invocation field bounds for the fixture credentials, stated rather than inherited.
pub(crate) fn holder_limits() -> super::Result<basalt::ucan::HolderInvocationLimits> {
    basalt::ucan::HolderInvocationLimits::try_new(MAX_AUDIENCE_BYTES, MAX_RESOURCE_BYTES, MAX_ABILITY_BYTES)
        .map_err(super::failure)
}

pub fn lease(scope: &super::bridge::Scope, nonce: u8) -> super::Result<super::authority::Lease> {
    issue(scope, nonce, Mode::Allowed)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Allowed,
    WrongGrant,
    WrongPeer,
    WrongComponent,
    InvalidSignature,
    Duplicate,
    PolicyDenied,
    MissingToken,
    Expired,
    OverBudget,
    DeliveryDrift,
}

pub fn negatives(scope: &super::bridge::Scope) -> super::Result<Vec<&'static str>> {
    let modes = [
        (Mode::WrongGrant, "ucan-wrong-grant"),
        (Mode::WrongPeer, "unauthorized-peer-before-effect"),
        (Mode::WrongComponent, "ungranted-component-before-effect"),
        (Mode::InvalidSignature, "ucan-invalid-holder-signature"),
        (Mode::Duplicate, "ucan-duplicate-nonce"),
        (Mode::PolicyDenied, "current-basalt-policy-deny"),
        (Mode::MissingToken, "missing-valid-token"),
        (Mode::Expired, "expired-current-authority"),
        (Mode::OverBudget, "overbudget-before-effect"),
        (Mode::DeliveryDrift, "delivery-contract-drift-before-effect"),
    ];
    let mut cases = Vec::with_capacity(modes.len());
    for (mode, label) in modes {
        if issue(scope, 9, mode).is_ok() {
            return Err(format!("real UCAN negative fixture was admitted: {label}"));
        }
        cases.push(label);
    }
    Ok(cases)
}

fn policy(mode: Mode) -> std::borrow::Cow<'static, str> {
    if mode == Mode::PolicyDenied {
        std::borrow::Cow::Owned(POLICY.replace("component-rpc/invoke", "component-rpc/other"))
    } else {
        std::borrow::Cow::Borrowed(POLICY)
    }
}

/// The invocation a mode presents; the fixture token always grants the original scope.
fn presented_scope(
    scope: &super::bridge::Scope,
    mode: Mode,
    policy: &str,
) -> super::Result<Option<super::bridge::Scope>> {
    let changed = match mode {
        Mode::WrongPeer => super::bridge::Scope {
            peer: super::reference(b"ungranted-peer"),
            ..scope.clone()
        },
        Mode::WrongComponent => super::bridge::Scope {
            callee: super::reference(b"ungranted-server-component"),
            ..scope.clone()
        },
        Mode::PolicyDenied => super::bridge::Scope {
            policy: super::authority::policy_ref(policy)?,
            ..scope.clone()
        },
        Mode::OverBudget => super::bridge::Scope {
            resources: super::reference(b"excess-budget"),
            ..scope.clone()
        },
        Mode::DeliveryDrift => super::bridge::Scope {
            delivery: super::reference(b"at-least-once-automatic-retry"),
            ..scope.clone()
        },
        Mode::Allowed
        | Mode::WrongGrant
        | Mode::InvalidSignature
        | Mode::Duplicate
        | Mode::MissingToken
        | Mode::Expired => return Ok(None),
    };
    Ok(Some(changed))
}

fn issue(scope: &super::bridge::Scope, nonce: u8, mode: Mode) -> super::Result<super::authority::Lease> {
    let original_resource = super::authority::resource(scope);
    let policy = policy(mode);
    let changed = presented_scope(scope, mode, &policy)?;
    let scope = changed.as_ref().unwrap_or(scope);
    let issuer = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([71; 32]);
    let holder = basalt::ucan::Ed25519InMemorySigner::from_seed_bytes([73; 32]);
    let audience = basalt::ucan::AudienceDid::from(holder.issuer().map_err(super::failure)?);
    let issuer_id = issuer.issuer().map_err(super::failure)?;
    let keys = basalt::ucan::KeyResolutionContext::new(vec![
        issuer.verification_key().map_err(super::failure)?,
        holder.verification_key().map_err(super::failure)?,
    ]);
    let resource = super::authority::resource(scope);
    let granted = if mode == Mode::WrongGrant {
        "urn:molten:component-rpc:other-peer".to_string()
    } else {
        original_resource
    };
    let capabilities = basalt::ucan::CapabilitySet::new(vec![
        basalt::ucan::CapabilityDocument::new(granted, super::authority::ABILITY.to_string())
            .map_err(super::failure)?,
    ])
    .map_err(super::failure)?;
    let token = basalt::ucan::issue_token_with_signer(
        &issuer,
        &audience,
        &capabilities,
        &basalt::ucan::ProofReferences::empty(),
        basalt::ucan::TokenTimeBounds::new(100, 200).map_err(super::failure)?,
    )
    .map_err(super::failure)?;
    let nonce = basalt::ucan::HolderInvocationNonce::from_bytes([nonce; 32]);
    let signature = basalt::ucan::sign_holder_invocation(
        &basalt::ucan::HolderInvocationSigningRequest::new(
            token.as_str(),
            &audience,
            &resource,
            super::authority::ABILITY,
            super::authority::binding(scope)?,
            nonce,
        ),
        &holder,
        verification_limits(),
        holder_limits()?,
    )
    .map_err(super::failure)?;
    let signature = if mode == Mode::InvalidSignature {
        basalt::ucan::SignerSignature::from_bytes([0; 64])
    } else {
        signature
    };
    let credential = super::authority::Credential {
        token: if mode == Mode::MissingToken { "" } else { token.as_str() },
        trusted_issuer: issuer_id.as_str(),
        expected_holder: audience.as_str(),
        nonce,
        signature: &signature,
    };
    admit_with_services(scope, mode, &policy, credential, &keys)
}

fn admit_with_services(
    scope: &super::bridge::Scope,
    mode: Mode,
    policy: &str,
    credential: super::authority::Credential<'_>,
    keys: &basalt::ucan::KeyResolutionContext,
) -> super::Result<super::authority::Lease> {
    let replay = Replay(std::cell::Cell::new(false));
    let services = basalt::ucan_enforcement::UcanEnforcementBoundaries {
        verification_time: basalt::ucan::VerificationTime::from_unix_seconds(if mode == Mode::Expired {
            201
        } else {
            150
        }),
        key_resolver: keys,
        proof_store: &PROOFS,
        revocations: &basalt::ucan::NoRevocations,
        caveat_policies: &NoCaveats,
        replay: &replay,
        verification_limits: verification_limits().with_clock_skew_tolerance_seconds(0),
        holder_limits: holder_limits()?,
    };
    let input = super::authority::Input {
        policy_json: policy,
        credential,
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
