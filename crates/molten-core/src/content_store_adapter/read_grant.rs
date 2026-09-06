use crate::fabric::valid_blake3_ref;

pub const MAX_CONTENT_READERS: usize = 16;

/// A local operator-configured read grant, not a verified UCAN credential.
/// Possession of a manifest, pin, locator, or transport identity cannot create it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentReadGrant {
    policy_ref: String,
    manifest_ref: String,
    reader_keys: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentReadGrantRejection {
    InvalidRef,
    ReaderCount,
    InvalidReader,
    DuplicateReader,
}

impl ContentReadGrant {
    // r[impl molten.content_live.read_grant]
    pub fn from_operator_policy(
        policy_ref: String,
        manifest_ref: String,
        mut reader_keys: Vec<String>,
    ) -> Result<Self, ContentReadGrantRejection> {
        if !valid_blake3_ref(&policy_ref) || !valid_blake3_ref(&manifest_ref) {
            return Err(ContentReadGrantRejection::InvalidRef);
        }
        if reader_keys.is_empty() || reader_keys.len() > MAX_CONTENT_READERS {
            return Err(ContentReadGrantRejection::ReaderCount);
        }
        if reader_keys.iter().any(|key| !valid_reader_key(key)) {
            return Err(ContentReadGrantRejection::InvalidReader);
        }
        reader_keys.sort();
        if reader_keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ContentReadGrantRejection::DuplicateReader);
        }
        Ok(Self {
            policy_ref,
            manifest_ref,
            reader_keys,
        })
    }

    pub fn manifest_ref(&self) -> &str {
        &self.manifest_ref
    }

    pub fn policy_ref(&self) -> &str {
        &self.policy_ref
    }

    pub fn allows(&self, manifest_ref: &str, authenticated_reader: &str) -> bool {
        self.manifest_ref == manifest_ref && self.reader_keys.iter().any(|key| key == authenticated_reader)
    }
}

pub fn valid_reader_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(byte: &str) -> String {
        format!("blake3:{}", byte.repeat(64))
    }

    // r[verify molten.content_live.read_grant]
    #[test]
    fn explicit_grant_is_reader_and_manifest_specific() {
        let manifest = reference("a");
        let reader = "b".repeat(64);
        let grant = ContentReadGrant::from_operator_policy(reference("c"), manifest.clone(), vec![reader.clone()])
            .expect("explicit grant");
        assert!(grant.allows(&manifest, &reader));
        assert!(!grant.allows(&reference("d"), &reader));
        assert!(!grant.allows(&manifest, &"e".repeat(64)));
        assert!(!grant.allows(&manifest, ""));
    }

    // r[verify molten.content_live.read_grant]
    #[test]
    fn malformed_duplicate_empty_and_overbound_policy_is_rejected() {
        let admit = |readers| ContentReadGrant::from_operator_policy(reference("c"), reference("a"), readers);
        assert_eq!(admit(vec![]), Err(ContentReadGrantRejection::ReaderCount));
        assert_eq!(admit(vec!["b".repeat(64); MAX_CONTENT_READERS + 1]), Err(ContentReadGrantRejection::ReaderCount));
        assert_eq!(admit(vec!["b".repeat(64); 2]), Err(ContentReadGrantRejection::DuplicateReader));
        for reader in ["b".repeat(63), "B".repeat(64), "z".repeat(64), "".to_string()] {
            assert_eq!(admit(vec![reader]), Err(ContentReadGrantRejection::InvalidReader));
        }
        assert_eq!(
            ContentReadGrant::from_operator_policy("policy".into(), reference("a"), vec!["b".repeat(64)]),
            Err(ContentReadGrantRejection::InvalidRef)
        );
        assert_eq!(
            ContentReadGrant::from_operator_policy(reference("c"), "manifest".into(), vec!["b".repeat(64)]),
            Err(ContentReadGrantRejection::InvalidRef)
        );
    }
}
