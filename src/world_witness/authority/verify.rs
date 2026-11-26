pub(super) fn authenticate(
    claim: &crate::world_head::CanonicalWorldHeadClaim,
    trust: &super::wire::Trust,
    credential: &super::wire::Credential,
) -> crate::error::Result<crate::world_head::WorldHeadAuthenticationResult> {
    let role = match trust.role.as_str() {
        "maintainer" => molten_core::world_head::WorldHeadSignerRole::Maintainer,
        "recovery" => molten_core::world_head::WorldHeadSignerRole::Recovery,
        _ => return Err(super::denied()),
    };
    let currentness_ref = artifact_auth_core::ArtifactRef {
        profile: "molten-world-head-operator-trust-v1".into(),
        algorithm: artifact_auth_core::ALGORITHM_BLAKE3.into(),
        digest_hex: crate::preserves_rail::content_ref_hex(&trust.identity()?)?.to_owned(),
    };
    let policy = artifact_auth_core::AuthenticationPolicy {
        schema: artifact_auth_core::POLICY_SCHEMA_V1.into(),
        profile_id: molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PROFILE.into(),
        threshold: 1,
        trusted_keys: vec![artifact_auth_core::TrustedKeyObservation {
            producer_id: trust.producer_id.clone(),
            key_id: trust.key_id.clone(),
            key_identity: artifact_auth_ed25519::public_key_identity(&trust.artifact_public_key),
            allowed_purposes: vec![molten_core::world_head::WORLD_HEAD_ARTIFACT_AUTH_PURPOSE.into()],
            generation: trust.key_generation,
            currentness: artifact_auth_core::KeyCurrentness::Current,
            currentness_ref,
        }],
    };
    let carrier = crate::world_head::WorldHeadSignatureCarrier {
        producer_id: trust.producer_id.clone(),
        key_id: trust.key_id.clone(),
        public_key_bytes: trust.artifact_public_key.to_vec(),
        signature_bytes: credential.artifact_signature.clone(),
        key_generation: trust.key_generation,
    };
    let mut operator = OperatorSigner { trust, role };
    let result = crate::world_head::evaluate_world_head_authentication(claim, &policy, &[carrier], &mut operator)?;
    // The signer must hold the trust record's role with port-admitted authority; a valid signature
    // by a key whose trusted key id names another role (or none) is not operator authority.
    let is_signer_admitted = result.observation.signers.len() == 1
        && result.observation.signers.iter().all(|signer| {
            signer.role == Some(role)
                && signer.authenticated
                && signer.current
                && !signer.revoked
                && signer.authority_admitted
        });
    if !result.observation.passed
        || !result.observation.purpose_matches
        || !result.observation.policy_matches
        || !is_signer_admitted
    {
        return Err(super::denied());
    }
    Ok(result)
}

/// Signer authority from the current operator trust record, consulted only after the invocation
/// credential has been authorized: the trusted key holds authority solely in its declared role.
struct OperatorSigner<'a> {
    trust: &'a super::wire::Trust,
    role: molten_core::world_head::WorldHeadSignerRole,
}

impl crate::world_head::WorldHeadSignerAuthorityPort for OperatorSigner<'_> {
    fn signer_authority(
        &mut self,
        _claim: &molten_core::world_head::WorldHeadClaim,
        signer: &crate::world_head::WorldHeadVerifiedSigner<'_>,
    ) -> Result<bool, crate::world_head::WorldHeadPortError> {
        let key_identity = artifact_auth_ed25519::public_key_identity(&self.trust.artifact_public_key);
        Ok(signer.role == self.role
            && signer.producer_id == self.trust.producer_id
            && signer.key_id == self.trust.key_id
            && signer.key_identity_ref == format!("blake3:{}", key_identity.digest_hex))
    }
}

#[cfg(test)]
mod tests {
    const KEY_WIDTH: usize = 32;

    fn reference(label: &str) -> String {
        format!("blake3:{}", blake3::hash(label.as_bytes()).to_hex())
    }

    fn claim() -> molten_core::world_head::WorldHeadClaim {
        molten_core::world_head::WorldHeadClaim {
            branch_id: molten_core::world_head::WorldBranchId::new("main").expect("branch"),
            branch_class: molten_core::world_head::WorldBranchClass::Local,
            expected_head: None,
            successor_head: molten_core::world_commit::WorldCommitRef::new(reference("root")).expect("commit ref"),
            expected_generation: 0,
            successor_generation: 1,
            purpose: molten_core::world_head::WorldHeadPurpose::Create,
            policy_ref: molten_core::world_head::WorldHeadPolicyRef::new(reference("head-policy")).expect("policy ref"),
            source_heads: Vec::new(),
        }
    }

    fn trust(carrier: &crate::world_head::WorldHeadSignatureCarrier, role: &str) -> super::super::wire::Trust {
        super::super::wire::Trust {
            issuer: "did:key:issuer".into(),
            issuer_key: [1; KEY_WIDTH],
            holder: "did:key:holder".into(),
            holder_key: [2; KEY_WIDTH],
            policy: "{}".into(),
            head_policy_ref: reference("head-policy"),
            witness_policy_ref: reference("witness-policy"),
            not_before: 0,
            expires: u64::MAX,
            authority_generation: 1,
            revoked: Vec::new(),
            producer_id: carrier.producer_id.clone(),
            key_id: carrier.key_id.clone(),
            artifact_public_key: carrier.public_key_bytes.as_slice().try_into().expect("ed25519 key width"),
            key_generation: carrier.key_generation,
            role: role.into(),
        }
    }

    fn credential(carrier: &crate::world_head::WorldHeadSignatureCarrier) -> super::super::wire::Credential {
        super::super::wire::Credential {
            token: String::new(),
            nonce: [0; KEY_WIDTH],
            signature: Vec::new(),
            proofs: Vec::new(),
            artifact_signature: carrier.signature_bytes.clone(),
        }
    }

    #[test]
    fn operator_signer_must_hold_the_trusted_role() {
        // r[verify molten.world_commit.world_head_truthful_receipts_and_verified_signers]
        let temporary = cap_tempfile::tempdir(cap_std::ambient_authority()).expect("temporary state root");
        let root =
            molten_node_host::node_state::DirectoryRoot::from_dir(temporary.try_clone().expect("clone temporary root"));
        root.create_layout().expect("node state layout");
        let secrets = root.secrets().expect("secrets namespace");
        let mut signer =
            crate::world_head::LocalWorldHeadSigningAdapter::new(&secrets, crate::world_head::SignerIdentity {
                profile_ref: reference("crypto-profile"),
                entropy_profile_ref: reference("entropy-profile"),
                backend_ref: reference("crypto-backend"),
                producer_id: "molten".to_string(),
                allow_generation: true,
            })
            .expect("signing adapter");
        let claim = claim();
        let (canonical, maintainer, _) = crate::world_head::sign_world_head_claim(
            &mut signer,
            &claim,
            molten_core::world_head::WorldHeadSignerRole::Maintainer,
        )
        .expect("maintainer signature");
        let (_, recovery, _) = crate::world_head::sign_world_head_claim(
            &mut signer,
            &claim,
            molten_core::world_head::WorldHeadSignerRole::Recovery,
        )
        .expect("recovery signature");

        let admitted = super::authenticate(&canonical, &trust(&maintainer, "maintainer"), &credential(&maintainer))
            .expect("maintainer key under maintainer trust");
        assert_eq!(
            admitted.observation.signers[0].role,
            Some(molten_core::world_head::WorldHeadSignerRole::Maintainer)
        );
        assert!(admitted.observation.signers[0].authority_admitted);

        // Valid signatures whose trusted key id names a different role than the trust record.
        assert!(super::authenticate(&canonical, &trust(&maintainer, "recovery"), &credential(&maintainer)).is_err());
        assert!(super::authenticate(&canonical, &trust(&recovery, "maintainer"), &credential(&recovery)).is_err());
    }
}
