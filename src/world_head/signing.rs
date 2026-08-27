use std::str::FromStr;

use artifact_auth_core::ArtifactStatement;
use molten_core::fabric_crypto_identity::KeyPurpose;
use molten_core::world_head::WorldHeadPolicyRef;
use molten_core::world_head::WorldHeadSignerRole;
use molten_node_host::node_state::NodeStateNamespace;

use super::WorldHeadPortError;
use super::WorldHeadSignatureCarrier;
use super::WorldHeadSignerIdentity;
use super::WorldHeadSigningPort;
use crate::fabric_crypto_identity::IrohEd25519FileAdapter;
use crate::fabric_crypto_identity::canonical_crypto_profile;
use crate::fabric_crypto_identity::production_ed25519_profile;

pub struct LocalWorldHeadSigningAdapter<'a> {
    adapter: IrohEd25519FileAdapter<'a>,
    producer_id: String,
    allow_generation: bool,
}

impl<'a> LocalWorldHeadSigningAdapter<'a> {
    pub fn new(
        namespace: &'a NodeStateNamespace,
        profile_ref: String,
        entropy_profile_ref: String,
        backend_ref: String,
        producer_id: String,
        allow_generation: bool,
    ) -> crate::error::Result<Self> {
        let profile = canonical_crypto_profile(&production_ed25519_profile(profile_ref, entropy_profile_ref))?;
        let adapter = IrohEd25519FileAdapter::new(namespace, profile, backend_ref)?;
        Ok(Self {
            adapter,
            producer_id,
            allow_generation,
        })
    }

    fn resolve(
        &self,
        policy_ref: &WorldHeadPolicyRef,
    ) -> Result<crate::fabric_crypto_identity::ResolvedProductionKey, WorldHeadPortError> {
        self.adapter
            .resolve_or_generate(KeyPurpose::Authority, policy_ref.as_str(), self.allow_generation)
            .map_err(signing_error)
    }
}

impl WorldHeadSigningPort for LocalWorldHeadSigningAdapter<'_> {
    fn signer_identity(
        &mut self,
        role: WorldHeadSignerRole,
        policy_ref: &WorldHeadPolicyRef,
    ) -> Result<WorldHeadSignerIdentity, WorldHeadPortError> {
        let resolved = self.resolve(policy_ref)?;
        let public_key = iroh::PublicKey::from_str(&resolved.public_key)
            .map_err(|_| WorldHeadPortError::new("world-head-public-key", "public key is malformed"))?;
        Ok(WorldHeadSignerIdentity {
            producer_id: self.producer_id.clone(),
            key_id: format!("world-head-{}-authority-key", role.as_str()),
            key_identity: artifact_auth_ed25519::public_key_identity(public_key.as_bytes()),
        })
    }

    fn sign_statement(
        &mut self,
        statement: &ArtifactStatement,
        role: WorldHeadSignerRole,
        policy_ref: &WorldHeadPolicyRef,
    ) -> Result<WorldHeadSignatureCarrier, WorldHeadPortError> {
        let resolved = self.resolve(policy_ref)?;
        let signed = self
            .adapter
            .sign_artifact_auth_statement(&resolved.handle.handle, statement, policy_ref.as_str())
            .map_err(signing_error)?;
        let public_key = iroh::PublicKey::from_str(&signed.public_key)
            .map_err(|_| WorldHeadPortError::new("world-head-public-key", "public key is malformed"))?;
        Ok(WorldHeadSignatureCarrier {
            producer_id: statement.producer_id.clone(),
            key_id: statement.key_id.clone(),
            public_key_bytes: public_key.as_bytes().to_vec(),
            signature_bytes: signed.signature_bytes,
            key_generation: resolved.handle.handle.generation,
            role,
            authority_admitted: false,
        })
    }
}

fn signing_error(error: crate::error::MoltenError) -> WorldHeadPortError {
    WorldHeadPortError::new("world-head-signing", error.to_string())
}
