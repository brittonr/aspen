//! SOPS metadata types for Aspen Transit key groups.
//!
//! The `[sops]` section in a SOPS-encrypted file contains metadata about
//! how the file was encrypted, including key groups (age, kms, aspen_transit).

use serde::Deserialize;
use serde::Serialize;

use super::sops_constants::SOPS_VERSION;

/// Aspen Transit recipient in SOPS metadata.
///
/// Stored as `[[sops.aspen_transit]]` in the encrypted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
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

/// HashiCorp Vault Transit recipient in SOPS metadata.
///
/// Go SOPS compatible format. Stored as `[[sops.hc_vault_transit]]`.
/// The keyservice bridge translates these into Aspen Transit RPCs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HcVaultTransitRecipient {
    /// Vault server address (unused by keyservice bridge, but required by Go SOPS).
    pub vault_address: String,
    /// Transit engine mount path (e.g., "transit").
    pub engine_path: String,
    /// Transit key name.
    pub key_name: String,
    /// Encrypted data key (Aspen Transit ciphertext: `aspen:v<ver>:<base64>`).
    #[serde(default)]
    pub enc: String,
    /// Creation timestamp (RFC 3339).
    pub created_at: String,
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

/// A key group for Go SOPS 3.7+ `key_groups` format.
///
/// Each key group contains one or more key types. SOPS requires decrypting
/// with at least one key from each key group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SopsKeyGroup {
    /// Vault Transit keys in this group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hc_vault_transit: Vec<HcVaultTransitRecipient>,
    /// Age keys in this group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub age: Vec<AgeRecipient>,
}

/// Complete SOPS file metadata section.
///
/// Represents the `[sops]` table in a SOPS-encrypted TOML file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SopsFileMetadata {
    /// Aspen Transit key groups.
    #[serde(default)]
    pub aspen_transit: Vec<AspenTransitRecipient>,

    /// HashiCorp Vault Transit key groups (Go SOPS interop).
    ///
    /// Populated alongside `aspen_transit` during encryption so Go SOPS can
    /// decrypt via `--keyservice` without understanding `aspen_transit`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hc_vault_transit: Vec<HcVaultTransitRecipient>,

    /// Key groups (Go SOPS 3.7+ format).
    ///
    /// Go SOPS 3.12+ requires keys to be in `key_groups` rather than flat
    /// at the top of the `sops` section. This is populated automatically
    /// from `hc_vault_transit` and `age` entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_groups: Vec<SopsKeyGroup>,

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
            hc_vault_transit: Vec::new(),
            key_groups: Vec::new(),
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

    /// Add a Go SOPS compatible Vault Transit recipient (mirrors an Aspen Transit entry).
    pub fn add_hc_vault_recipient(&mut self, recipient: HcVaultTransitRecipient) {
        if let Some(existing) = self
            .hc_vault_transit
            .iter_mut()
            .find(|r| r.engine_path == recipient.engine_path && r.key_name == recipient.key_name)
        {
            *existing = recipient;
        } else {
            self.hc_vault_transit.push(recipient);
        }
    }

    /// Add a Go SOPS compatible Vault Transit entry that mirrors an Aspen Transit recipient.
    ///
    /// Maps: mount → engine_path, name → key_name, enc → enc.
    /// `vault_address` is set to "aspen" (unused by keyservice bridge).
    pub fn add_hc_vault_mirror(&mut self, aspen: &AspenTransitRecipient) {
        self.add_hc_vault_recipient(HcVaultTransitRecipient {
            vault_address: "aspen".into(),
            engine_path: aspen.mount.clone(),
            key_name: aspen.name.clone(),
            enc: aspen.enc.clone(),
            created_at: self.lastmodified.clone(),
        });
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

    /// Rebuild `key_groups` from `hc_vault_transit` and `age` entries.
    ///
    /// Go SOPS 3.12+ reads `key_groups` instead of flat key arrays.
    /// Call this before serialization to ensure interop.
    pub fn sync_key_groups(&mut self) {
        let group = SopsKeyGroup {
            hc_vault_transit: self.hc_vault_transit.clone(),
            age: self.age.clone(),
        };
        if !group.hc_vault_transit.is_empty() || !group.age.is_empty() {
            self.key_groups = vec![group];
        } else {
            self.key_groups.clear();
        }
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
pub fn extract_metadata_from_toml(value: &toml::Value) -> super::sops_error::Result<Option<SopsFileMetadata>> {
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

/// Legacy alias for `extract_metadata_from_toml`.
pub fn extract_metadata(value: &toml::Value) -> super::sops_error::Result<Option<SopsFileMetadata>> {
    extract_metadata_from_toml(value)
}

use super::sops_error::SopsError;

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
