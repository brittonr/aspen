use std::str::FromStr;

use super::*;

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
    namespace: &'a crate::node_state::NodeStateNamespace,
    profile: CanonicalCryptoProfile,
    backend_ref: String,
}

/// The domain, signature, signer currentness and generation, and policy a verification is evaluated
/// against.
#[derive(Clone, Copy)]
pub struct VerificationInput<'a> {
    pub expected_domain: &'a CanonicalSignatureDomain,
    pub signature: &'a CanonicalSignatureOutcome,
    pub signer_currentness: KeyCurrentness,
    pub signer_generation: u64,
    pub policy_ref: &'a str,
}
