use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::*;

/// The version byte of the catalog encoding.
pub const CATALOG_VERSION: u8 = 1;
/// The most published heads one catalog holds, matching the Redb store.
pub const MAX_CATALOG_HEADS: u32 = 1_024;
/// The most pins one catalog holds, matching the Redb store.
pub const MAX_CATALOG_PINS: u32 = 65_536;
const MAX_MAP_ID_BYTES: usize = 96;
const MAX_HEAD_BYTES: usize = 1_024;

/// The mutable state of one store, written atomically as one object.
///
/// Heads are opaque encoded roots keyed by map identifier; the shell owns the
/// root encoding. Publication, pin, and deletion batches rewrite the catalog
/// in the same marble batch as their blocks, so a crash leaves either the old
/// or the new catalog, never a mixture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarbleCatalog {
    pub heads: BTreeMap<String, Vec<u8>>,
    pub pins: BTreeSet<Digest>,
    pub retention_generation: u64,
}

/// Encode a catalog canonically: sorted heads and pins, bounded sizes.
pub fn encode_catalog(catalog: &MarbleCatalog) -> Result<Vec<u8>, MarbleStoreIssue> {
    let head_count = bounded_count(catalog.heads.len(), MAX_CATALOG_HEADS, "head-count")?;
    let pin_count = bounded_count(catalog.pins.len(), MAX_CATALOG_PINS, "pin-count")?;
    let mut bytes = Vec::with_capacity(1 + 8 + 4 + 4 + catalog.pins.len() * DIGEST_BYTES);
    bytes.push(CATALOG_VERSION);
    bytes.extend_from_slice(&catalog.retention_generation.to_le_bytes());
    bytes.extend_from_slice(&head_count.to_le_bytes());
    for (map_id, head) in &catalog.heads {
        push_field(&mut bytes, map_id.as_bytes(), MAX_MAP_ID_BYTES, "map-id-length")?;
        push_field(&mut bytes, head, MAX_HEAD_BYTES, "head-length")?;
    }
    bytes.extend_from_slice(&pin_count.to_le_bytes());
    for pin in &catalog.pins {
        bytes.extend_from_slice(pin);
    }
    Ok(bytes)
}

/// Decode a catalog, refusing any non-canonical or out-of-bound encoding.
pub fn decode_catalog(bytes: &[u8]) -> Result<MarbleCatalog, MarbleStoreIssue> {
    let mut reader = Reader { rest: bytes };
    if reader.take_array::<1>()? != [CATALOG_VERSION] {
        return Err(MarbleStoreIssue::InvalidCatalog("version"));
    }
    let retention_generation = u64::from_le_bytes(reader.take_array()?);
    let head_count = u32::from_le_bytes(reader.take_array()?);
    if head_count > MAX_CATALOG_HEADS {
        return Err(MarbleStoreIssue::InvalidCatalog("head-count"));
    }
    let mut heads = BTreeMap::new();
    for _ in 0..head_count {
        let map_id = std::str::from_utf8(reader.take_field(MAX_MAP_ID_BYTES)?)
            .map_err(|_| MarbleStoreIssue::InvalidCatalog("map-id-utf8"))?
            .to_owned();
        let head = reader.take_field(MAX_HEAD_BYTES)?.to_vec();
        if heads.last_key_value().is_some_and(|(last, _)| *last >= map_id) {
            return Err(MarbleStoreIssue::InvalidCatalog("head-order"));
        }
        heads.insert(map_id, head);
    }
    let pin_count = u32::from_le_bytes(reader.take_array()?);
    if pin_count > MAX_CATALOG_PINS {
        return Err(MarbleStoreIssue::InvalidCatalog("pin-count"));
    }
    let mut pins = BTreeSet::new();
    for _ in 0..pin_count {
        let pin = reader.take_array::<DIGEST_BYTES>()?;
        if pins.last().is_some_and(|last| *last >= pin) {
            return Err(MarbleStoreIssue::InvalidCatalog("pin-order"));
        }
        pins.insert(pin);
    }
    if !reader.rest.is_empty() {
        return Err(MarbleStoreIssue::InvalidCatalog("trailing-bytes"));
    }
    Ok(MarbleCatalog {
        heads,
        pins,
        retention_generation,
    })
}

fn bounded_count(length: usize, maximum: u32, field: &'static str) -> Result<u32, MarbleStoreIssue> {
    u32::try_from(length)
        .ok()
        .filter(|count| *count <= maximum)
        .ok_or(MarbleStoreIssue::InvalidCatalog(field))
}

fn push_field(bytes: &mut Vec<u8>, field: &[u8], maximum: usize, name: &'static str) -> Result<(), MarbleStoreIssue> {
    let length = u16::try_from(field.len())
        .ok()
        .filter(|length| usize::from(*length) <= maximum)
        .ok_or(MarbleStoreIssue::InvalidCatalog(name))?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(field);
    Ok(())
}

struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], MarbleStoreIssue> {
        let (head, rest) = self.rest.split_first_chunk::<N>().ok_or(MarbleStoreIssue::InvalidCatalog("truncated"))?;
        self.rest = rest;
        Ok(*head)
    }

    fn take_field(&mut self, maximum: usize) -> Result<&'a [u8], MarbleStoreIssue> {
        let length = usize::from(u16::from_le_bytes(self.take_array()?));
        if length > maximum || length > self.rest.len() {
            return Err(MarbleStoreIssue::InvalidCatalog("field-length"));
        }
        let (field, rest) = self.rest.split_at(length);
        self.rest = rest;
        Ok(field)
    }
}
