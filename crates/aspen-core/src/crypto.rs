//! Cryptographic primitives for Aspen.
//!
//! This module provides fundamental cryptographic types used across the Aspen ecosystem.

use serde::{Deserialize, Serialize};

/// Ed25519 signature (64 bytes).
///
/// A wrapper around a 64-byte array representing an Ed25519 signature.
/// Used for signing objects in Forge, federation announcements, and other
/// authenticated data structures.
///
/// # Example
///
/// ```ignore
/// use aspen_core::Signature;
///
/// // Create from raw bytes
/// let sig = Signature::from_bytes([0u8; 64]);
///
/// // Access raw bytes
/// let bytes: &[u8; 64] = sig.as_bytes();
/// ```
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
                    *byte = seq.next_element()?.ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_roundtrip() {
        let sig = Signature::from_bytes([42u8; 64]);

        // Serialize with postcard
        let bytes = postcard::to_allocvec(&sig).expect("should serialize");

        // Deserialize
        let recovered: Signature = postcard::from_bytes(&bytes).expect("should deserialize");

        assert_eq!(sig, recovered);
    }

    #[test]
    fn test_signature_json_roundtrip() {
        let sig = Signature::from_bytes([42u8; 64]);

        // Serialize with serde_json
        let json = serde_json::to_string(&sig).expect("should serialize");

        // Deserialize
        let recovered: Signature = serde_json::from_str(&json).expect("should deserialize");

        assert_eq!(sig, recovered);
    }
}
