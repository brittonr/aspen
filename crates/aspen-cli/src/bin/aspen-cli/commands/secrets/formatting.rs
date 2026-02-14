//! Output formatting types for secrets commands.
//!
//! Each struct implements `Outputable` for JSON and human-readable display.

use std::collections::HashMap;

use serde_json::json;

use crate::output::Outputable;

pub(crate) struct KvReadOutput {
    pub(crate) success: bool,
    pub(crate) data: Option<HashMap<String, String>>,
    pub(crate) version: Option<u64>,
    pub(crate) error: Option<String>,
}

impl Outputable for KvReadOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "data": self.data,
            "version": self.version,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        let mut output = String::new();
        if let Some(version) = self.version {
            output.push_str(&format!("Version: {}\n", version));
        }
        if let Some(data) = &self.data {
            output.push_str("Data:\n");
            for (k, v) in data {
                output.push_str(&format!("  {}: {}\n", k, v));
            }
        }
        output
    }
}

pub(crate) struct KvWriteOutput {
    pub(crate) success: bool,
    pub(crate) version: Option<u64>,
    pub(crate) error: Option<String>,
}

impl Outputable for KvWriteOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "version": self.version,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(version) = self.version {
            format!("Secret written at version {}", version)
        } else {
            "Secret written".to_string()
        }
    }
}

pub(crate) struct KvListOutput {
    pub(crate) success: bool,
    pub(crate) keys: Vec<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for KvListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "keys": self.keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.keys.is_empty() {
            return "No secrets found".to_string();
        }

        let mut output = format!("Keys ({}):\n", self.keys.len());
        for key in &self.keys {
            output.push_str(&format!("  {}\n", key));
        }
        output
    }
}

pub(crate) struct SimpleSuccessOutput {
    pub(crate) success: bool,
    pub(crate) message: String,
    pub(crate) error: Option<String>,
}

impl Outputable for SimpleSuccessOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "message": self.message,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            format!("Error: {}", err)
        } else {
            self.message.clone()
        }
    }
}

pub(crate) struct TransitEncryptOutput {
    pub(crate) success: bool,
    pub(crate) ciphertext: Option<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for TransitEncryptOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "ciphertext": self.ciphertext,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(ct) = &self.ciphertext {
            format!("Ciphertext: {}", ct)
        } else {
            "Encryption failed".to_string()
        }
    }
}

pub(crate) struct TransitDecryptOutput {
    pub(crate) success: bool,
    pub(crate) plaintext: Option<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for TransitDecryptOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "plaintext": self.plaintext,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(pt) = &self.plaintext {
            format!("Plaintext: {}", pt)
        } else {
            "Decryption failed".to_string()
        }
    }
}

pub(crate) struct TransitSignOutput {
    pub(crate) success: bool,
    pub(crate) signature: Option<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for TransitSignOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "signature": self.signature,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(sig) = &self.signature {
            format!("Signature: {}", sig)
        } else {
            "Signing failed".to_string()
        }
    }
}

pub(crate) struct TransitVerifyOutput {
    pub(crate) success: bool,
    pub(crate) valid: Option<bool>,
    pub(crate) error: Option<String>,
}

impl Outputable for TransitVerifyOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "valid": self.valid,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        match self.valid {
            Some(true) => "Signature is VALID".to_string(),
            Some(false) => "Signature is INVALID".to_string(),
            None => "Verification failed".to_string(),
        }
    }
}

pub(crate) struct TransitListOutput {
    pub(crate) success: bool,
    pub(crate) keys: Vec<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for TransitListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "keys": self.keys,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.keys.is_empty() {
            return "No keys found".to_string();
        }

        let mut output = format!("Keys ({}):\n", self.keys.len());
        for key in &self.keys {
            output.push_str(&format!("  {}\n", key));
        }
        output
    }
}

pub(crate) struct PkiCertificateOutput {
    pub(crate) success: bool,
    pub(crate) certificate: Option<String>,
    pub(crate) private_key: Option<String>,
    pub(crate) serial: Option<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for PkiCertificateOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "certificate": self.certificate,
            "private_key": self.private_key,
            "serial": self.serial,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        let mut output = String::new();
        if let Some(serial) = &self.serial {
            output.push_str(&format!("Serial: {}\n\n", serial));
        }
        if let Some(cert) = &self.certificate {
            output.push_str("Certificate:\n");
            output.push_str(cert);
            output.push('\n');
        }
        if let Some(key) = &self.private_key {
            output.push_str("\nPrivate Key:\n");
            output.push_str(key);
            output.push('\n');
        }
        output
    }
}

pub(crate) struct PkiListOutput {
    pub(crate) success: bool,
    pub(crate) items: Vec<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for PkiListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "items": self.items,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if self.items.is_empty() {
            return "No items found".to_string();
        }

        let mut output = format!("Items ({}):\n", self.items.len());
        for item in &self.items {
            output.push_str(&format!("  {}\n", item));
        }
        output
    }
}

pub(crate) struct NixCacheKeyOutput {
    pub(crate) success: bool,
    pub(crate) public_key: Option<String>,
    pub(crate) error: Option<String>,
}

impl Outputable for NixCacheKeyOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "public_key": self.public_key,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }
        if let Some(public_key) = &self.public_key {
            format!("Public key: {}", public_key)
        } else {
            "Operation completed".to_string()
        }
    }
}

pub(crate) struct NixCacheDeleteOutput {
    pub(crate) success: bool,
    pub(crate) error: Option<String>,
}

impl Outputable for NixCacheDeleteOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            format!("Error: {}", err)
        } else if self.success {
            "Signing key deleted successfully".to_string()
        } else {
            "Failed to delete signing key".to_string()
        }
    }
}

pub(crate) struct NixCacheListOutput {
    pub(crate) success: bool,
    pub(crate) cache_names: Option<Vec<String>>,
    pub(crate) error: Option<String>,
}

impl Outputable for NixCacheListOutput {
    fn to_json(&self) -> serde_json::Value {
        json!({
            "success": self.success,
            "cache_names": self.cache_names,
            "error": self.error
        })
    }

    fn to_human(&self) -> String {
        if let Some(err) = &self.error {
            return format!("Error: {}", err);
        }

        if let Some(cache_names) = &self.cache_names {
            if cache_names.is_empty() {
                "No cache signing keys found".to_string()
            } else {
                let mut output = format!("Cache signing keys ({}):\n", cache_names.len());
                for name in cache_names {
                    output.push_str(&format!("  {}\n", name));
                }
                output
            }
        } else {
            "Failed to list cache signing keys".to_string()
        }
    }
}
