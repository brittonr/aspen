use std::collections::BTreeSet;

pub const CRYPTO_ADAPTER_PROFILE_SCHEMA: &str = "molten.crypto-identity.adapter-profile.v1";
pub const OPAQUE_KEY_HANDLE_SCHEMA: &str = "molten.crypto-identity.opaque-key-handle.v1";
pub const SIGNATURE_DOMAIN_SCHEMA: &str = "molten.crypto-identity.signature-domain.v1";
pub const CRYPTO_OPERATION_SCHEMA: &str = "molten.crypto-identity.operation.v1";
pub const CRYPTO_OUTCOME_SCHEMA: &str = "molten.crypto-identity.outcome.v1";
pub const CRYPTO_ROTATION_SCHEMA: &str = "molten.crypto-identity.rotation.v1";
pub const CRYPTO_STATUS_SCHEMA: &str = "molten.crypto-identity.status.v1";

pub const MAX_CRYPTO_TEXT_BYTES: usize = 256;
pub const MAX_CRYPTO_COLLECTION_ITEMS: usize = 256;
pub const MAX_SIGNATURE_BYTES: u64 = 1_024;
const REQUIRED_NON_CLAIM_COUNT: usize = 6;
pub(crate) const ADJACENT_PAIR_WIDTH: usize = 2;

pub const REQUIRED_CRYPTO_NON_CLAIMS: [CryptoNonClaim; REQUIRED_NON_CLAIM_COUNT] = [
    CryptoNonClaim::NoCapabilityAuthority,
    CryptoNonClaim::NoMembership,
    CryptoNonClaim::NoProvenance,
    CryptoNonClaim::NoPayloadCorrectness,
    CryptoNonClaim::NoBackendAvailability,
    CryptoNonClaim::NoAlgorithmAgility,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CryptoNonClaim {
    NoCapabilityAuthority,
    NoMembership,
    NoProvenance,
    NoPayloadCorrectness,
    NoBackendAvailability,
    NoAlgorithmAgility,
}

impl CryptoNonClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCapabilityAuthority => "verified-signature-does-not-grant-capability-authority",
            Self::NoMembership => "public-identity-does-not-grant-membership",
            Self::NoProvenance => "signature-alone-does-not-prove-provenance",
            Self::NoPayloadCorrectness => "signature-does-not-prove-payload-correctness",
            Self::NoBackendAvailability => "profile-does-not-guarantee-backend-availability",
            Self::NoAlgorithmAgility => "profile-does-not-imply-algorithm-agility",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CryptoProfileClass {
    Production,
    FixtureSimulation,
}

impl CryptoProfileClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Production => "production",
            Self::FixtureSimulation => "fixture-simulation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CryptoAlgorithm {
    Ed25519Iroh,
    Blake3Fixture,
}

impl CryptoAlgorithm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519Iroh => "ed25519-iroh-v1",
            Self::Blake3Fixture => "blake3-local-fixture-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyBackendClass {
    CapabilityFile,
    ManagedSecret,
    InMemoryFixture,
}

impl KeyBackendClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityFile => "capability-file",
            Self::ManagedSecret => "managed-secret",
            Self::InMemoryFixture => "in-memory-fixture",
        }
    }

    pub const fn is_production_eligible(self) -> bool {
        matches!(self, Self::CapabilityFile | Self::ManagedSecret)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyPurpose {
    TransportEndpoint,
    FederationOrigin,
    Delegation,
    EvidenceSigning,
    Authority,
}

impl KeyPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransportEndpoint => "transport-endpoint",
            Self::FederationOrigin => "federation-origin",
            Self::Delegation => "delegation",
            Self::EvidenceSigning => "evidence-signing",
            Self::Authority => "authority",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoAdapterProfile {
    pub schema: String,
    pub profile_id: String,
    pub profile_ref: String,
    pub class: CryptoProfileClass,
    pub algorithm: CryptoAlgorithm,
    pub backend_classes: Vec<KeyBackendClass>,
    pub allowed_purposes: Vec<KeyPurpose>,
    pub entropy_profile_ref: Option<String>,
    pub domain_version: String,
    pub allow_key_sharing: bool,
    pub sharing_policy_ref: Option<String>,
    pub max_signature_bytes: u64,
    pub non_claims: Vec<CryptoNonClaim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCurrentness {
    Current,
    Overlap,
    Superseded,
    Revoked,
}

impl KeyCurrentness {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Overlap => "overlap",
            Self::Superseded => "superseded",
            Self::Revoked => "revoked",
        }
    }

    pub const fn permits_signing(self) -> bool {
        matches!(self, Self::Current | Self::Overlap)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterPermissionStatus {
    Restricted,
    Unsafe,
    Unsupported,
}

impl AdapterPermissionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Restricted => "restricted-owner-only",
            Self::Unsafe => "unsafe-shared",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueKeyHandle {
    pub schema: String,
    pub handle_ref: String,
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub generation: u64,
    pub public_key_ref: String,
    pub backend_class: KeyBackendClass,
    pub backend_ref: String,
    pub currentness: KeyCurrentness,
    pub currentness_evidence_ref: String,
    pub secret_material_exposed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyGenerationRequest {
    pub operation_id: String,
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub backend_class: KeyBackendClass,
    pub backend_ref: String,
    pub entropy_profile_ref: String,
    pub generation: u64,
    pub policy_ref: String,
    pub permit_first_boot_generation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyGenerationPlan {
    pub request: KeyGenerationRequest,
    pub algorithm: CryptoAlgorithm,
    pub persist_restricted: bool,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureDomain {
    pub schema: String,
    pub domain_id: String,
    pub domain_version: String,
    pub purpose: KeyPurpose,
    pub payload_schema: String,
    pub payload_ref: String,
    pub signer_public_ref: String,
    pub verifier_context_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignRequest {
    pub operation_id: String,
    pub profile_ref: String,
    pub handle: OpaqueKeyHandle,
    pub domain: SignatureDomain,
    pub current_generation: u64,
    pub current_handle_ref: String,
    pub policy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignPlan {
    pub operation_id: String,
    pub handle_ref: String,
    pub profile_ref: String,
    pub algorithm: CryptoAlgorithm,
    pub purpose: KeyPurpose,
    pub generation: u64,
    pub domain: SignatureDomain,
    pub policy_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureMetadata {
    pub profile_ref: String,
    pub algorithm: CryptoAlgorithm,
    pub purpose: KeyPurpose,
    pub generation: u64,
    pub signer_public_ref: String,
    pub domain_ref: String,
    pub payload_ref: String,
    pub verifier_context_ref: String,
    pub signature_ref: String,
    pub signature_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequest {
    pub operation_id: String,
    pub profile_ref: String,
    pub expected_domain: SignatureDomain,
    pub observed: SignatureMetadata,
    pub cryptographic_verification_passed: bool,
    pub signer_currentness: KeyCurrentness,
    pub signer_generation: u64,
    pub policy_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDecisionKind {
    Accept,
    Deny,
}

impl VerificationDecisionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationDecision {
    pub kind: VerificationDecisionKind,
    pub issues: Vec<CryptoIdentityIssue>,
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub payload_ref: String,
    pub signature_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterDiagnosticInput {
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub generation: Option<u64>,
    pub currentness: Option<KeyCurrentness>,
    pub permission_status: AdapterPermissionStatus,
    pub backend_class: KeyBackendClass,
    pub public_key_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub backend_locator: Option<String>,
    pub raw_error: Option<String>,
    pub bearer_token: Option<String>,
    pub private_material_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedAdapterStatus {
    pub profile_ref: String,
    pub purpose: KeyPurpose,
    pub generation: Option<u64>,
    pub currentness: Option<KeyCurrentness>,
    pub permission_status: AdapterPermissionStatus,
    pub backend_class: KeyBackendClass,
    pub public_key_ref: Option<String>,
    pub receipt_refs: Vec<String>,
    pub has_redacted_backend_locator: bool,
    pub has_redacted_error: bool,
    pub has_redacted_bearer_token: bool,
    pub denied_private_material: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoIdentityIssue {
    SchemaMismatch(&'static str),
    EmptyField(&'static str),
    MalformedToken(&'static str),
    MalformedRef(&'static str),
    CollectionLimitExceeded(&'static str),
    DuplicateValue(&'static str),
    MissingNonClaim(&'static str),
    UnsupportedProductionAlgorithm,
    FixtureProfileDeniedInProduction,
    FixtureBackendDeniedInProduction,
    MissingEntropyProfile,
    KeySharingPolicyRequired,
    UnsupportedPurpose(KeyPurpose),
    UnsupportedBackend(KeyBackendClass),
    ZeroGeneration,
    GenerationNotAdvanced,
    ProfileMismatch,
    PurposeMismatch,
    DomainVersionMismatch,
    PayloadSchemaMismatch,
    PayloadRefMismatch,
    SignerPublicRefMismatch,
    VerifierContextMismatch,
    HandleNotCurrent(KeyCurrentness),
    HandleGenerationStale { expected: u64, actual: u64 },
    HandleRefStale,
    SecretMaterialExposed,
    SignatureTooLarge { actual: u64, maximum: u64 },
    SignatureMalformed,
    CryptographicVerificationFailed,
    BackendUnavailable,
    UnsafePermissions,
    RotationPolicyMismatch,
    RevocationEvidenceRequired,
    DiagnosticSecretLeak,
}

pub(crate) fn required_non_claim_issues(claims: &[CryptoNonClaim]) -> Vec<CryptoIdentityIssue> {
    let mut issues = Vec::new();
    if claims.len() > MAX_CRYPTO_COLLECTION_ITEMS {
        issues.push(CryptoIdentityIssue::CollectionLimitExceeded("crypto-non-claims"));
    }
    let supplied = claims.iter().copied().collect::<BTreeSet<_>>();
    if supplied.len() != claims.len() {
        issues.push(CryptoIdentityIssue::DuplicateValue("crypto-non-claim"));
    }
    for required in REQUIRED_CRYPTO_NON_CLAIMS {
        if !supplied.contains(&required) {
            issues.push(CryptoIdentityIssue::MissingNonClaim(required.as_str()));
        }
    }
    issues
}
