
impl<'a> IrohEd25519FileAdapter<'a> {
    pub fn rotate(&self, request: &KeyRotationRequest) -> crate::error::Result<CompletedProductionRotation> {
        if request.overlap != RotationOverlapPolicy::None {
            return Err(crate::error::MoltenError::invalid_harness(
                "capability-file crypto adapter supports no-overlap rotation only",
            ));
        }
        let current = self.resolve_or_generate(request.purpose, &request.policy_ref, false)?;
        let plan = plan_key_rotation(&self.profile.profile, &current.handle.handle, request)
            .map_err(|issues| validation_error("production key rotation", &issues))?;
        let next_record = KeyRecord {
            generation: request.new_generation,
            secret_key: iroh::SecretKey::generate(),
        };
        let next = self.resolved_key(request.purpose, &next_record, true, KeyPermissionStatus::Restricted)?;
        let outcome = complete_key_rotation(&plan, &next.handle.handle)
            .map_err(|issues| validation_error("production key rotation completion", &issues))?;
        self.namespace.write_restricted(
            &key_path(request.purpose)?,
            &encode_key_record(&next_record),
            OWNER_ONLY_SECRET_FILE_MODE,
        )?;
        Ok(CompletedProductionRotation {
            handle: next.handle,
            outcome,
        })
    }

    pub fn redacted_status(
        &self,
        purpose: KeyPurpose,
        receipt_refs: Vec<String>,
    ) -> crate::error::Result<CryptoStatusReadback> {
        let currentness = if is_revoked(self.namespace, purpose)? {
            KeyCurrentness::Revoked
        } else {
            KeyCurrentness::Current
        };
        self.status_from_record(purpose, currentness, receipt_refs)
    }

    fn status_from_record(
        &self,
        purpose: KeyPurpose,
        currentness: KeyCurrentness,
        receipt_refs: Vec<String>,
    ) -> crate::error::Result<CryptoStatusReadback> {
        let path = key_path(purpose)?;
        let crate::node_state::NodeStateFileObservation::Regular(file) = self.namespace.observe_file(&path)? else {
            return Err(crate::error::MoltenError::invalid_harness("production key status is unavailable"));
        };
        let permission_status = permission_status(&file);
        let record = decode_key_record(&file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?)?;
        canonical_crypto_status(&AdapterDiagnosticInput {
            profile_ref: self.profile.profile.profile_ref.clone(),
            purpose,
            generation: Some(record.generation),
            currentness: Some(currentness),
            permission_status: permission_status.canonical(),
            backend_class: KeyBackendClass::CapabilityFile,
            public_key_ref: Some(crate::preserves_rail::content_ref_from_bytes(record.secret_key.public().as_bytes())),
            receipt_refs,
            backend_locator: Some("capability-rooted-key-leaf".to_string()),
            raw_error: None,
            bearer_token: None,
            private_material_present: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn load_transport_secret(&self, handle: &OpaqueKeyHandle) -> crate::error::Result<iroh::SecretKey> {
        if handle.purpose != KeyPurpose::TransportEndpoint {
            return Err(crate::error::MoltenError::invalid_harness(
                "only transport-purpose handles may configure an Iroh endpoint",
            ));
        }
        let current = self.resolve_or_generate(
            KeyPurpose::TransportEndpoint,
            &crate::preserves_rail::content_ref_from_bytes(b"transport-endpoint-load-policy"),
            false,
        )?;
        if current.handle.handle.handle_ref != handle.handle_ref
            || current.handle.handle.generation != handle.generation
        {
            return Err(crate::error::MoltenError::invalid_harness("stale transport key handle denied"));
        }
        Ok(self.load_current_record(KeyPurpose::TransportEndpoint)?.secret_key)
    }

    fn load_current_record(&self, purpose: KeyPurpose) -> crate::error::Result<KeyRecord> {
        let path = key_path(purpose)?;
        let observation = self.namespace.observe_file(&path)?;
        let crate::node_state::NodeStateFileObservation::Regular(file) = observation else {
            return Err(crate::error::MoltenError::invalid_harness("current production key is unavailable"));
        };
        require_restricted_permission(permission_status(&file))?;
        decode_key_record(&file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?)
    }
}

pub fn production_ed25519_profile(profile_ref: String, entropy_profile_ref: String) -> CryptoAdapterProfile {
    CryptoAdapterProfile {
        schema: CRYPTO_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "molten.crypto.ed25519-iroh.v1".to_string(),
        profile_ref,
        class: CryptoProfileClass::Production,
        algorithm: CryptoAlgorithm::Ed25519Iroh,
        backend_classes: vec![KeyBackendClass::CapabilityFile, KeyBackendClass::ManagedSecret],
        allowed_purposes: vec![
            KeyPurpose::TransportEndpoint,
            KeyPurpose::FederationOrigin,
            KeyPurpose::Delegation,
            KeyPurpose::EvidenceSigning,
            KeyPurpose::Authority,
        ],
        entropy_profile_ref: Some(entropy_profile_ref),
        domain_version: "v1".to_string(),
        allow_key_sharing: false,
        sharing_policy_ref: None,
        max_signature_bytes: MAX_SIGNATURE_BYTES,
        non_claims: REQUIRED_CRYPTO_NON_CLAIMS.to_vec(),
    }
}

pub fn fixture_blake3_profile(profile_ref: String) -> CryptoAdapterProfile {
    CryptoAdapterProfile {
        schema: CRYPTO_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "molten.crypto.blake3-fixture.v1".to_string(),
        profile_ref,
        class: CryptoProfileClass::FixtureSimulation,
        algorithm: CryptoAlgorithm::Blake3Fixture,
        backend_classes: vec![KeyBackendClass::InMemoryFixture],
        allowed_purposes: vec![KeyPurpose::FederationOrigin, KeyPurpose::EvidenceSigning],
        entropy_profile_ref: None,
        domain_version: "fixture-v1".to_string(),
        allow_key_sharing: false,
        sharing_policy_ref: None,
        max_signature_bytes: MAX_SIGNATURE_BYTES,
        non_claims: REQUIRED_CRYPTO_NON_CLAIMS.to_vec(),
    }
}

pub(crate) fn transport_key_path() -> crate::error::Result<crate::node_state::NodeStatePath> {
    key_path(KeyPurpose::TransportEndpoint)
}

pub(crate) fn generate_transport_key_record() -> Vec<u8> {
    encode_key_record(&KeyRecord {
        generation: FIRST_KEY_GENERATION,
        secret_key: iroh::SecretKey::generate(),
    })
    .to_vec()
}

pub(crate) fn transport_key_record_from_secret_hex(secret: &str) -> crate::error::Result<Vec<u8>> {
    let secret_bytes = decode_secret_hex(secret)?;
    Ok(encode_key_record(&KeyRecord {
        generation: FIRST_KEY_GENERATION,
        secret_key: iroh::SecretKey::from_bytes(&secret_bytes),
    })
    .to_vec())
}

pub(crate) fn transport_endpoint_material(
    record_bytes: &[u8],
    backend_ref: &str,
) -> crate::error::Result<TransportEndpointKeyMaterial> {
    require_blake3_ref("transport identity backend", backend_ref)?;
    let key_record = decode_key_record(record_bytes)?;
    let public_key = key_record.secret_key.public();
    let public_key_ref = crate::preserves_rail::content_ref_from_bytes(public_key.as_bytes());
    let profile = canonical_crypto_profile(&production_ed25519_profile(
        crate::preserves_rail::content_ref_from_bytes(NODE_TRANSPORT_PROFILE_LABEL),
        crate::preserves_rail::content_ref_from_bytes(NODE_TRANSPORT_ENTROPY_LABEL),
    ))?;
    let currentness_ref = currentness_evidence_ref(
        &profile.profile.profile_ref,
        KeyPurpose::TransportEndpoint,
        key_record.generation,
        &public_key_ref,
        backend_ref,
    )?;
    let handle = canonical_key_handle(KeyHandleInput {
        profile: &profile,
        purpose: KeyPurpose::TransportEndpoint,
        generation: key_record.generation,
        public_key_ref: &public_key_ref,
        backend_class: KeyBackendClass::CapabilityFile,
        backend_ref,
        currentness: KeyCurrentness::Current,
        currentness_evidence_ref: &currentness_ref,
    })?;
    let handle_ref = handle.handle.handle_ref;
    Ok(TransportEndpointKeyMaterial {
        public_key: public_key.to_string(),
        endpoint_id: format!("iroh:{public_key}"),
        handle_ref,
        generation: key_record.generation,
    })
}

pub(crate) fn load_transport_secret_for_identity(
    namespace: &crate::node_state::NodeStateNamespace,
    expected_endpoint_id: &str,
    expected_handle_ref: &str,
    backend_ref: &str,
) -> crate::error::Result<iroh::SecretKey> {
    require_not_revoked(namespace, KeyPurpose::TransportEndpoint)?;
    let observation = namespace.observe_file(&transport_key_path()?)?;
    let crate::node_state::NodeStateFileObservation::Regular(file) = observation else {
        return Err(crate::error::MoltenError::invalid_harness("persisted transport key is unavailable"));
    };
    require_restricted_permission(permission_status(&file))?;
    let bytes = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
    let material = transport_endpoint_material(&bytes, backend_ref)?;
    if material.endpoint_id != expected_endpoint_id || material.handle_ref != expected_handle_ref {
        return Err(crate::error::MoltenError::invalid_harness(
            "persisted transport key does not match admitted node identity",
        ));
    }
    Ok(decode_key_record(&bytes)?.secret_key)
}

fn key_path(purpose: KeyPurpose) -> crate::error::Result<crate::node_state::NodeStatePath> {
    crate::node_state::NodeStatePath::parse(key_file_name(purpose))
}

fn revocation_path(purpose: KeyPurpose) -> crate::error::Result<crate::node_state::NodeStatePath> {
    let path = format!("{}{REVOCATION_MARKER_SUFFIX}", key_file_name(purpose));
    crate::node_state::NodeStatePath::parse(&path)
}

const fn key_file_name(purpose: KeyPurpose) -> &'static str {
    match purpose {
        KeyPurpose::TransportEndpoint => "node-endpoint.secret",
        KeyPurpose::FederationOrigin => "crypto-federation-origin.key",
        KeyPurpose::Delegation => "crypto-delegation.key",
        KeyPurpose::EvidenceSigning => "crypto-evidence-signing.key",
        KeyPurpose::Authority => "crypto-authority.key",
    }
}

fn is_revoked(namespace: &crate::node_state::NodeStateNamespace, purpose: KeyPurpose) -> crate::error::Result<bool> {
    match namespace.observe_file(&revocation_path(purpose)?)? {
        crate::node_state::NodeStateFileObservation::Missing => Ok(false),
        crate::node_state::NodeStateFileObservation::Regular(file) => {
            require_restricted_permission(permission_status(&file))?;
            let marker = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
            if marker.is_empty() {
                return Err(crate::error::MoltenError::invalid_harness("cryptographic key revocation marker is empty"));
            }
            Ok(true)
        }
        crate::node_state::NodeStateFileObservation::NonRegular(kind) => {
            Err(crate::error::MoltenError::invalid_harness(format!(
                "cryptographic key revocation marker must be a regular file, got {kind:?}"
            )))
        }
    }
}

fn require_not_revoked(
    namespace: &crate::node_state::NodeStateNamespace,
    purpose: KeyPurpose,
) -> crate::error::Result<()> {
    if is_revoked(namespace, purpose)? {
        return Err(crate::error::MoltenError::invalid_harness(format!("{} key is revoked", purpose.as_str())));
    }
    Ok(())
}

fn require_current_handle(requested: &OpaqueKeyHandle, current: &OpaqueKeyHandle) -> crate::error::Result<()> {
    if requested != current {
        return Err(crate::error::MoltenError::invalid_harness("stale or mismatched cryptographic key handle denied"));
    }
    Ok(())
}

fn encode_key_record(record: &KeyRecord) -> [u8; KEY_RECORD_BYTES] {
    let mut bytes = [0u8; KEY_RECORD_BYTES];
    bytes[..KEY_GENERATION_START].copy_from_slice(KEY_RECORD_SCHEMA);
    bytes[KEY_GENERATION_START..KEY_SECRET_START].copy_from_slice(&record.generation.to_be_bytes());
    bytes[KEY_SECRET_START..].copy_from_slice(&record.secret_key.to_bytes());
    bytes
}
