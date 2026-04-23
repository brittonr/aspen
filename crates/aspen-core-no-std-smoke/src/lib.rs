#![no_std]

extern crate alloc;

use alloc::string::String;
use aspen_core::KeyValueStore;
use aspen_core::KvEntry;
use aspen_core::NodeId;
use aspen_core::decode_continuation_token;
use aspen_core::encode_continuation_token;
use aspen_core::normalize_scan_limit;
use aspen_core::validate_client_key;

const SMOKE_NODE_ID_RAW: u64 = 7;
const REQUESTED_SCAN_LIMIT: u32 = 5;
const DEFAULT_SCAN_LIMIT: u32 = 32;
const MAX_SCAN_LIMIT: u32 = 1024;
const CLIENT_KEY: &str = "tenant/config";
const ENTRY_VALUE: &str = "value";
const ENTRY_VERSION: i64 = 1;
const ENTRY_CREATE_REVISION: i64 = 10;
const ENTRY_MOD_REVISION: i64 = 11;

pub fn smoke_surface() -> (NodeId, u32, bool, Option<String>, KvEntry) {
    let node_id = NodeId::new(SMOKE_NODE_ID_RAW);
    let token = encode_continuation_token(CLIENT_KEY);
    let decoded = decode_continuation_token(Some(&token));
    let is_key_valid = validate_client_key(CLIENT_KEY).is_ok();
    let entry = KvEntry {
        value: String::from(ENTRY_VALUE),
        version: ENTRY_VERSION,
        create_revision: ENTRY_CREATE_REVISION,
        mod_revision: ENTRY_MOD_REVISION,
        expires_at_ms: None,
        lease_id: None,
    };
    debug_assert!(is_key_valid, "smoke key should remain valid");
    debug_assert_eq!(decoded.as_deref(), Some(CLIENT_KEY), "smoke token should decode to the source key");
    debug_assert_eq!(entry.value.as_str(), ENTRY_VALUE, "smoke entry should preserve the constant payload");
    (
        node_id,
        normalize_scan_limit(Some(REQUESTED_SCAN_LIMIT), DEFAULT_SCAN_LIMIT, MAX_SCAN_LIMIT),
        is_key_valid,
        decoded,
        entry,
    )
}

pub fn accepts_store<T: KeyValueStore + ?Sized>(_store: &T) {}
