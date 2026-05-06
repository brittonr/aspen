#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Error returned by Aspen's bincode compatibility wrapper.
pub type Error = bincode::Error;

/// Serialize using Aspen's legacy bincode 1.x storage/wire format.
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    bincode::serialize(value)
}

/// Deserialize using Aspen's legacy bincode 1.x storage/wire format.
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Fixture {
        id: u64,
        payload: Vec<u8>,
    }

    #[test]
    fn preserves_legacy_bincode_vector_layout() {
        let fixture = Fixture {
            id: 7,
            payload: vec![1, 2, 3],
        };
        let bytes = serialize(&fixture).expect("serialize");

        assert_eq!(bytes, vec![7, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3]);
        assert_eq!(deserialize::<Fixture>(&bytes).expect("deserialize"), fixture);
    }
}
