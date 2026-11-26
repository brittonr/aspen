/// Collision-free identity material for simulation refs.
///
/// The hasher is keyed by a domain string (BLAKE3 `derive_key`), and every field is
/// written with a fixed-width big-endian length prefix, so no field value can imitate
/// a field or list boundary. Lists are prefixed by their element count, and optional
/// fields by an explicit presence tag.
pub(crate) struct FramedDigest {
    hasher: blake3::Hasher,
}

const ABSENT_TAG: &[u8] = b"none";
const PRESENT_TAG: &[u8] = b"some";

impl FramedDigest {
    pub(crate) fn new(domain: &'static str) -> Self {
        Self {
            hasher: blake3::Hasher::new_derive_key(domain),
        }
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) {
        let length = u64::try_from(value.len()).unwrap_or(u64::MAX);
        self.hasher.update(&length.to_be_bytes());
        self.hasher.update(value);
    }

    pub(crate) fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(crate) fn number(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    pub(crate) fn count(&mut self, value: usize) {
        self.number(u64::try_from(value).unwrap_or(u64::MAX));
    }

    pub(crate) fn optional_text(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.bytes(PRESENT_TAG);
                self.text(value);
            }
            None => self.bytes(ABSENT_TAG),
        }
    }

    pub(crate) fn optional_number(&mut self, value: Option<u64>) {
        match value {
            Some(value) => {
                self.bytes(PRESENT_TAG);
                self.number(value);
            }
            None => self.bytes(ABSENT_TAG),
        }
    }

    pub(crate) fn content_ref(self) -> String {
        format!("blake3:{}", self.hasher.finalize().to_hex())
    }
}
