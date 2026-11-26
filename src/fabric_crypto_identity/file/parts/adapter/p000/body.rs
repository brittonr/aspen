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
const KEY_REVOCATION_LABEL: &str = "fabric-crypto-key-revocation-v1";
const KEY_REVOCATIONS_LABEL: &str = "fabric-crypto-key-revocations-v1";
const KEY_REVOCATION_ARITY: usize = 5;
pub(super) const MAX_KEY_REVOCATIONS: usize = 64;
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
    pub handle: KeyHandle,
    pub public_key: String,
    pub generated: bool,
    pub permission_status: KeyPermissionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedProductionRotation {
    pub handle: KeyHandle,
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

/// The verified handle and the key material read from one opened key-leaf observation.
#[derive(Debug)]
struct CurrentKey {
    resolved: ResolvedProductionKey,
    record: KeyRecord,
}

#[derive(Debug)]
struct ObservedKeyRecord {
    record: KeyRecord,
    permission_status: KeyPermissionStatus,
}

/// One persisted revocation record: the exact revoked handle, its generation and public key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeyRevocation {
    pub handle_ref: String,
    pub public_key_ref: String,
    pub generation: u64,
    pub evidence_ref: String,
    pub policy_ref: String,
}

/// Test-only hooks at the adapter's observe/sign and revocation read/write boundaries, so a
/// regression can force a concurrent rotation or revocation into exactly that window.
#[cfg(test)]
pub(super) mod seam {
    type Hook = Box<dyn FnMut(&super::IrohEd25519<'_>)>;

    thread_local! {
        static HOOKS: std::cell::RefCell<Vec<(&'static str, Hook)>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    pub(in super::super) fn set(point: &'static str, hook: impl FnMut(&super::IrohEd25519<'_>) + 'static) {
        HOOKS.with(|hooks| hooks.borrow_mut().push((point, Box::new(hook))));
    }

    pub(super) fn fire(point: &'static str, adapter: &super::IrohEd25519<'_>) {
        let hook = HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            let index = hooks.iter().position(|(name, _)| *name == point)?;
            Some(hooks.remove(index).1)
        });
        if let Some(mut hook) = hook {
            hook(adapter);
        }
    }
}

/// The exact key a revocation check is about: its purpose, handle, generation and public key.
#[derive(Debug, Clone, Copy)]
pub(super) struct KeyRevocationSubject<'a> {
    pub purpose: KeyPurpose,
    pub handle_ref: &'a str,
    pub public_key_ref: &'a str,
    pub generation: u64,
}

impl<'a> KeyRevocationSubject<'a> {
    pub(super) fn of(handle: &'a OpaqueKeyHandle) -> Self {
        Self {
            purpose: handle.purpose,
            handle_ref: &handle.handle_ref,
            public_key_ref: &handle.public_key_ref,
            generation: handle.generation,
        }
    }
}

pub(crate) struct ExactArtifactAuthSignature {
    pub public_key: String,
    pub signature_bytes: Vec<u8>,
}

pub struct IrohEd25519<'a> {
    namespace: &'a crate::node_state::Scope,
    profile: Profile,
    backend_ref: String,
}

/// The domain, signature, signer currentness and generation, and policy a verification is evaluated
/// against.
#[derive(Clone, Copy)]
pub struct VerificationInput<'a> {
    pub expected_domain: &'a DomainEnvelope,
    pub signature: &'a SignatureOutcome,
    pub signer_currentness: KeyCurrentness,
    pub signer_generation: u64,
    pub policy_ref: &'a str,
}
