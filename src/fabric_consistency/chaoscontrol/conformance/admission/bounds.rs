use std::io::Write;

use serde::de::DeserializeOwned;
use serde::de::DeserializeSeed;
use serde::de::Error;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;

const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 1_000_000;
const MAX_COLLECTION: usize = 65_536;
const MAX_STRING: usize = 4_096;

/// Scan without constructing a JSON tree, then deserialize only after all generic
/// allocation limits have been checked. Typed/profile limits are checked next.
pub(super) fn decode<T: DeserializeOwned>(bytes: &[u8], maximum: u64) -> Result<T, super::ImportIssue> {
    let maximum = usize::try_from(maximum).map_err(|_| super::ImportIssue::Bounds)?;
    if bytes.len() > maximum {
        return Err(super::ImportIssue::Bounds);
    }
    let mut nodes = MAX_NODES;
    let mut de = serde_json::Deserializer::from_slice(bytes);
    Scan {
        depth: 0,
        nodes: &mut nodes,
    }
    .deserialize(&mut de)
    .map_err(|_| super::ImportIssue::Decode)?;
    de.end().map_err(|_| super::ImportIssue::Decode)?;
    serde_json::from_slice(bytes).map_err(|_| super::ImportIssue::Decode)
}

struct Scan<'a> {
    depth: usize,
    nodes: &'a mut usize,
}
impl<'de> DeserializeSeed<'de> for Scan<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, de: D) -> Result<(), D::Error> {
        if self.depth > MAX_DEPTH || *self.nodes == 0 {
            return Err(D::Error::custom("structure bound"));
        }
        *self.nodes -= 1;
        de.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Scan<'_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bounded JSON")
    }
    fn visit_bool<E: Error>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E: Error>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E: Error>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E: Error>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_none<E: Error>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<(), E> {
        if value.len() > MAX_STRING {
            Err(E::custom("string bound"))
        } else {
            Ok(())
        }
    }
    fn visit_string<E: Error>(self, value: String) -> Result<(), E> {
        self.visit_str(&value)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        for count in 0..=MAX_COLLECTION {
            if seq
                .next_element_seed(Scan {
                    depth: self.depth + 1,
                    nodes: self.nodes,
                })?
                .is_none()
            {
                return Ok(());
            }
            if count == MAX_COLLECTION {
                return Err(A::Error::custom("collection bound"));
            }
        }
        Err(A::Error::custom("collection bound"))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        for count in 0..=MAX_COLLECTION {
            if map
                .next_key_seed(Scan {
                    depth: self.depth + 1,
                    nodes: self.nodes,
                })?
                .is_none()
            {
                return Ok(());
            }
            if count == MAX_COLLECTION {
                return Err(A::Error::custom("collection bound"));
            }
            map.next_value_seed(Scan {
                depth: self.depth + 1,
                nodes: self.nodes,
            })?;
        }
        Err(A::Error::custom("collection bound"))
    }
}

/// Count serialized bytes without allocating another whole-bundle buffer.
pub(super) fn encoded<T: serde::Serialize>(value: &T, maximum: u64) -> Result<(), super::ImportIssue> {
    let maximum = usize::try_from(maximum).map_err(|_| super::ImportIssue::Bounds)?;
    struct Counter {
        remaining: usize,
    }
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.remaining =
                self.remaining.checked_sub(bytes.len()).ok_or_else(|| std::io::Error::other("encoded bound"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter { remaining: maximum }, value).map_err(|_| super::ImportIssue::Bounds)
}

/// Use the codec's token reader without building a tree. The ordinary canonical
/// decoder is invoked only after its recursion and node allocations are bounded.
pub(super) fn command(bytes: &[u8], expected_ref: &str) -> Result<(), super::ImportIssue> {
    const COMMAND_NODES: u64 = 8_192;
    crate::preserves_rail::packed_bounds::preflight(bytes, COMMAND_NODES).map_err(|_| super::ImportIssue::Bounds)?;
    crate::codec::strict_canonical_decode_with_ref(bytes, expected_ref, "chaoscontrol canonical command")
        .map_err(|_| super::ImportIssue::Identity)?;
    Ok(())
}
