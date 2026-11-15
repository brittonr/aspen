//! PCR-policy-only TPM2 transport unseal. No provisioning, secret publication or fallback.
use super::*;

mod process;

// r[impl molten.crypto_identity.tpm2_sealed_transport]
pub struct Sealed<'a> {
    namespace: &'a crate::node_state::Scope,
    executable: &'a crate::node_state::Executable,
    workspace: crate::node_state::Workspace,
    profile: Profile,
    policy: Tpm2SealedPolicy,
    backend_ref: String,
}

impl<'a> Sealed<'a> {
    pub fn new(
        namespace: &'a crate::node_state::Scope,
        executable: &'a crate::node_state::Executable,
        profile: Profile,
        policy: Tpm2SealedPolicy,
    ) -> Result<Self, Tpm2IdentityFailure> {
        policy.validate()?;
        admit_profile_for_production(&profile.profile).map_err(|_| Tpm2IdentityFailure::ProfileDenied)?;
        if !profile.profile.backend_classes.contains(&KeyBackendClass::Tpm2Sealed)
            || !profile.profile.allowed_purposes.contains(&KeyPurpose::TransportEndpoint)
            || !matches!(
                namespace.kind(),
                crate::node_state::NamespaceKind::Identity | crate::node_state::NamespaceKind::Secrets
            )
        {
            return Err(Tpm2IdentityFailure::ProfileDenied);
        }
        let backend_ref = backend_ref(&policy)?;
        let workspace = namespace.process_workspace().map_err(|_| Tpm2IdentityFailure::CapabilityUnavailable)?;
        let adapter = Self {
            namespace,
            executable,
            workspace,
            profile,
            policy,
            backend_ref,
        };
        adapter.require_tool_identity()?;
        Ok(adapter)
    }

    pub fn backend_ref(&self) -> &str {
        &self.backend_ref
    }

    /// Resolution exposes only a public key and a purpose/generation-scoped handle.
    pub fn resolve(&self) -> Result<(KeyHandle, String), Tpm2IdentityFailure> {
        let (generation, secret) = self.unseal()?;
        let public = secret.public();
        Ok((self.handle(generation, public.as_bytes())?, public.to_string()))
    }

    /// Pass directly to Iroh in memory. No file adapter or replacement generation is consulted.
    pub fn load_transport_secret(&self, expected: &OpaqueKeyHandle) -> Result<iroh::SecretKey, Tpm2IdentityFailure> {
        if expected.backend_class != KeyBackendClass::Tpm2Sealed
            || expected.backend_ref != self.backend_ref
            || expected.purpose != KeyPurpose::TransportEndpoint
        {
            return Err(Tpm2IdentityFailure::HandleMismatch);
        }
        let (generation, secret) = self.unseal()?;
        let actual = self.handle(generation, secret.public().as_bytes())?;
        if &actual.handle != expected {
            return Err(Tpm2IdentityFailure::HandleMismatch);
        }
        Ok(secret)
    }

    // r[impl molten.crypto_identity.tpm2_backend_rotation]
    /// Check actual unsealed destination state against an already admitted rotation plan.
    /// Caller owns fenced activation and old-key revocation; this returns transition evidence.
    pub fn complete_rotation(
        &self,
        plan: &KeyRotationPlan,
    ) -> Result<CompletedProductionRotation, Tpm2IdentityFailure> {
        let (handle, _) = self.resolve()?;
        let outcome = complete_key_rotation(plan, &handle.handle).map_err(|_| Tpm2IdentityFailure::RotationDenied)?;
        Ok(CompletedProductionRotation { handle, outcome })
    }

    fn unseal(&self) -> Result<(u64, iroh::SecretKey), Tpm2IdentityFailure> {
        super::file_adapter::require_not_revoked(self.namespace, KeyPurpose::TransportEndpoint)
            .map_err(|_| Tpm2IdentityFailure::HandleMismatch)?;
        self.require_object()?;
        let mut bytes = self.run_tool(
            vec![
                "unseal".to_string(),
                "-c".to_string(),
                format!("0x{:08x}", self.policy.persistent_handle),
                "-p".to_string(),
                format!("pcr:{}", self.policy.pcr_selection()),
            ],
            48,
        )?;
        let decoded =
            super::file_adapter::decode_transport_record(&bytes).map_err(|_| Tpm2IdentityFailure::MalformedRecord);
        bytes.fill(0);
        // Detect concurrent persistent-handle replacement before releasing material.
        self.require_object()?;
        decoded
    }

    fn require_object(&self) -> Result<(), Tpm2IdentityFailure> {
        let bytes = self.run_tool(
            vec![
                "readpublic".to_string(),
                "-c".to_string(),
                format!("0x{:08x}", self.policy.persistent_handle),
                "-f".to_string(),
                "tss".to_string(),
                "-o".to_string(),
                "/dev/stdout".to_string(),
                "-Q".to_string(),
            ],
            80,
        )?;
        if bytes != self.policy.public_object {
            return Err(Tpm2IdentityFailure::ObjectMismatch);
        }
        Ok(())
    }

    fn handle(&self, generation: u64, public: &[u8]) -> Result<KeyHandle, Tpm2IdentityFailure> {
        let public_key_ref = crate::preserves_rail::content_ref_from_bytes(public);
        let currentness =
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten.tpm2-currentness.v1", vec![
                crate::preserves_rail::string(&self.profile.profile.profile_ref),
                crate::preserves_rail::string(&self.backend_ref),
                crate::preserves_rail::string(&public_key_ref),
                crate::preserves_rail::u64_value(generation),
            ]))
            .map_err(|_| Tpm2IdentityFailure::InvalidPolicy)?;
        key_handle(KeyHandleInput {
            profile: &self.profile,
            purpose: KeyPurpose::TransportEndpoint,
            generation,
            public_key_ref: &public_key_ref,
            backend_class: KeyBackendClass::Tpm2Sealed,
            backend_ref: &self.backend_ref,
            currentness: KeyCurrentness::Current,
            currentness_evidence_ref: &currentness,
        })
        .map_err(|_| Tpm2IdentityFailure::ProfileDenied)
    }
}

pub fn backend_ref(policy: &Tpm2SealedPolicy) -> Result<String, Tpm2IdentityFailure> {
    policy.validate()?;
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("molten.tpm2-sealed-backend.v1", vec![
        crate::preserves_rail::string(policy.pcr_selection()),
        crate::preserves_rail::string(crate::preserves_rail::content_ref_from_bytes(&policy.public_object)),
        crate::preserves_rail::u64_value(u64::from(policy.persistent_handle)),
        crate::preserves_rail::string(policy.provider_class.as_str()),
        crate::preserves_rail::string(&policy.provider_ref),
        crate::preserves_rail::string(&policy.executable_ref),
    ]))
    .map_err(|_| Tpm2IdentityFailure::InvalidPolicy)
}

#[cfg(test)]
mod tests;
