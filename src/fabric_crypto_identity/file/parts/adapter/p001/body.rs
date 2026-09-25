
impl<'a> IrohEd25519FileAdapter<'a> {
    pub fn new(
        namespace: &'a crate::node_state::NodeStateNamespace,
        profile: CanonicalCryptoProfile,
        backend_ref: String,
    ) -> crate::error::Result<Self> {
        admit_profile_for_production(&profile.profile)
            .map_err(|issues| validation_error("production crypto profile", &issues))?;
        if !profile.profile.backend_classes.contains(&KeyBackendClass::CapabilityFile) {
            return Err(crate::error::MoltenError::invalid_harness(
                "production crypto profile does not admit capability-file keys",
            ));
        }
        require_blake3_ref("crypto file backend", &backend_ref)?;
        match namespace.kind() {
            crate::node_state::NodeStateNamespaceKind::Identity
            | crate::node_state::NodeStateNamespaceKind::Secrets => {}
            other => {
                return Err(crate::error::MoltenError::invalid_harness(format!(
                    "crypto file adapter requires identity or secrets namespace, got {other:?}"
                )));
            }
        }
        Ok(Self {
            namespace,
            profile,
            backend_ref,
        })
    }

    pub fn profile(&self) -> &CanonicalCryptoProfile {
        &self.profile
    }

    // r[impl molten.crypto_identity.production_key_lifecycle]
    pub fn resolve_or_generate(
        &self,
        purpose: KeyPurpose,
        policy_ref: &str,
        permit_first_boot_generation: bool,
    ) -> crate::error::Result<ResolvedProductionKey> {
        require_blake3_ref("key resolution policy", policy_ref)?;
        require_not_revoked(self.namespace, purpose)?;
        let path = key_path(purpose)?;
        match self.namespace.observe_file(&path)? {
            crate::node_state::NodeStateFileObservation::Missing => {
                if !permit_first_boot_generation {
                    return Err(crate::error::MoltenError::invalid_harness(
                        "required production key is unavailable and replacement generation is disabled",
                    ));
                }
                self.generate_first_key(purpose, policy_ref, &path)
            }
            crate::node_state::NodeStateFileObservation::NonRegular(kind) => {
                Err(crate::error::MoltenError::invalid_harness(format!(
                    "production key leaf must be a regular file, got {kind:?}"
                )))
            }
            crate::node_state::NodeStateFileObservation::Regular(file) => {
                let permission_status = permission_status(&file);
                require_restricted_permission(permission_status)?;
                let bytes = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
                let record = decode_key_record(&bytes)?;
                self.resolved_key(purpose, &record, false, permission_status)
            }
        }
    }

    fn generate_first_key(
        &self,
        purpose: KeyPurpose,
        policy_ref: &str,
        path: &crate::node_state::NodeStatePath,
    ) -> crate::error::Result<ResolvedProductionKey> {
        let entropy_profile_ref = self.profile.profile.entropy_profile_ref.clone().ok_or_else(|| {
            crate::error::MoltenError::invalid_harness("production crypto profile has no entropy profile")
        })?;
        let request = KeyGenerationRequest {
            operation_id: format!("generate-{}", purpose.as_str()),
            profile_ref: self.profile.profile.profile_ref.clone(),
            purpose,
            backend_class: KeyBackendClass::CapabilityFile,
            backend_ref: self.backend_ref.clone(),
            entropy_profile_ref,
            generation: FIRST_KEY_GENERATION,
            policy_ref: policy_ref.to_string(),
            permit_first_boot_generation: true,
        };
        admit_key_generation(&self.profile.profile, &request)
            .map_err(|issues| validation_error("production key generation", &issues))?;
        let record = KeyRecord {
            generation: FIRST_KEY_GENERATION,
            secret_key: iroh::SecretKey::generate(),
        };
        let encoded = encode_key_record(&record);
        self.namespace.write_restricted(path, &encoded, OWNER_ONLY_SECRET_FILE_MODE)?;
        self.resolved_key(purpose, &record, true, KeyPermissionStatus::Restricted)
    }

    fn resolved_key(
        &self,
        purpose: KeyPurpose,
        record: &KeyRecord,
        generated: bool,
        permission_status: KeyPermissionStatus,
    ) -> crate::error::Result<ResolvedProductionKey> {
        let public_key = record.secret_key.public();
        let public_key_ref = crate::preserves_rail::content_ref_from_bytes(public_key.as_bytes());
        let currentness_evidence_ref = currentness_evidence_ref(
            &self.profile.profile.profile_ref,
            purpose,
            record.generation,
            &public_key_ref,
            &self.backend_ref,
        )?;
        let handle = canonical_key_handle(KeyHandleInput {
            profile: &self.profile,
            purpose,
            generation: record.generation,
            public_key_ref: &public_key_ref,
            backend_class: KeyBackendClass::CapabilityFile,
            backend_ref: &self.backend_ref,
            currentness: KeyCurrentness::Current,
            currentness_evidence_ref: &currentness_evidence_ref,
        })?;
        Ok(ResolvedProductionKey {
            handle,
            public_key: public_key.to_string(),
            generated,
            permission_status,
        })
    }

    // r[impl molten.crypto_identity.canonical_signature_binding]
    pub fn sign(
        &self,
        requested_handle: &OpaqueKeyHandle,
        domain: &CanonicalSignatureDomain,
        policy_ref: &str,
    ) -> crate::error::Result<CanonicalSignatureOutcome> {
        require_canonical_domain(&self.profile, domain)?;
        let current = self.resolve_or_generate(requested_handle.purpose, policy_ref, false)?;
        let request = SignRequest {
            operation_id: format!("sign-{}", requested_handle.purpose.as_str()),
            profile_ref: self.profile.profile.profile_ref.clone(),
            handle: requested_handle.clone(),
            domain: domain.domain.clone(),
            current_generation: current.handle.handle.generation,
            current_handle_ref: current.handle.handle.handle_ref.clone(),
            policy_ref: policy_ref.to_string(),
        };
        let plan = plan_sign(&self.profile.profile, &request)
            .map_err(|issues| validation_error("production signing", &issues))?;
        if domain.domain_ref != crate::preserves_rail::canonical_hash(&domain.value)? {
            return Err(crate::error::MoltenError::invalid_harness(
                "signature domain ref does not match canonical value",
            ));
        }
        let record = self.load_current_record(requested_handle.purpose)?;
        let signature = record.secret_key.sign(&domain.bytes).to_bytes().to_vec();
        canonical_signature_outcome(&self.profile, &plan, domain, signature)
    }

    // r[impl molten.artifact_auth_shell.exact_verification]
    pub(crate) fn sign_artifact_auth_statement(
        &self,
        requested_handle: &OpaqueKeyHandle,
        statement: &artifact_auth_core::ArtifactStatement,
        policy_ref: &str,
    ) -> crate::error::Result<ExactArtifactAuthSignature> {
        require_blake3_ref("artifact-auth signing policy", policy_ref)?;
        let current = self.resolve_or_generate(requested_handle.purpose, policy_ref, false)?;
        require_current_handle(requested_handle, &current.handle.handle)?;
        if statement.scope.profile_id != self.profile.profile.profile_id {
            return Err(crate::error::MoltenError::invalid_harness(
                "artifact-auth statement profile does not match the key profile",
            ));
        }
        if statement.scope.purpose != requested_handle.purpose.as_str() {
            return Err(crate::error::MoltenError::invalid_harness(
                "artifact-auth statement purpose does not match the key purpose",
            ));
        }
        let record = self.load_current_record(requested_handle.purpose)?;
        let public_key = record.secret_key.public();
        let key_identity = artifact_auth_ed25519::public_key_identity(public_key.as_bytes());
        if statement.key_identity != key_identity {
            return Err(crate::error::MoltenError::invalid_harness(
                "artifact-auth statement full-key identity does not match the current key",
            ));
        }
        let statement_bytes = artifact_auth_core::canonical_statement_bytes(statement).map_err(|_| {
            crate::error::MoltenError::invalid_harness("artifact-auth statement is not canonicalizable")
        })?;
        let signature_bytes = record.secret_key.sign(&statement_bytes).to_bytes().to_vec();
        debug_assert_eq!(public_key.as_bytes().len(), artifact_auth_ed25519::ED25519_PUBLIC_KEY_BYTES);
        debug_assert_eq!(signature_bytes.len(), artifact_auth_ed25519::ED25519_SIGNATURE_BYTES);
        Ok(ExactArtifactAuthSignature {
            public_key: public_key.to_string(),
            signature_bytes,
        })
    }

    // r[impl molten.crypto_identity.canonical_signature_binding]
    pub fn verify(
        &self,
        public_key: &str,
        input: VerificationInput<'_>,
    ) -> crate::error::Result<CanonicalVerificationOutcome> {
        let VerificationInput {
            expected_domain,
            signature,
            signer_currentness,
            signer_generation,
            policy_ref,
        } = input;
        require_blake3_ref("verification policy", policy_ref)?;
        require_canonical_domain(&self.profile, expected_domain)?;
        require_canonical_signature(signature)?;
        let public_key = iroh::PublicKey::from_str(public_key)
            .map_err(|_| crate::error::MoltenError::invalid_harness("verification public key is malformed"))?;
        let public_key_ref = crate::preserves_rail::content_ref_from_bytes(public_key.as_bytes());
        let parsed_signature = iroh::Signature::try_from(signature.signature.as_slice()).ok();
        let is_crypto_passed = parsed_signature
            .as_ref()
            .is_some_and(|parsed| public_key.verify(&expected_domain.bytes, parsed).is_ok());
        let mut observed = signature.metadata.clone();
        if observed.signer_public_ref != public_key_ref {
            observed.signer_public_ref = public_key_ref;
        }
        let request = VerificationRequest {
            operation_id: format!("verify-{}", expected_domain.domain.purpose.as_str()),
            profile_ref: self.profile.profile.profile_ref.clone(),
            expected_domain: expected_domain.domain.clone(),
            observed,
            cryptographic_verification_passed: is_crypto_passed,
            signer_currentness,
            signer_generation,
            policy_ref: policy_ref.to_string(),
        };
        canonical_verification_outcome(evaluate_verification(&self.profile.profile, &request))
    }

    // r[impl molten.crypto_identity.rotation_revocation]
    pub fn revoke(
        &self,
        handle: &OpaqueKeyHandle,
        revocation_evidence_ref: &str,
        policy_ref: &str,
    ) -> crate::error::Result<CryptoStatusReadback> {
        require_blake3_ref("revocation evidence", revocation_evidence_ref)?;
        require_blake3_ref("revocation policy", policy_ref)?;
        let current = self.resolve_or_generate(handle.purpose, policy_ref, false)?;
        require_current_handle(handle, &current.handle.handle)?;
        let marker = crate::preserves_rail::record("fabric-crypto-key-revocation-v1", vec![
            crate::preserves_rail::string(&handle.handle_ref),
            crate::preserves_rail::string(&handle.public_key_ref),
            crate::preserves_rail::u64_value(handle.generation),
            crate::preserves_rail::string(revocation_evidence_ref),
            crate::preserves_rail::string(policy_ref),
        ]);
        self.namespace.write_restricted(
            &revocation_path(handle.purpose)?,
            &crate::preserves_rail::canonical_bytes(&marker)?,
            OWNER_ONLY_SECRET_FILE_MODE,
        )?;
        self.status_from_record(handle.purpose, KeyCurrentness::Revoked, vec![revocation_evidence_ref.to_string()])
    }
}
