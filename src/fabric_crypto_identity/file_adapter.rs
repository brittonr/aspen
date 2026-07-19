use std::str::FromStr;

use super::*;
use crate::error::MoltenError;
use crate::error::Result;
use crate::node_state::MAX_NODE_SECRET_BYTES;
use crate::node_state::NodeStateFileObservation;
use crate::node_state::NodeStateNamespace;
use crate::node_state::NodeStatePath;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const OWNER_ONLY_SECRET_FILE_MODE: u32 = 0o600;
#[cfg(unix)]
const GROUP_OR_OTHER_PERMISSION_BITS: u32 = 0o077;
const KEY_RECORD_SCHEMA_BYTES: usize = 8;
const KEY_RECORD_SCHEMA: &[u8; KEY_RECORD_SCHEMA_BYTES] = b"MCKEY001";
const KEY_GENERATION_BYTES: usize = std::mem::size_of::<u64>();
const ED25519_SECRET_BYTES: usize = 32;
const KEY_GENERATION_START: usize = KEY_RECORD_SCHEMA.len();
const KEY_SECRET_START: usize = KEY_GENERATION_START + KEY_GENERATION_BYTES;
const KEY_RECORD_BYTES: usize = KEY_SECRET_START + ED25519_SECRET_BYTES;
const FIRST_KEY_GENERATION: u64 = 1;
const REVOCATION_MARKER_SUFFIX: &str = ".revoked";
const NODE_TRANSPORT_PROFILE_LABEL: &[u8] = b"molten.node.transport.crypto-profile.v1";
const NODE_TRANSPORT_ENTROPY_LABEL: &[u8] = b"molten.node.transport.os-csprng.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPermissionStatus {
    Restricted,
    Unsafe,
    Unsupported,
}

impl KeyPermissionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Restricted => "restricted-owner-only",
            Self::Unsafe => "unsafe-shared",
            Self::Unsupported => "unsupported",
        }
    }

    const fn canonical(self) -> AdapterPermissionStatus {
        match self {
            Self::Restricted => AdapterPermissionStatus::Restricted,
            Self::Unsafe => AdapterPermissionStatus::Unsafe,
            Self::Unsupported => AdapterPermissionStatus::Unsupported,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProductionKey {
    pub handle: CanonicalKeyHandle,
    pub public_key: String,
    pub generated: bool,
    pub permission_status: KeyPermissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedProductionRotation {
    pub handle: CanonicalKeyHandle,
    pub outcome: KeyRotationOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransportEndpointKeyMaterial {
    pub public_key: String,
    pub endpoint_id: String,
    pub handle_ref: String,
    pub generation: u64,
}

#[derive(Debug)]
struct KeyRecord {
    generation: u64,
    secret_key: iroh::SecretKey,
}

pub(crate) struct ExactArtifactAuthSignature {
    pub public_key: String,
    pub signature_bytes: Vec<u8>,
}

pub struct IrohEd25519FileAdapter<'a> {
    namespace: &'a NodeStateNamespace,
    profile: CanonicalCryptoProfile,
    backend_ref: String,
}

impl<'a> IrohEd25519FileAdapter<'a> {
    pub fn new(
        namespace: &'a NodeStateNamespace,
        profile: CanonicalCryptoProfile,
        backend_ref: String,
    ) -> Result<Self> {
        admit_profile_for_production(&profile.profile)
            .map_err(|issues| validation_error("production crypto profile", &issues))?;
        if !profile.profile.backend_classes.contains(&KeyBackendClass::CapabilityFile) {
            return Err(MoltenError::invalid_harness("production crypto profile does not admit capability-file keys"));
        }
        require_blake3_ref("crypto file backend", &backend_ref)?;
        match namespace.kind() {
            crate::node_state::NodeStateNamespaceKind::Identity
            | crate::node_state::NodeStateNamespaceKind::Secrets => {}
            other => {
                return Err(MoltenError::invalid_harness(format!(
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
    ) -> Result<ResolvedProductionKey> {
        require_blake3_ref("key resolution policy", policy_ref)?;
        require_not_revoked(self.namespace, purpose)?;
        let path = key_path(purpose)?;
        match self.namespace.observe_file(&path)? {
            NodeStateFileObservation::Missing => {
                if !permit_first_boot_generation {
                    return Err(MoltenError::invalid_harness(
                        "required production key is unavailable and replacement generation is disabled",
                    ));
                }
                self.generate_first_key(purpose, policy_ref, &path)
            }
            NodeStateFileObservation::NonRegular(kind) => {
                Err(MoltenError::invalid_harness(format!("production key leaf must be a regular file, got {kind:?}")))
            }
            NodeStateFileObservation::Regular(file) => {
                let permission_status = permission_status(&file);
                require_restricted_permission(permission_status)?;
                let bytes = file.read_bounded(MAX_NODE_SECRET_BYTES)?;
                let record = decode_key_record(&bytes)?;
                self.resolved_key(purpose, &record, false, permission_status)
            }
        }
    }

    fn generate_first_key(
        &self,
        purpose: KeyPurpose,
        policy_ref: &str,
        path: &NodeStatePath,
    ) -> Result<ResolvedProductionKey> {
        let entropy_profile_ref = self
            .profile
            .profile
            .entropy_profile_ref
            .clone()
            .ok_or_else(|| MoltenError::invalid_harness("production crypto profile has no entropy profile"))?;
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
    ) -> Result<ResolvedProductionKey> {
        let public_key = record.secret_key.public();
        let public_key_ref = content_ref_from_bytes(public_key.as_bytes());
        let currentness_evidence_ref = currentness_evidence_ref(
            &self.profile.profile.profile_ref,
            purpose,
            record.generation,
            &public_key_ref,
            &self.backend_ref,
        )?;
        let handle = canonical_key_handle(
            &self.profile,
            purpose,
            record.generation,
            &public_key_ref,
            KeyBackendClass::CapabilityFile,
            &self.backend_ref,
            KeyCurrentness::Current,
            &currentness_evidence_ref,
        )?;
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
    ) -> Result<CanonicalSignatureOutcome> {
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
        if domain.domain_ref != canonical_hash(&domain.value)? {
            return Err(MoltenError::invalid_harness("signature domain ref does not match canonical value"));
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
    ) -> Result<ExactArtifactAuthSignature> {
        require_blake3_ref("artifact-auth signing policy", policy_ref)?;
        let current = self.resolve_or_generate(requested_handle.purpose, policy_ref, false)?;
        require_current_handle(requested_handle, &current.handle.handle)?;
        if statement.scope.profile_id != self.profile.profile.profile_id {
            return Err(MoltenError::invalid_harness("artifact-auth statement profile does not match the key profile"));
        }
        if statement.scope.purpose != requested_handle.purpose.as_str() {
            return Err(MoltenError::invalid_harness("artifact-auth statement purpose does not match the key purpose"));
        }
        let record = self.load_current_record(requested_handle.purpose)?;
        let public_key = record.secret_key.public();
        let key_identity = artifact_auth_ed25519::public_key_identity(public_key.as_bytes());
        if statement.key_identity != key_identity {
            return Err(MoltenError::invalid_harness(
                "artifact-auth statement full-key identity does not match the current key",
            ));
        }
        let statement_bytes = artifact_auth_core::canonical_statement_bytes(statement)
            .map_err(|_| MoltenError::invalid_harness("artifact-auth statement is not canonicalizable"))?;
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
        expected_domain: &CanonicalSignatureDomain,
        signature: &CanonicalSignatureOutcome,
        signer_currentness: KeyCurrentness,
        signer_generation: u64,
        policy_ref: &str,
    ) -> Result<CanonicalVerificationOutcome> {
        require_blake3_ref("verification policy", policy_ref)?;
        require_canonical_domain(&self.profile, expected_domain)?;
        require_canonical_signature(signature)?;
        let public_key = iroh::PublicKey::from_str(public_key)
            .map_err(|_| MoltenError::invalid_harness("verification public key is malformed"))?;
        let public_key_ref = content_ref_from_bytes(public_key.as_bytes());
        let parsed_signature = iroh::Signature::try_from(signature.signature.as_slice()).ok();
        let crypto_passed = parsed_signature
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
            cryptographic_verification_passed: crypto_passed,
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
    ) -> Result<CryptoStatusReadback> {
        require_blake3_ref("revocation evidence", revocation_evidence_ref)?;
        require_blake3_ref("revocation policy", policy_ref)?;
        let current = self.resolve_or_generate(handle.purpose, policy_ref, false)?;
        require_current_handle(handle, &current.handle.handle)?;
        let marker = record("fabric-crypto-key-revocation-v1", vec![
            string(&handle.handle_ref),
            string(&handle.public_key_ref),
            u64_value(handle.generation),
            string(revocation_evidence_ref),
            string(policy_ref),
        ]);
        self.namespace.write_restricted(
            &revocation_path(handle.purpose)?,
            &crate::preserves_rail::canonical_bytes(&marker)?,
            OWNER_ONLY_SECRET_FILE_MODE,
        )?;
        self.status_from_record(handle.purpose, KeyCurrentness::Revoked, vec![revocation_evidence_ref.to_string()])
    }

    pub fn rotate(&self, request: &KeyRotationRequest) -> Result<CompletedProductionRotation> {
        if request.overlap != RotationOverlapPolicy::None {
            return Err(MoltenError::invalid_harness(
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

    pub fn redacted_status(&self, purpose: KeyPurpose, receipt_refs: Vec<String>) -> Result<CryptoStatusReadback> {
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
    ) -> Result<CryptoStatusReadback> {
        let path = key_path(purpose)?;
        let NodeStateFileObservation::Regular(file) = self.namespace.observe_file(&path)? else {
            return Err(MoltenError::invalid_harness("production key status is unavailable"));
        };
        let permission_status = permission_status(&file);
        let record = decode_key_record(&file.read_bounded(MAX_NODE_SECRET_BYTES)?)?;
        canonical_crypto_status(&AdapterDiagnosticInput {
            profile_ref: self.profile.profile.profile_ref.clone(),
            purpose,
            generation: Some(record.generation),
            currentness: Some(currentness),
            permission_status: permission_status.canonical(),
            backend_class: KeyBackendClass::CapabilityFile,
            public_key_ref: Some(content_ref_from_bytes(record.secret_key.public().as_bytes())),
            receipt_refs,
            backend_locator: Some("capability-rooted-key-leaf".to_string()),
            raw_error: None,
            bearer_token: None,
            private_material_present: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn load_transport_secret(&self, handle: &OpaqueKeyHandle) -> Result<iroh::SecretKey> {
        if handle.purpose != KeyPurpose::TransportEndpoint {
            return Err(MoltenError::invalid_harness("only transport-purpose handles may configure an Iroh endpoint"));
        }
        let current = self.resolve_or_generate(
            KeyPurpose::TransportEndpoint,
            &content_ref_from_bytes(b"transport-endpoint-load-policy"),
            false,
        )?;
        if current.handle.handle.handle_ref != handle.handle_ref
            || current.handle.handle.generation != handle.generation
        {
            return Err(MoltenError::invalid_harness("stale transport key handle denied"));
        }
        Ok(self.load_current_record(KeyPurpose::TransportEndpoint)?.secret_key)
    }

    fn load_current_record(&self, purpose: KeyPurpose) -> Result<KeyRecord> {
        let path = key_path(purpose)?;
        let observation = self.namespace.observe_file(&path)?;
        let NodeStateFileObservation::Regular(file) = observation else {
            return Err(MoltenError::invalid_harness("current production key is unavailable"));
        };
        require_restricted_permission(permission_status(&file))?;
        decode_key_record(&file.read_bounded(MAX_NODE_SECRET_BYTES)?)
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

pub(crate) fn transport_key_path() -> Result<NodeStatePath> {
    key_path(KeyPurpose::TransportEndpoint)
}

pub(crate) fn generate_transport_key_record() -> Vec<u8> {
    encode_key_record(&KeyRecord {
        generation: FIRST_KEY_GENERATION,
        secret_key: iroh::SecretKey::generate(),
    })
    .to_vec()
}

pub(crate) fn transport_key_record_from_secret_hex(secret: &str) -> Result<Vec<u8>> {
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
) -> Result<TransportEndpointKeyMaterial> {
    require_blake3_ref("transport identity backend", backend_ref)?;
    let key_record = decode_key_record(record_bytes)?;
    let public_key = key_record.secret_key.public();
    let public_key_ref = content_ref_from_bytes(public_key.as_bytes());
    let profile = canonical_crypto_profile(&production_ed25519_profile(
        content_ref_from_bytes(NODE_TRANSPORT_PROFILE_LABEL),
        content_ref_from_bytes(NODE_TRANSPORT_ENTROPY_LABEL),
    ))?;
    let currentness_ref = currentness_evidence_ref(
        &profile.profile.profile_ref,
        KeyPurpose::TransportEndpoint,
        key_record.generation,
        &public_key_ref,
        backend_ref,
    )?;
    let handle = canonical_key_handle(
        &profile,
        KeyPurpose::TransportEndpoint,
        key_record.generation,
        &public_key_ref,
        KeyBackendClass::CapabilityFile,
        backend_ref,
        KeyCurrentness::Current,
        &currentness_ref,
    )?;
    let handle_ref = handle.handle.handle_ref;
    Ok(TransportEndpointKeyMaterial {
        public_key: public_key.to_string(),
        endpoint_id: format!("iroh:{public_key}"),
        handle_ref,
        generation: key_record.generation,
    })
}

pub(crate) fn load_transport_secret_for_identity(
    namespace: &NodeStateNamespace,
    expected_endpoint_id: &str,
    expected_handle_ref: &str,
    backend_ref: &str,
) -> Result<iroh::SecretKey> {
    require_not_revoked(namespace, KeyPurpose::TransportEndpoint)?;
    let observation = namespace.observe_file(&transport_key_path()?)?;
    let NodeStateFileObservation::Regular(file) = observation else {
        return Err(MoltenError::invalid_harness("persisted transport key is unavailable"));
    };
    require_restricted_permission(permission_status(&file))?;
    let bytes = file.read_bounded(MAX_NODE_SECRET_BYTES)?;
    let material = transport_endpoint_material(&bytes, backend_ref)?;
    if material.endpoint_id != expected_endpoint_id || material.handle_ref != expected_handle_ref {
        return Err(MoltenError::invalid_harness("persisted transport key does not match admitted node identity"));
    }
    Ok(decode_key_record(&bytes)?.secret_key)
}

fn key_path(purpose: KeyPurpose) -> Result<NodeStatePath> {
    NodeStatePath::parse(key_file_name(purpose))
}

fn revocation_path(purpose: KeyPurpose) -> Result<NodeStatePath> {
    let path = format!("{}{REVOCATION_MARKER_SUFFIX}", key_file_name(purpose));
    NodeStatePath::parse(&path)
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

fn is_revoked(namespace: &NodeStateNamespace, purpose: KeyPurpose) -> Result<bool> {
    match namespace.observe_file(&revocation_path(purpose)?)? {
        NodeStateFileObservation::Missing => Ok(false),
        NodeStateFileObservation::Regular(file) => {
            require_restricted_permission(permission_status(&file))?;
            let marker = file.read_bounded(MAX_NODE_SECRET_BYTES)?;
            if marker.is_empty() {
                return Err(MoltenError::invalid_harness("cryptographic key revocation marker is empty"));
            }
            Ok(true)
        }
        NodeStateFileObservation::NonRegular(kind) => Err(MoltenError::invalid_harness(format!(
            "cryptographic key revocation marker must be a regular file, got {kind:?}"
        ))),
    }
}

fn require_not_revoked(namespace: &NodeStateNamespace, purpose: KeyPurpose) -> Result<()> {
    if is_revoked(namespace, purpose)? {
        return Err(MoltenError::invalid_harness(format!("{} key is revoked", purpose.as_str())));
    }
    Ok(())
}

fn require_current_handle(requested: &OpaqueKeyHandle, current: &OpaqueKeyHandle) -> Result<()> {
    if requested != current {
        return Err(MoltenError::invalid_harness("stale or mismatched cryptographic key handle denied"));
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

fn decode_key_record(bytes: &[u8]) -> Result<KeyRecord> {
    let bytes: [u8; KEY_RECORD_BYTES] = bytes
        .try_into()
        .map_err(|_| MoltenError::invalid_harness("production key record has an invalid length"))?;
    if &bytes[..KEY_GENERATION_START] != KEY_RECORD_SCHEMA {
        return Err(MoltenError::invalid_harness("production key record schema is malformed"));
    }
    let generation_bytes: [u8; KEY_GENERATION_BYTES] = bytes[KEY_GENERATION_START..KEY_SECRET_START]
        .try_into()
        .map_err(|_| MoltenError::invalid_harness("production key generation is malformed"))?;
    let generation = u64::from_be_bytes(generation_bytes);
    if generation == 0 {
        return Err(MoltenError::invalid_harness("production key generation must be positive"));
    }
    let secret_bytes: [u8; ED25519_SECRET_BYTES] = bytes[KEY_SECRET_START..]
        .try_into()
        .map_err(|_| MoltenError::invalid_harness("production secret key bytes are malformed"))?;
    Ok(KeyRecord {
        generation,
        secret_key: iroh::SecretKey::from_bytes(&secret_bytes),
    })
}

fn decode_secret_hex(secret: &str) -> Result<[u8; ED25519_SECRET_BYTES]> {
    const HEX_CHARS_PER_BYTE: usize = 2;
    const HEX_RADIX: u32 = 16;
    const EXPECTED_HEX_CHARS: usize = ED25519_SECRET_BYTES * HEX_CHARS_PER_BYTE;
    let secret = secret.trim();
    if secret.len() != EXPECTED_HEX_CHARS {
        return Err(MoltenError::invalid_harness(format!(
            "explicit Ed25519 secret must contain exactly {EXPECTED_HEX_CHARS} lowercase hexadecimal characters"
        )));
    }
    let mut bytes = [0u8; ED25519_SECRET_BYTES];
    for (index, slot) in bytes.iter_mut().enumerate() {
        let offset = index
            .checked_mul(HEX_CHARS_PER_BYTE)
            .ok_or_else(|| MoltenError::invalid_harness("secret hex offset overflow"))?;
        let pair = &secret[offset..offset + HEX_CHARS_PER_BYTE];
        if !pair.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f')) {
            return Err(MoltenError::invalid_harness(
                "explicit Ed25519 secret must use lowercase hexadecimal characters",
            ));
        }
        *slot = u8::from_str_radix(pair, HEX_RADIX)
            .map_err(|_| MoltenError::invalid_harness("explicit Ed25519 secret contains malformed hex"))?;
    }
    Ok(bytes)
}

fn currentness_evidence_ref(
    profile_ref: &str,
    purpose: KeyPurpose,
    generation: u64,
    public_key_ref: &str,
    backend_ref: &str,
) -> Result<String> {
    canonical_hash(&record("crypto-key-currentness-v1", vec![
        string(profile_ref),
        string(purpose.as_str()),
        u64_value(generation),
        string(public_key_ref),
        string(backend_ref),
        string("current"),
    ]))
}

fn permission_status(file: &crate::node_state::NodeStateFile) -> KeyPermissionStatus {
    #[cfg(unix)]
    {
        match file.unix_mode() {
            Some(mode) if mode & GROUP_OR_OTHER_PERMISSION_BITS == 0 => KeyPermissionStatus::Restricted,
            Some(_) => KeyPermissionStatus::Unsafe,
            None => KeyPermissionStatus::Unsupported,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        KeyPermissionStatus::Unsupported
    }
}

fn require_canonical_domain(profile: &CanonicalCryptoProfile, supplied: &CanonicalSignatureDomain) -> Result<()> {
    let rebuilt = canonical_signature_domain(profile, &supplied.domain)?;
    if &rebuilt != supplied {
        return Err(MoltenError::invalid_harness("signature domain does not match its canonical Preserves identity"));
    }
    Ok(())
}

fn require_canonical_signature(supplied: &CanonicalSignatureOutcome) -> Result<()> {
    let signature_bytes = u64::try_from(supplied.signature.len())
        .map_err(|_| MoltenError::invalid_harness("signature length does not fit u64"))?;
    let signature_ref = content_ref_from_bytes(&supplied.signature);
    let value = canonical_signature_value(&supplied.metadata, &supplied.signature);
    let outcome_ref = canonical_hash(&value)?;
    if supplied.metadata.signature_bytes != signature_bytes
        || supplied.metadata.signature_ref != signature_ref
        || supplied.value != value
        || supplied.outcome_ref != outcome_ref
    {
        return Err(MoltenError::invalid_harness("signature outcome does not match its canonical Preserves identity"));
    }
    Ok(())
}

fn require_restricted_permission(status: KeyPermissionStatus) -> Result<()> {
    match status {
        KeyPermissionStatus::Restricted => Ok(()),
        KeyPermissionStatus::Unsafe => {
            Err(MoltenError::invalid_harness("production key permissions are not owner-only"))
        }
        KeyPermissionStatus::Unsupported => {
            Err(MoltenError::invalid_harness("production key permission verification is unavailable"))
        }
    }
}

fn require_blake3_ref(label: &str, value: &str) -> Result<()> {
    const BLAKE3_PREFIX: &str = "blake3:";
    const BLAKE3_HEX_LENGTH: usize = 64;
    let is_valid = value.strip_prefix(BLAKE3_PREFIX).is_some_and(|hex| {
        hex.len() == BLAKE3_HEX_LENGTH && hex.chars().all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    });
    if is_valid {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} ref is malformed")))
    }
}

fn validation_error<T: std::fmt::Debug>(label: &str, issues: &[T]) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation failed: {issues:?}"))
}
