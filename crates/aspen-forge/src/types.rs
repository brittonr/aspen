//! Core types for the Forge module.
//!
//! This module defines the fundamental data structures used throughout Forge,
//! including the `SignedObject` wrapper for content-addressed, authenticated objects.

// Re-export Signature from aspen-core for use in this crate
pub use aspen_core::Signature;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use iroh::PublicKey;
use serde::Deserialize;
use serde::Serialize;

use super::error::ForgeError;
use super::error::ForgeResult;

/// A content-addressed, cryptographically signed object.
///
/// All immutable objects in Forge (Git objects, COB changes, attestations) are
/// wrapped in this structure, which provides:
///
/// - **Content addressing**: The BLAKE3 hash of the serialized object serves as its ID
/// - **Authentication**: Ed25519 signature from the author
/// - **HLC Timestamping**: Hybrid Logical Clock timestamp for deterministic ordering
///
/// # Type Parameter
///
/// - `T`: The payload type, which must implement `Serialize` and `Deserialize`
///
/// # Example
///
/// ```ignore
/// use aspen_forge::types::SignedObject;
/// use aspen_core::hlc::create_hlc;
///
/// let hlc = create_hlc("node-1");
/// let payload = MyPayload { /* ... */ };
/// let signed = SignedObject::new(payload, &secret_key, &hlc)?;
///
/// // Verify the signature
/// signed.verify()?;
///
/// // Get the content-addressed hash
/// let hash = signed.hash();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedObject<T> {
    /// The payload data.
    pub payload: T,

    /// The public key of the signer (author).
    pub author: PublicKey,

    /// HLC timestamp for deterministic distributed ordering.
    pub hlc_timestamp: SerializableTimestamp,

    /// Ed25519 signature over the payload + author + timestamp.
    pub signature: Signature,
}

impl<T: Serialize> SignedObject<T> {
    /// Create a new signed object.
    ///
    /// Signs the payload with the provided secret key and uses the HLC for timestamping.
    ///
    /// # Arguments
    ///
    /// * `payload` - The data to sign
    /// * `secret_key` - The Ed25519 secret key for signing
    /// * `hlc` - The Hybrid Logical Clock for generating timestamps
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn new(payload: T, secret_key: &iroh::SecretKey, hlc: &HLC) -> ForgeResult<Self> {
        let author = secret_key.public();
        let hlc_timestamp = SerializableTimestamp::from(hlc.new_timestamp());

        // Serialize the signing data (payload + author + timestamp)
        let signing_data = Self::signing_data(&payload, &author, &hlc_timestamp)?;

        // Sign with Ed25519
        let sig = secret_key.sign(&signing_data);
        let signature = Signature::from_bytes(sig.to_bytes());

        Ok(Self {
            payload,
            author,
            hlc_timestamp,
            signature,
        })
    }

    /// Compute the BLAKE3 hash of this signed object.
    ///
    /// This hash serves as the content address (ID) of the object.
    pub fn hash(&self) -> blake3::Hash {
        let bytes = self.to_bytes();
        blake3::hash(&bytes)
    }

    /// Serialize the object to bytes using postcard.
    pub fn to_bytes(&self) -> Vec<u8> {
        // This should not fail for well-formed objects
        postcard::to_allocvec(self).expect("serialization should not fail")
    }

    /// Compute the data that is signed (payload + author + timestamp).
    fn signing_data(payload: &T, author: &PublicKey, hlc_timestamp: &SerializableTimestamp) -> ForgeResult<Vec<u8>> {
        #[derive(Serialize)]
        struct SigningData<'a, T> {
            payload: &'a T,
            author: &'a PublicKey,
            hlc_timestamp: &'a SerializableTimestamp,
        }

        let data = SigningData {
            payload,
            author,
            hlc_timestamp,
        };

        postcard::to_allocvec(&data).map_err(ForgeError::from)
    }
}

impl<T: Serialize + for<'de> Deserialize<'de>> SignedObject<T> {
    /// Verify the signature on this object.
    ///
    /// # Errors
    ///
    /// Returns `ForgeError::InvalidSignature` if verification fails.
    pub fn verify(&self) -> ForgeResult<()> {
        let signing_data = Self::signing_data(&self.payload, &self.author, &self.hlc_timestamp)?;

        let sig = iroh::Signature::from_bytes(self.signature.as_bytes());

        self.author
            .verify(&signing_data, &sig)
            .map_err(|e| ForgeError::InvalidSignature { message: e.to_string() })
    }

    /// Deserialize a signed object from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> ForgeResult<Self> {
        postcard::from_bytes(bytes).map_err(ForgeError::from)
    }
}

/// Convert an iroh Hash to a hex string for display.
pub fn hash_to_hex(hash: &blake3::Hash) -> String {
    hex::encode(hash.as_bytes())
}

/// Parse a hex string to a BLAKE3 hash.
///
/// # Errors
///
/// Returns an error if the string is not valid hex or not 32 bytes.
pub fn hash_from_hex(s: &str) -> ForgeResult<blake3::Hash> {
    let bytes = hex::decode(s).map_err(|e| ForgeError::InvalidObject {
        message: format!("invalid hex: {}", e),
    })?;

    if bytes.len() != 32 {
        return Err(ForgeError::InvalidObject {
            message: format!("hash must be 32 bytes, got {}", bytes.len()),
        });
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(blake3::Hash::from_bytes(arr))
}

#[cfg(test)]
mod tests {
    use aspen_core::hlc::create_hlc;

    use super::*;

    #[test]
    fn test_signed_object_roundtrip() {
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("test-node");

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        struct TestPayload {
            message: String,
            value: u64,
        }

        let payload = TestPayload {
            message: "hello".to_string(),
            value: 42,
        };

        let signed = SignedObject::new(payload.clone(), &secret_key, &hlc).expect("should sign");

        // Verify signature
        signed.verify().expect("signature should be valid");

        // Serialize and deserialize
        let bytes = signed.to_bytes();
        let recovered: SignedObject<TestPayload> = SignedObject::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(recovered.payload, payload);
        assert_eq!(recovered.author, secret_key.public());

        // Verify recovered signature
        recovered.verify().expect("recovered signature should be valid");

        // Hash should be deterministic
        assert_eq!(signed.hash(), recovered.hash());
    }

    #[test]
    fn test_signed_object_hlc_ordering() {
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        let hlc = create_hlc("test-node");

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        struct TestPayload {
            value: u64,
        }

        let signed1 = SignedObject::new(TestPayload { value: 1 }, &secret_key, &hlc).expect("should sign");
        let signed2 = SignedObject::new(TestPayload { value: 2 }, &secret_key, &hlc).expect("should sign");

        // HLC timestamps should be strictly ordered
        assert!(signed2.hlc_timestamp > signed1.hlc_timestamp, "second object should have later HLC timestamp");
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let hash = blake3::hash(b"test data");
        let hex_str = hash_to_hex(&hash);
        let recovered = hash_from_hex(&hex_str).expect("should parse");
        assert_eq!(hash, recovered);
    }

    #[test]
    fn test_invalid_hash_hex() {
        // Too short
        assert!(hash_from_hex("abcd").is_err());

        // Invalid hex
        assert!(hash_from_hex("not valid hex!").is_err());

        // Wrong length (31 bytes)
        let short = "00".repeat(31);
        assert!(hash_from_hex(&short).is_err());
    }
}
