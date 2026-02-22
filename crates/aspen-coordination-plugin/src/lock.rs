//! Distributed lock operations.
//!
//! Lock state stored as JSON in `__coord:lock:{key}`.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::LockResultResponse;
use aspen_wasm_guest_sdk::host::current_time_ms;
use serde::Deserialize;
use serde::Serialize;

use crate::kv;

/// Lock entry stored in KV.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LockEntry {
    holder_id: String,
    fencing_token: u64,
    acquired_at_ms: u64,
    ttl_ms: u64,
    deadline_ms: u64,
}

fn lock_key(key: &str) -> String {
    format!("__coord:lock:{key}")
}

fn lock_ok(fencing_token: u64, holder_id: String, deadline_ms: u64) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        is_success: true,
        fencing_token: Some(fencing_token),
        holder_id: Some(holder_id),
        deadline_ms: Some(deadline_ms),
        error: None,
    })
}

fn lock_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        is_success: false,
        fencing_token: None,
        holder_id: None,
        deadline_ms: None,
        error: Some(msg),
    })
}

fn lock_err_with_holder(msg: String, entry: &LockEntry) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        is_success: false,
        fencing_token: Some(entry.fencing_token),
        holder_id: Some(entry.holder_id.clone()),
        deadline_ms: Some(entry.deadline_ms),
        error: Some(msg),
    })
}

/// Check if a lock entry is expired.
fn is_expired(entry: &LockEntry) -> bool {
    current_time_ms() > entry.deadline_ms
}

pub fn handle_acquire(key: String, holder_id: String, ttl_ms: u64, timeout_ms: Option<u64>) -> ClientRpcResponse {
    let k = lock_key(&key);
    let now = current_time_ms();
    let deadline = timeout_ms.filter(|&t| t > 0).map(|t| now + t).unwrap_or(u64::MAX);

    // CAS retry loop with timeout
    loop {
        if current_time_ms() > deadline {
            return lock_err("lock acquisition timed out".to_string());
        }

        let current_raw = match kv::get_string(&k) {
            Ok(v) => v,
            Err(e) => return lock_err(e),
        };

        // Parse current lock state
        let current: Option<LockEntry> = match &current_raw {
            Some(s) => match serde_json::from_str(s) {
                Ok(entry) => Some(entry),
                Err(e) => return lock_err(format!("corrupt lock state: {e}")),
            },
            None => None,
        };

        // Determine if we can acquire
        let (next_token, can_acquire) = match &current {
            None => (1u64, true),
            Some(entry) if is_expired(entry) => (entry.fencing_token + 1, true),
            Some(entry) if entry.holder_id == holder_id => (entry.fencing_token, true), // Re-entrant
            Some(entry) => {
                // Lock held by someone else — if we have a timeout, retry; otherwise fail
                if timeout_ms.is_none() || timeout_ms == Some(0) {
                    return lock_err_with_holder(format!("lock held by '{}'", entry.holder_id), entry);
                }
                (0, false) // Will retry
            }
        };

        if !can_acquire {
            continue;
        }

        let acq_time = current_time_ms();
        let new_entry = LockEntry {
            holder_id: holder_id.clone(),
            fencing_token: next_token,
            acquired_at_ms: acq_time,
            ttl_ms,
            deadline_ms: acq_time + ttl_ms,
        };

        match kv::cas_json(&k, current_raw.as_deref(), &new_entry) {
            Ok(true) => return lock_ok(next_token, holder_id, new_entry.deadline_ms),
            Ok(false) => continue, // CAS conflict, retry
            Err(e) => return lock_err(e),
        }
    }
}

pub fn handle_try_acquire(key: String, holder_id: String, ttl_ms: u64) -> ClientRpcResponse {
    handle_acquire(key, holder_id, ttl_ms, None)
}

pub fn handle_release(key: String, holder_id: String, fencing_token: u64) -> ClientRpcResponse {
    let k = lock_key(&key);

    let current_raw = match kv::get_string(&k) {
        Ok(v) => v,
        Err(e) => return lock_err(e),
    };

    let entry: LockEntry = match &current_raw {
        None => return lock_err("lock not found".to_string()),
        Some(s) => match serde_json::from_str(s) {
            Ok(e) => e,
            Err(e) => return lock_err(format!("corrupt lock state: {e}")),
        },
    };

    if entry.holder_id != holder_id {
        return lock_err_with_holder(format!("lock held by '{}', not '{holder_id}'", entry.holder_id), &entry);
    }

    if entry.fencing_token != fencing_token {
        return lock_err(format!("fencing token mismatch: expected {}, got {fencing_token}", entry.fencing_token));
    }

    match kv::delete(&k) {
        Ok(()) => lock_ok(entry.fencing_token, holder_id, entry.deadline_ms),
        Err(e) => lock_err(e),
    }
}

pub fn handle_renew(key: String, holder_id: String, ttl_ms: u64, fencing_token: u64) -> ClientRpcResponse {
    let k = lock_key(&key);

    match kv::cas_loop_json::<LockEntry, _>(&k, |current| {
        let entry = current.ok_or_else(|| "lock not found".to_string())?;
        if entry.holder_id != holder_id {
            return Err(format!("lock held by '{}', not '{holder_id}'", entry.holder_id));
        }
        if entry.fencing_token != fencing_token {
            return Err(format!("fencing token mismatch: expected {}, got {fencing_token}", entry.fencing_token));
        }
        let now = current_time_ms();
        Ok(LockEntry {
            holder_id: holder_id.clone(),
            fencing_token: entry.fencing_token,
            acquired_at_ms: entry.acquired_at_ms,
            ttl_ms,
            deadline_ms: now + ttl_ms,
        })
    }) {
        Ok(entry) => lock_ok(entry.fencing_token, entry.holder_id, entry.deadline_ms),
        Err(e) => lock_err(e),
    }
}
