//! Core types for the Forge module.
//!
//! This module defines the fundamental data structures used throughout Forge,
//! including the `SignedObject` wrapper for content-addressed, authenticated objects.

use iroh::PublicKey;
use serde::{Deserialize, Serialize};

use super::error::{ForgeError, ForgeResult};

/// A content-addressed, cryptographically signed object.
///
/// All immutable objects in Forge (Git objects, COB changes, attestations) are
/// wrapped in this structure, which provides:
///
/// - **Content addressing**: The BLAKE3 hash of the serialized object serves as its ID
/// - **Authentication**: Ed25519 signature from the author
/// - **Timestamping**: Unix timestamp of when the object was created
///
/// # Type Parameter
///
/// - `T`: The payload type, which must implement `Serialize` and `Deserialize`
///
/// # Example
///
/// ```ignore
/// use aspen::forge::types::SignedObject;
///
/// let payload = MyPayload { /* ... */ };
/// let signed = SignedObject::sign(payload, &secret_key)?;
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

    /// Unix timestamp in milliseconds when this object was created.
    pub timestamp_ms: u64,

    /// Ed25519 signature over the payload + author + timestamp.
    pub signature: Signature,
}

/// Ed25519 signature (64 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SignatureVisitor;

        impl<'de> serde::de::Visitor<'de> for SignatureVisitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("64 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 64 {
                    return Err(E::invalid_length(v.len(), &self));
                }
                let mut arr = [0u8; 64];
                arr.copy_from_slice(v);
                Ok(Signature(arr))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 64];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature(arr))
            }
        }

        deserializer.deserialize_bytes(SignatureVisitor)
    }
}

impl Signature {
    /// Create a signature from raw bytes.
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    /// Get the raw signature bytes.
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
}

impl<T: Serialize> SignedObject<T> {
    /// Create a new signed object.
    ///
    /// Signs the payload with the provided secret key and sets the current timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn new(payload: T, secret_key: &iroh::SecretKey) -> ForgeResult<Self> {
        let author = secret_key.public();
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_millis() as u64;

        // Serialize the signing data (payload + author + timestamp)
        let signing_data = Self::signing_data(&payload, &author, timestamp_ms)?;

        // Sign with Ed25519
        let sig = secret_key.sign(&signing_data);
        let signature = Signature::from_bytes(sig.to_bytes());

        Ok(Self {
            payload,
            author,
            timestamp_ms,
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
    fn signing_data(payload: &T, author: &PublicKey, timestamp_ms: u64) -> ForgeResult<Vec<u8>> {
        #[derive(Serialize)]
        struct SigningData<'a, T> {
            payload: &'a T,
            author: &'a PublicKey,
            timestamp_ms: u64,
        }

        let data = SigningData {
            payload,
            author,
            timestamp_ms,
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
        let signing_data = Self::signing_data(&self.payload, &self.author, self.timestamp_ms)?;

        let sig = iroh::Signature::from_bytes(self.signature.as_bytes());

        self.author.verify(&signing_data, &sig).map_err(|e| {
            ForgeError::InvalidSignature {
                message: e.to_string(),
            }
        })
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
    use super::*;

    #[test]
    fn test_signed_object_roundtrip() {
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        struct TestPayload {
            message: String,
            value: u64,
        }

        let payload = TestPayload {
            message: "hello".to_string(),
            value: 42,
        };

        let signed = SignedObject::new(payload.clone(), &secret_key).expect("should sign");

        // Verify signature
        signed.verify().expect("signature should be valid");

        // Serialize and deserialize
        let bytes = signed.to_bytes();
        let recovered: SignedObject<TestPayload> =
            SignedObject::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(recovered.payload, payload);
        assert_eq!(recovered.author, secret_key.public());

        // Verify recovered signature
        recovered.verify().expect("recovered signature should be valid");

        // Hash should be deterministic
        assert_eq!(signed.hash(), recovered.hash());
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
