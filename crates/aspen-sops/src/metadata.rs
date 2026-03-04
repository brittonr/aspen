//! SOPS metadata types for Aspen Transit key groups.
//!
//! The `[sops]` section in a SOPS-encrypted file contains metadata about
//! how the file was encrypted, including key groups (age, kms, aspen_transit).

use serde::Deserialize;
use serde::Serialize;

use crate::constants::SOPS_VERSION;

/// Aspen Transit recipient in SOPS metadata.
///
/// Stored as `[[sops.aspen_transit]]` in the encrypted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AspenTransitRecipient {
    /// Aspen cluster ticket for connecting to the Transit engine.
    pub cluster_ticket: String,
    /// Transit mount point (default: "transit").
    pub mount: String,
    /// Transit key name used to encrypt the data key.
    pub name: String,
    /// Encrypted data key (`aspen:v<version>:<base64>` format).
    pub enc: String,
    /// Transit key version used for encryption.
    pub key_version: u32,
}

/// Age recipient in SOPS metadata.
///
/// Stored as `[[sops.age]]` in the encrypted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgeRecipient {
    /// Age public key (recipient).
    pub recipient: String,
    /// Encrypted data key (armored age ciphertext).
    #[serde(default)]
    pub enc: Option<String>,
}

/// Complete SOPS file metadata section.
///
/// Represents the `[sops]` table in a SOPS-encrypted TOML file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopsFileMetadata {
    /// Aspen Transit key groups.
    #[serde(default)]
    pub aspen_transit: Vec<AspenTransitRecipient>,

    /// Age key groups.
    #[serde(default)]
    pub age: Vec<AgeRecipient>,

    /// Timestamp when the file was last modified.
    #[serde(default)]
    pub lastmodified: String,

    /// Encrypted MAC (HMAC-SHA256 over all plaintext values).
    #[serde(default)]
    pub mac: String,

    /// SOPS version that created the file.
    #[serde(default = "default_sops_version")]
    pub version: String,

    /// Regex pattern — only matching key paths are encrypted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_regex: Option<String>,
}

fn default_sops_version() -> String {
    SOPS_VERSION.to_string()
}

impl SopsFileMetadata {
    /// Create new metadata with no key groups.
    pub fn new() -> Self {
        Self {
            aspen_transit: Vec::new(),
            age: Vec::new(),
            lastmodified: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            mac: String::new(),
            version: SOPS_VERSION.to_string(),
            encrypted_regex: None,
        }
    }

    /// Check if any Aspen Transit key groups exist.
    pub fn has_aspen_transit(&self) -> bool {
        !self.aspen_transit.is_empty()
    }

    /// Check if any age key groups exist.
    pub fn has_age(&self) -> bool {
        !self.age.is_empty()
    }

    /// Find an Aspen Transit recipient by cluster ticket.
    pub fn find_aspen_recipient(&self, cluster_ticket: &str) -> Option<&AspenTransitRecipient> {
        self.aspen_transit.iter().find(|r| r.cluster_ticket == cluster_ticket)
    }

    /// Find a mutable Aspen Transit recipient by cluster ticket.
    pub fn find_aspen_recipient_mut(&mut self, cluster_ticket: &str) -> Option<&mut AspenTransitRecipient> {
        self.aspen_transit.iter_mut().find(|r| r.cluster_ticket == cluster_ticket)
    }

    /// Add an Aspen Transit recipient.
    pub fn add_aspen_recipient(&mut self, recipient: AspenTransitRecipient) {
        // Replace existing recipient for the same cluster, or append
        if let Some(existing) = self.aspen_transit.iter_mut().find(|r| r.cluster_ticket == recipient.cluster_ticket) {
            *existing = recipient;
        } else {
            self.aspen_transit.push(recipient);
        }
    }

    /// Remove an Aspen Transit recipient by cluster ticket.
    pub fn remove_aspen_recipient(&mut self, cluster_ticket: &str) {
        self.aspen_transit.retain(|r| r.cluster_ticket != cluster_ticket);
    }

    /// Add an age recipient.
    pub fn add_age_recipient(&mut self, recipient: AgeRecipient) {
        if let Some(existing) = self.age.iter_mut().find(|r| r.recipient == recipient.recipient) {
            *existing = recipient;
        } else {
            self.age.push(recipient);
        }
    }

    /// Remove an age recipient by public key.
    pub fn remove_age_recipient(&mut self, public_key: &str) {
        self.age.retain(|r| r.recipient != public_key);
    }

    /// Update the lastmodified timestamp to now.
    pub fn touch(&mut self) {
        self.lastmodified = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
}

impl Default for SopsFileMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract SOPS metadata from a parsed TOML value.
///
/// Returns `None` if the `[sops]` table is not present.
pub fn extract_metadata(value: &toml::Value) -> crate::error::Result<Option<SopsFileMetadata>> {
    let table = match value.as_table() {
        Some(t) => t,
        None => return Ok(None),
    };

    let sops_value = match table.get("sops") {
        Some(v) => v,
        None => return Ok(None),
    };

    let metadata: SopsFileMetadata = sops_value
        .clone()
        .try_into()
        .map_err(|e: toml::de::Error| SopsError::InvalidMetadata { reason: e.to_string() })?;

    Ok(Some(metadata))
}

use crate::error::SopsError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_metadata() {
        let meta = SopsFileMetadata::new();
        assert!(!meta.has_aspen_transit());
        assert!(!meta.has_age());
        assert_eq!(meta.version, SOPS_VERSION);
    }

    #[test]
    fn test_add_remove_aspen_recipient() {
        let mut meta = SopsFileMetadata::new();

        let recipient = AspenTransitRecipient {
            cluster_ticket: "aspen1qtest".into(),
            mount: "transit".into(),
            name: "sops-key".into(),
            enc: "aspen:v1:encrypted".into(),
            key_version: 1,
        };

        meta.add_aspen_recipient(recipient.clone());
        assert!(meta.has_aspen_transit());
        assert_eq!(meta.aspen_transit.len(), 1);

        // Adding same cluster_ticket replaces
        let updated = AspenTransitRecipient {
            enc: "aspen:v2:updated".into(),
            key_version: 2,
            ..recipient.clone()
        };
        meta.add_aspen_recipient(updated);
        assert_eq!(meta.aspen_transit.len(), 1);
        assert_eq!(meta.aspen_transit[0].key_version, 2);

        meta.remove_aspen_recipient("aspen1qtest");
        assert!(!meta.has_aspen_transit());
    }

    #[test]
    fn test_find_aspen_recipient() {
        let mut meta = SopsFileMetadata::new();
        meta.add_aspen_recipient(AspenTransitRecipient {
            cluster_ticket: "ticket-a".into(),
            mount: "transit".into(),
            name: "key-a".into(),
            enc: "aspen:v1:a".into(),
            key_version: 1,
        });
        meta.add_aspen_recipient(AspenTransitRecipient {
            cluster_ticket: "ticket-b".into(),
            mount: "transit".into(),
            name: "key-b".into(),
            enc: "aspen:v1:b".into(),
            key_version: 1,
        });

        assert!(meta.find_aspen_recipient("ticket-a").is_some());
        assert!(meta.find_aspen_recipient("ticket-b").is_some());
        assert!(meta.find_aspen_recipient("ticket-c").is_none());
    }

    #[test]
    fn test_metadata_serialization_roundtrip() {
        let mut meta = SopsFileMetadata::new();
        meta.lastmodified = "2026-03-03T21:00:00Z".into();
        meta.mac = "ENC[AES256_GCM,data:abc,iv:def,tag:ghi,type:str]".into();

        meta.add_aspen_recipient(AspenTransitRecipient {
            cluster_ticket: "aspen1qtest".into(),
            mount: "transit".into(),
            name: "sops-data-key".into(),
            enc: "aspen:v3:c2VjcmV0".into(),
            key_version: 3,
        });

        meta.add_age_recipient(AgeRecipient {
            recipient: "age1abc".into(),
            enc: Some("-----BEGIN AGE ENCRYPTED FILE-----\ndata".into()),
        });

        // Wrap in [sops] table for TOML serialization
        let wrapper = toml::Value::Table({
            let mut t = toml::map::Map::new();
            t.insert("sops".into(), toml::Value::try_from(&meta).expect("serialize"));
            t
        });

        let toml_str = toml::to_string_pretty(&wrapper).expect("to toml");

        // Verify it contains the expected structure
        assert!(toml_str.contains("[[sops.aspen_transit]]"));
        assert!(toml_str.contains("cluster_ticket = \"aspen1qtest\""));
        assert!(toml_str.contains("[[sops.age]]"));
        assert!(toml_str.contains("recipient = \"age1abc\""));

        // Roundtrip: parse back
        let parsed: toml::Value = toml::from_str(&toml_str).expect("parse");
        let roundtrip = extract_metadata(&parsed).expect("extract").expect("has sops");
        assert_eq!(roundtrip.aspen_transit.len(), 1);
        assert_eq!(roundtrip.aspen_transit[0].key_version, 3);
        assert_eq!(roundtrip.age.len(), 1);
    }

    #[test]
    fn test_metadata_mixed_key_groups() {
        let toml_str = r#"
            [sops]
            lastmodified = "2026-03-03T21:00:00Z"
            mac = "ENC[AES256_GCM,data:test,iv:test,tag:test,type:str]"
            version = "3.9.0"

            [[sops.aspen_transit]]
            cluster_ticket = "aspen1qfirst"
            mount = "transit"
            name = "sops-key"
            enc = "aspen:v1:first"
            key_version = 1

            [[sops.aspen_transit]]
            cluster_ticket = "aspen1qsecond"
            mount = "transit"
            name = "sops-key"
            enc = "aspen:v2:second"
            key_version = 2

            [[sops.age]]
            recipient = "age1fallback"
            enc = "armored-data"
        "#;

        let parsed: toml::Value = toml::from_str(toml_str).expect("parse");
        let meta = extract_metadata(&parsed).expect("extract").expect("has sops");

        assert_eq!(meta.aspen_transit.len(), 2);
        assert_eq!(meta.age.len(), 1);
        assert_eq!(meta.aspen_transit[0].cluster_ticket, "aspen1qfirst");
        assert_eq!(meta.aspen_transit[1].cluster_ticket, "aspen1qsecond");
    }
}
