//! Hash conversion logic for mapping BLAKE3 hashes to DHT infohashes.

use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;

/// Convert a BLAKE3 hash + format to a DHT-compatible infohash.
///
/// Uses SHA-1 of the BLAKE3 hash concatenated with the format byte
/// to create a 20-byte infohash for DHT lookups.
pub fn to_dht_infohash(hash: &Hash, format: BlobFormat) -> [u8; 20] {
    use sha2::Digest;
    use sha2::Sha256;

    // Concatenate hash bytes with format discriminant
    let mut hasher = Sha256::new();
    hasher.update(hash.as_bytes());
    hasher.update([format_to_byte(format)]);
    let full_hash = hasher.finalize();

    // Truncate to 20 bytes for DHT compatibility
    let mut infohash = [0u8; 20];
    infohash.copy_from_slice(&full_hash[..20]);
    infohash
}

fn format_to_byte(format: BlobFormat) -> u8 {
    match format {
        BlobFormat::Raw => 0,
        BlobFormat::HashSeq => 1,
    }
}

/// Convert an iroh SecretKey to a mainline SigningKey.
///
/// Iroh uses the same ed25519 curve, so this is a direct byte copy.
/// We use mainline's re-exported SigningKey to avoid version conflicts.
#[cfg(feature = "global-discovery")]
pub(crate) fn iroh_secret_to_signing_key(secret_key: &iroh::SecretKey) -> mainline::SigningKey {
    let secret_bytes: [u8; 32] = secret_key.to_bytes();
    mainline::SigningKey::from_bytes(&secret_bytes)
}

/// Stub signing key type when feature is disabled.
#[cfg(not(feature = "global-discovery"))]
pub(crate) struct StubSigningKey;

/// Convert an iroh SecretKey to a stub signing key (no-op when feature disabled).
#[cfg(not(feature = "global-discovery"))]
pub(crate) fn iroh_secret_to_signing_key(_secret_key: &iroh::SecretKey) -> StubSigningKey {
    StubSigningKey
}
