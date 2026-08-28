pub(super) struct FramedHasher {
    hasher: blake3::Hasher,
}

impl FramedHasher {
    pub(super) fn new(domain: &'static str) -> Self {
        Self {
            hasher: blake3::Hasher::new_derive_key(domain),
        }
    }

    pub(super) fn text(&mut self, label: &str, value: &str) {
        self.bytes(label, value.as_bytes());
    }

    pub(super) fn optional_text(&mut self, label: &str, value: Option<&str>) {
        match value {
            Some(value) => {
                self.text(label, "some");
                self.text("optional-value", value);
            }
            None => self.text(label, "none"),
        }
    }

    pub(super) fn number(&mut self, label: &str, value: u64) {
        self.bytes(label, &value.to_le_bytes());
    }

    fn bytes(&mut self, label: &str, value: &[u8]) {
        self.hasher.update(label.len().to_string().as_bytes());
        self.hasher.update(b":");
        self.hasher.update(label.as_bytes());
        self.hasher.update(value.len().to_string().as_bytes());
        self.hasher.update(b":");
        self.hasher.update(value);
    }

    pub(super) fn finish(self) -> String {
        format!("blake3:{}", self.hasher.finalize().to_hex())
    }
}
