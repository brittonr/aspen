//! KV helper wrappers for DNS key format.
//!
//! All DNS records are stored under the `dns:` prefix.
//! Records: `dns:{domain}:{record_type}` (e.g., `dns:example.com:A`)
//! Zones:   `dns:_zone:{zone_name}` (e.g., `dns:_zone:example.com`)

use aspen_wasm_guest_sdk::host;

/// KV prefix for DNS records.
pub const DNS_KEY_PREFIX: &str = "dns:";

/// KV prefix for DNS zone metadata.
pub const DNS_ZONE_PREFIX: &str = "dns:_zone:";

/// Maximum records in a batch/scan.
pub const MAX_BATCH_SIZE: u32 = 100;

/// Maximum zones per cluster.
pub const MAX_ZONES: u32 = 10_000;

/// Build the KV key for a DNS record.
pub fn record_key(domain: &str, record_type: &str) -> String {
    format!("{DNS_KEY_PREFIX}{domain}:{record_type}")
}

/// Build the KV key for a zone.
pub fn zone_key(zone_name: &str) -> String {
    format!("{DNS_ZONE_PREFIX}{zone_name}")
}

/// Read a JSON value from KV.
pub fn kv_get_json<T: serde::de::DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    match host::kv_get_value(key)? {
        Some(bytes) => {
            let val: T =
                serde_json::from_slice(&bytes).map_err(|e| format!("json decode error for key '{key}': {e}"))?;
            Ok(Some(val))
        }
        None => Ok(None),
    }
}

/// Write a JSON value to KV.
pub fn kv_put_json<T: serde::Serialize>(key: &str, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| format!("json encode error: {e}"))?;
    host::kv_put_value(key, &bytes)
}

/// Delete a key from KV.
pub fn kv_delete(key: &str) -> Result<(), String> {
    host::kv_delete_key(key)
}

/// Scan KV keys by prefix, returning raw (key, value-bytes) pairs.
pub fn kv_scan_raw(prefix: &str, limit: u32) -> Result<Vec<(String, Vec<u8>)>, String> {
    host::kv_scan_prefix(prefix, limit)
}

/// Check if a domain is a wildcard (`*.example.com`).
pub fn is_wildcard_domain(domain: &str) -> bool {
    domain.starts_with("*.")
}

/// Compute the wildcard parent for a domain.
///
/// `foo.example.com` → `*.example.com`
/// `bar.sub.example.com` → `*.sub.example.com`
pub fn wildcard_parent(domain: &str) -> Option<String> {
    domain.find('.').map(|pos| format!("*{}", &domain[pos..]))
}
