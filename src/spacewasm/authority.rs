use std::rc;

use basalt::ucan;
use basalt::ucan_enforcement;

use super::admission;
use super::continuation;
use super::host;

pub struct Credential {
    pub token: String,
    pub policy_json: String,
    pub nonce: ucan::HolderInvocationNonce,
    pub signature: ucan::SignerSignature,
}

pub trait AuthoritySource: continuation::CurrentAdmission {
    fn credential(&self, name: &str, operation: &ucan::OperationBinding) -> Result<Credential, String>;
    fn services(&self) -> ucan_enforcement::UcanEnforcementBoundaries<'_>;
    fn trusted_issuer(&self) -> &str;
    fn expected_holder(&self) -> &str;
}

pub trait EffectPort {
    fn perform(&self, name: &str, bytes: &[u8]) -> Result<host::Reply, continuation::Failure>;
}

pub struct AuthorizedHost {
    pub authority: rc::Rc<dyn AuthoritySource>,
    pub effects: rc::Rc<dyn EffectPort>,
}

struct ExactProfile(ucan::OperationProfileId);
impl ucan::OperationProfileAdmission for ExactProfile {
    fn admit_operation_profile(
        &self,
        profile: &ucan::OperationProfileId,
    ) -> Result<(), ucan::OperationProfileAdmissionError> {
        if *profile == self.0 {
            Ok(())
        } else {
            Err(ucan::OperationProfileAdmissionError::UnknownProfile)
        }
    }
}

fn operation(binding: &continuation::Binding, name: &str, bytes: &[u8]) -> Result<ucan::OperationBinding, String> {
    let profile = crate::codec::record("spacewasm-host-operation-profile-v1", vec![
        crate::codec::string(super::profile::REVISION),
        crate::codec::string(super::profile::ABI),
    ]);
    let request = crate::codec::record("spacewasm-host-operation-v1", vec![
        crate::codec::string(binding.identity()?),
        crate::codec::string(name),
        crate::codec::string(crate::codec::content_ref_from_bytes(bytes)),
    ]);
    let profile = crate::codec::canonical_bytes(&profile).map_err(|e| e.to_string())?;
    let request = crate::codec::canonical_bytes(&request).map_err(|e| e.to_string())?;
    Ok(ucan::OperationBinding::new(
        ucan::OperationProfileId::from_bytes(*blake3::hash(&profile).as_bytes()),
        ucan::RequestBindingDigest::from_bytes(*blake3::hash(&request).as_bytes()),
    ))
}

// r[impl molten.spacewasm_core.admission]
impl AuthorizedHost {
    fn authorize(&self, binding: &continuation::Binding, name: &str, bytes: &[u8]) -> Result<(), String> {
        if self.authority.current()? != *binding {
            return Err("stale host authority".into());
        }
        let operation = operation(binding, name, bytes)?;
        let credential = self.authority.credential(name, &operation)?;
        let services = self.authority.services();
        let current = admission::Current {
            actor: &binding.actor,
            generation: binding.generation,
            authority_ref: &binding.authority,
            resource_ref: &binding.resource,
            expected_issuer: self.authority.trusted_issuer(),
            expected_holder: self.authority.expected_holder(),
        };
        let grants = [admission::Credential {
            hostcall: name,
            token: &credential.token,
            policy_json: &credential.policy_json,
        }];
        admission::verify_authority(current, &grants, &services)?;
        let resource = format!("molten://spacewasm/{}/{}", binding.actor, binding.generation);
        let profiles = ExactProfile(operation.profile());
        let invocation = ucan::HolderInvocationRequest::new(
            &credential.token,
            &resource,
            name,
            operation,
            credential.nonce,
            &credential.signature,
        );
        let tokens = ucan::HolderTokenVerificationContext::new(
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
        ucan::verify_holder_signed_invocation(&invocation, &tokens, &policy).map_err(|_| "holder invocation denied")?;
        if self.authority.current()? != *binding {
            return Err("authority changed during verification".into());
        }
        Ok(())
    }
}

impl host::HostPort for AuthorizedHost {
    fn dispatch(
        &self,
        binding: &continuation::Binding,
        name: &str,
        bytes: &[u8],
    ) -> Result<host::Reply, continuation::Failure> {
        self.authorize(binding, name, bytes).map_err(continuation::Failure::Denied)?;
        self.effects.perform(name, bytes)
    }
}
