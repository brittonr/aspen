
impl<'a> IrohEd25519<'a> {
    pub fn rotate(&self, request: &KeyRotationRequest) -> crate::error::Result<CompletedProductionRotation> {
        if request.overlap != RotationOverlapPolicy::None {
            return Err(crate::error::Failure::invalid_harness(
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
    ) -> crate::error::Result<StatusReadback> {
        let Some(observed) = self.observe_record(&key_path(purpose)?)? else {
            return Err(crate::error::Failure::invalid_harness("production key status is unavailable"));
        };
        let resolved = self.resolved_key(purpose, &observed.record, false, observed.permission_status)?;
        let revocations = read_key_revocations(self.namespace, purpose)?;
        let currentness = if key_revocation_matches(&revocations, KeyRevocationSubject::of(&resolved.handle.handle)) {
            KeyCurrentness::Revoked
        } else {
            KeyCurrentness::Current
        };
        self.status_for_handle(&resolved.handle.handle, observed.permission_status, currentness, receipt_refs)
    }

    /// Public status of one resolved key. It names the key's opaque handle reference and public key
    /// reference only; the key record itself never reaches the status projection.
    // r[impl molten.crypto_identity.status_handle_reference]
    fn status_for_handle(
        &self,
        handle: &OpaqueKeyHandle,
        permission_status: KeyPermissionStatus,
        currentness: KeyCurrentness,
        receipt_refs: Vec<String>,
    ) -> crate::error::Result<StatusReadback> {
        status(&AdapterDiagnosticInput {
            profile_ref: self.profile.profile.profile_ref.clone(),
            purpose: handle.purpose,
            generation: Some(handle.generation),
            currentness: Some(currentness),
            permission_status: permission_status.canonical(),
            backend_class: KeyBackendClass::CapabilityFile,
            public_key_ref: Some(handle.public_key_ref.clone()),
            handle_ref: Some(handle.handle_ref.clone()),
            receipt_refs,
            backend_locator: Some("capability-rooted-key-leaf".to_string()),
            raw_error: None,
            bearer_token: None,
            private_material_present: false,
        })
    }

    pub(crate) fn load_transport_secret(&self, handle: &OpaqueKeyHandle) -> crate::error::Result<iroh::SecretKey> {
        if handle.purpose != KeyPurpose::TransportEndpoint {
            return Err(crate::error::Failure::invalid_harness(
                "only transport-purpose handles may configure an Iroh endpoint",
            ));
        }
        let current = self.current_key(
            KeyPurpose::TransportEndpoint,
            &crate::preserves_rail::content_ref_from_bytes(b"transport-endpoint-load-policy"),
            false,
        )?;
        if current.resolved.handle.handle.handle_ref != handle.handle_ref
            || current.resolved.handle.handle.generation != handle.generation
        {
            return Err(crate::error::Failure::invalid_harness("stale transport key handle denied"));
        }
        Ok(current.record.secret_key)
    }
}

pub fn production_ed25519_profile(profile_ref: String, entropy_profile_ref: String) -> CryptoAdapterProfile {
    CryptoAdapterProfile {
        schema: CRYPTO_ADAPTER_PROFILE_SCHEMA.to_string(),
        profile_id: "molten.crypto.ed25519-iroh.v1".to_string(),
        profile_ref,
        class: CryptoProfileClass::Production,
        algorithm: CryptoAlgorithm::Ed25519Iroh,
        backend_classes: vec![KeyBackendClass::CapabilityFile, KeyBackendClass::ManagedSecret, KeyBackendClass::Tpm2Sealed],
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

pub(crate) fn transport_key_path() -> crate::error::Result<crate::node_state::RelativePath> {
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
    let profile = profile(&production_ed25519_profile(
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
    let handle = key_handle(KeyHandleInput {
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

pub(crate) fn load_transport_secret_for(
    namespace: &crate::node_state::Scope,
    expected_endpoint_id: &str,
    expected_handle_ref: &str,
    backend_ref: &str,
) -> crate::error::Result<iroh::SecretKey> {
    let observation = namespace.observe_file(&transport_key_path()?)?;
    let crate::node_state::FileObservation::Regular(file) = observation else {
        return Err(crate::error::Failure::invalid_harness("persisted transport key is unavailable"));
    };
    require_restricted_permission(permission_status(&file))?;
    let bytes = file.read_bounded(crate::node_state::MAX_NODE_SECRET_BYTES)?;
    let material = transport_endpoint_material(&bytes, backend_ref)?;
    if material.endpoint_id != expected_endpoint_id || material.handle_ref != expected_handle_ref {
        return Err(crate::error::Failure::invalid_harness(
            "persisted transport key does not match admitted node identity",
        ));
    }
    let record = decode_key_record(&bytes)?;
    let public_key_ref = crate::preserves_rail::content_ref_from_bytes(record.secret_key.public().as_bytes());
    require_key_not_revoked(namespace, KeyRevocationSubject {
        purpose: KeyPurpose::TransportEndpoint,
        handle_ref: &material.handle_ref,
        public_key_ref: &public_key_ref,
        generation: material.generation,
    })?;
    Ok(record.secret_key)
}

fn key_path(purpose: KeyPurpose) -> crate::error::Result<crate::node_state::RelativePath> {
    crate::node_state::RelativePath::parse(key_name(purpose))
}

pub(super) fn revocation_path(purpose: KeyPurpose) -> crate::error::Result<crate::node_state::RelativePath> {
    let path = format!("{}{REVOCATION_MARKER_SUFFIX}", key_name(purpose));
    crate::node_state::RelativePath::parse(&path)
}

const fn key_name(purpose: KeyPurpose) -> &'static str {
    match purpose {
        KeyPurpose::TransportEndpoint => "node-endpoint.secret",
        KeyPurpose::FederationOrigin => "crypto-federation-origin.key",
        KeyPurpose::Delegation => "crypto-delegation.key",
        KeyPurpose::EvidenceSigning => "crypto-evidence-signing.key",
        KeyPurpose::Authority => "crypto-authority.key",
    }
}
