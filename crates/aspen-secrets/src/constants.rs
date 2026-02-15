//! Tiger Style resource limits for secrets management.
//!
//! All limits are explicit to prevent resource exhaustion and ensure predictable behavior.

// ============================================================================
// SOPS / Bootstrap Secrets
// ============================================================================

/// Maximum size of a SOPS-encrypted secrets file (1 MB).
pub const MAX_SECRETS_FILE_SIZE: usize = 1024 * 1024;

/// Maximum size of decrypted age ciphertext (1 KB).
/// The SOPS data key is 32 bytes, so this provides generous headroom
/// while preventing compression/encryption bomb attacks.
pub const MAX_DECRYPTED_AGE_SIZE: usize = 1024;

/// Maximum number of trusted root keys.
pub const MAX_TRUSTED_ROOTS: usize = 64;

/// Maximum number of pre-built tokens in secrets file.
pub const MAX_PREBUILT_TOKENS: usize = 100;

/// Maximum size of a single secret value (256 KB).
pub const MAX_SECRET_VALUE_SIZE: usize = 256 * 1024;

/// Default cache TTL for decrypted secrets (300 seconds = 5 minutes).
pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;

// ============================================================================
// KV v2 Secrets Engine
// ============================================================================

/// Maximum size of a versioned secret (1 MB).
pub const MAX_KV_SECRET_SIZE: usize = 1024 * 1024;

/// Maximum number of versions to keep per secret.
pub const MAX_VERSIONS_PER_SECRET: u32 = 100;

/// Default number of versions to keep.
pub const DEFAULT_MAX_VERSIONS: u32 = 10;

/// Maximum length of a secret path.
pub const MAX_SECRET_PATH_LENGTH: usize = 512;

/// Maximum number of secrets per mount.
pub const MAX_SECRETS_PER_MOUNT: u32 = 100_000;

/// Maximum number of key-value pairs in a single secret.
pub const MAX_KV_PAIRS_PER_SECRET: usize = 100;

/// Maximum length of a secret key name.
pub const MAX_SECRET_KEY_NAME_LENGTH: usize = 256;

// ============================================================================
// Transit Engine
// ============================================================================

/// Maximum number of transit keys per mount.
pub const MAX_TRANSIT_KEYS_PER_MOUNT: u32 = 10_000;

/// Maximum number of key versions.
pub const MAX_KEY_VERSIONS: u32 = 100;

/// Maximum plaintext size for single encrypt operation (32 KB).
pub const MAX_PLAINTEXT_SIZE: usize = 32 * 1024;

/// Maximum batch size for transit operations.
pub const MAX_TRANSIT_BATCH_SIZE: u32 = 100;

/// Maximum transit key name length.
pub const MAX_TRANSIT_KEY_NAME_LENGTH: usize = 128;

/// Ciphertext wire format prefix.
pub const TRANSIT_CIPHERTEXT_PREFIX: &str = "aspen:v";

// ============================================================================
// PKI Engine
// ============================================================================

/// Maximum certificate TTL (10 years in seconds).
pub const MAX_CERT_TTL_SECS: u64 = 315_360_000;

/// Default certificate TTL (90 days in seconds).
pub const DEFAULT_CERT_TTL_SECS: u64 = 7_776_000;

/// Maximum number of issued certificates per mount.
pub const MAX_CERTS_PER_MOUNT: u32 = 100_000;

/// Maximum number of roles per mount.
pub const MAX_ROLES_PER_MOUNT: u32 = 1_000;

/// Maximum entries in CRL.
pub const MAX_CRL_ENTRIES: u32 = 100_000;

/// Maximum Subject Alternative Names per certificate.
pub const MAX_SAN_COUNT: u32 = 100;

/// Maximum role name length.
pub const MAX_ROLE_NAME_LENGTH: usize = 128;

/// Maximum Common Name length.
pub const MAX_COMMON_NAME_LENGTH: usize = 256;

// ============================================================================
// Storage Prefixes
// ============================================================================

/// System prefix for all secrets engine storage.
pub const SECRETS_SYSTEM_PREFIX: &str = "_system:secrets:";

/// KV v2 storage prefix.
pub const KV_PREFIX: &str = "_system:secrets:kv:";

/// Transit storage prefix.
pub const TRANSIT_PREFIX: &str = "_system:secrets:transit:";

/// PKI storage prefix.
pub const PKI_PREFIX: &str = "_system:secrets:pki:";

/// Policy storage prefix.
pub const POLICY_PREFIX: &str = "_system:secrets:policy:";

// ============================================================================
// General Limits
// ============================================================================

/// Maximum number of mounts.
pub const MAX_MOUNTS: u32 = 100;

/// Maximum mount name length.
pub const MAX_MOUNT_NAME_LENGTH: usize = 64;

/// Maximum concurrent secrets operations.
pub const MAX_CONCURRENT_OPS: usize = 1000;

// ============================================================================
// Cryptographic Constants
// ============================================================================

/// XChaCha20-Poly1305 nonce size (24 bytes).
pub const XCHACHA_NONCE_SIZE: usize = 24;

/// XChaCha20-Poly1305 key size (32 bytes).
pub const XCHACHA_KEY_SIZE: usize = 32;

/// XChaCha20-Poly1305 auth tag size (16 bytes).
pub const XCHACHA_TAG_SIZE: usize = 16;

/// Ed25519 signature size (64 bytes).
pub const ED25519_SIGNATURE_SIZE: usize = 64;

/// Ed25519 public key size (32 bytes).
pub const ED25519_PUBLIC_KEY_SIZE: usize = 32;

/// Ed25519 secret key size (32 bytes).
pub const ED25519_SECRET_KEY_SIZE: usize = 32;

/// BLAKE3 hash output size (32 bytes).
pub const BLAKE3_HASH_SIZE: usize = 32;

/// Domain separator for master key derivation.
pub const MASTER_KEY_DOMAIN: &[u8] = b"aspen-secrets-master-key-v1";

/// Domain separator for transit key derivation.
pub const TRANSIT_KEY_DOMAIN: &[u8] = b"aspen-secrets-transit-key-v1";

/// Domain separator for PKI key derivation.
pub const PKI_KEY_DOMAIN: &[u8] = b"aspen-secrets-pki-key-v1";
