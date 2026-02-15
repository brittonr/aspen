// Vault response types.

use serde::Deserialize;
use serde::Serialize;

/// List vaults response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultListResponse {
    pub vaults: Vec<VaultInfo>,
    pub error: Option<String>,
}

/// Information about a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub name: String,
    pub key_count: u64,
}

/// Vault keys response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeysResponse {
    pub vault: String,
    pub keys: Vec<VaultKeyValue>,
    pub error: Option<String>,
}

/// Key-value pair within a vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKeyValue {
    pub key: String,
    pub value: String,
}
