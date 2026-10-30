//! Isolated fixture signing roots, never selected by normal actor execution.
use std::cell;
use std::collections;

use basalt::ucan;

use super::admission;
use super::authority;
use super::continuation;

pub(crate) const POLICY: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/config/spacewasm/fixturepolicy.json"));
static PROOFS: ucan::ProofCollection = ucan::ProofCollection::empty();
struct Caveats;
impl ucan::CaveatPolicySet for Caveats {
    fn policy_for(&self, _: &ucan::CaveatIdentifier) -> Option<&dyn ucan::CaveatPolicy> {
        None
    }
}

struct Replay(cell::RefCell<collections::BTreeSet<(String, [u8; 32])>>);
impl ucan::HolderInvocationReplayAdmission for Replay {
    fn admit_holder_invocation(
        &self,
        candidate: &ucan::HolderInvocationReplayCandidate<'_>,
    ) -> Result<(), ucan::HolderInvocationReplayError> {
        let key = (candidate.audience().as_str().to_owned(), *candidate.nonce().as_bytes());
        let mut seen = self.0.borrow_mut();
        if seen.len() >= 64 {
            return Err(ucan::HolderInvocationReplayError::Backend);
        }
        if seen.insert(key) {
            Ok(())
        } else {
            Err(ucan::HolderInvocationReplayError::Duplicate)
        }
    }
}

pub(crate) struct Authority {
    current: cell::RefCell<continuation::Binding>,
    holder: ucan::Ed25519InMemorySigner,
    audience: ucan::AudienceDid,
    issuer: String,
    token: String,
    keys: ucan::KeyResolutionContext,
    nonce: cell::Cell<u8>,
    replay: Replay,
}

impl Authority {
    pub fn new(mut binding: continuation::Binding) -> Result<Self, String> {
        if binding.actor != "fixture" || binding.generation != 1 {
            return Err("isolated fixture scope required".into());
        }
        let issuer = ucan::Ed25519InMemorySigner::from_seed_bytes([117; 32]);
        let holder = ucan::Ed25519InMemorySigner::from_seed_bytes([119; 32]);
        let audience = ucan::AudienceDid::from(ucan::TokenSigner::issuer(&holder).map_err(|e| e.to_string())?);
        let issuer_id = ucan::TokenSigner::issuer(&issuer).map_err(|e| e.to_string())?.as_str().to_owned();
        let keys = ucan::KeyResolutionContext::new(vec![
            issuer.verification_key().map_err(|e| e.to_string())?,
            holder.verification_key().map_err(|e| e.to_string())?,
        ]);
        let resource = format!("molten://spacewasm/{}/{}", binding.actor, binding.generation);
        let grants = ["assert", "send"]
            .iter()
            .map(|ability| {
                ucan::CapabilityDocument::new(resource.clone(), (*ability).to_owned()).map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let capabilities = ucan::CapabilitySet::new(grants).map_err(|e| e.to_string())?;
        let token = ucan::issue_token_with_signer(
            &issuer,
            &audience,
            &capabilities,
            &ucan::ProofReferences::empty(),
            ucan::TokenTimeBounds::new(100, 200).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let identity = crate::codec::record("spacewasm-isolated-fixture-authority-v1", vec![
            crate::codec::string(&issuer_id),
            crate::codec::string(audience.as_str()),
            crate::codec::string(POLICY),
            crate::codec::string(&resource),
            crate::codec::string("not-production-authority"),
        ]);
        binding.authority = crate::codec::canonical_hash(&identity).map_err(|e| e.to_string())?;
        Ok(Self {
            current: cell::RefCell::new(binding),
            holder,
            audience,
            issuer: issuer_id,
            token: token.as_str().to_owned(),
            keys,
            nonce: cell::Cell::new(0),
            replay: Replay(cell::RefCell::new(collections::BTreeSet::new())),
        })
    }

    pub fn advance_generation(&self) {
        self.current.borrow_mut().generation += 1;
    }

    pub fn with_admission<T>(&self, operation: impl FnOnce(admission::Authority<'_>) -> T) -> Result<T, String> {
        let current = self.current.borrow();
        let context = admission::Current {
            actor: &current.actor,
            generation: current.generation,
            authority_ref: &current.authority,
            resource_ref: &current.resource,
            expected_issuer: &self.issuer,
            expected_holder: self.audience.as_str(),
        };
        let credentials = [
            admission::Credential {
                hostcall: "assert",
                token: &self.token,
                policy_json: POLICY,
            },
            admission::Credential {
                hostcall: "send",
                token: &self.token,
                policy_json: POLICY,
            },
        ];
        let grant = admission::verify_authority(context, &credentials, &authority::AuthoritySource::services(self))?;
        Ok(operation(grant))
    }
}

impl continuation::CurrentAdmission for Authority {
    fn current(&self) -> Result<continuation::Binding, String> {
        Ok(self.current.borrow().clone())
    }
}

impl authority::AuthoritySource for Authority {
    fn credential(&self, name: &str, operation: &ucan::OperationBinding) -> Result<authority::Credential, String> {
        let sequence = self.nonce.get().checked_add(1).ok_or("fixture nonce bound")?;
        self.nonce.set(sequence);
        let nonce = ucan::HolderInvocationNonce::from_bytes([sequence; 32]);
        let current = self.current.borrow();
        let resource = format!("molten://spacewasm/{}/{}", current.actor, current.generation);
        let request =
            ucan::HolderInvocationSigningRequest::new(&self.token, &self.audience, &resource, name, *operation, nonce);
        let signature = ucan::sign_holder_invocation(
            &request,
            &self.holder,
            ucan::VerificationLimits::default(),
            ucan::HolderInvocationLimits::default(),
        )
        .map_err(|e| e.to_string())?;
        Ok(authority::Credential {
            token: self.token.clone(),
            policy_json: POLICY.into(),
            nonce,
            signature,
        })
    }
    fn services(&self) -> basalt::ucan_enforcement::UcanEnforcementBoundaries<'_> {
        basalt::ucan_enforcement::UcanEnforcementBoundaries {
            verification_time: ucan::VerificationTime::from_unix_seconds(150),
            key_resolver: &self.keys,
            proof_store: &PROOFS,
            revocations: &ucan::NoRevocations,
            caveat_policies: &Caveats,
            replay: &self.replay,
            verification_limits: ucan::VerificationLimits::default(),
            holder_limits: ucan::HolderInvocationLimits::default(),
        }
    }
    fn trusted_issuer(&self) -> &str {
        &self.issuer
    }
    fn expected_holder(&self) -> &str {
        self.audience.as_str()
    }
}
