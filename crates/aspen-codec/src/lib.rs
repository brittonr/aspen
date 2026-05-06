#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Error returned by Aspen's default storage/wire codec.
pub type Error = postcard::Error;

/// Serialize using Aspen's default compact postcard storage/wire format.
pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    postcard::to_allocvec(value)
}

/// Deserialize using Aspen's default compact postcard storage/wire format.
pub fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    postcard::from_bytes(bytes)
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
    fn uses_compact_postcard_layout() {
        let fixture = Fixture {
            id: 7,
            payload: vec![1, 2, 3],
        };
        let bytes = serialize(&fixture).expect("serialize");

        assert_eq!(bytes, vec![7, 3, 1, 2, 3]);
        assert_eq!(deserialize::<Fixture>(&bytes).expect("deserialize"), fixture);
    }
}
