//! Counting semaphore operations.
//!
//! State stored as JSON at `__coord:sem:{name}`.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SemaphoreResultResponse;
use aspen_wasm_guest_sdk::host::current_time_ms;
use serde::Deserialize;
use serde::Serialize;

use crate::kv;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemaphoreState {
    capacity: u32,
    holders: Vec<SemaphoreHolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemaphoreHolder {
    holder_id: String,
    acquired_at_ms: u64,
}

fn sem_key(name: &str) -> String {
    format!("__coord:sem:{name}")
}

fn sem_ok(variant: &str, acquired: u32, available: u32, capacity: u32) -> ClientRpcResponse {
    let resp = SemaphoreResultResponse {
        is_success: true,
        permits_acquired: Some(acquired),
        available: Some(available),
        capacity_permits: Some(capacity),
        retry_after_ms: None,
        error: None,
    };
    match variant {
        "acquire" => ClientRpcResponse::SemaphoreAcquireResult(resp),
        "try_acquire" => ClientRpcResponse::SemaphoreTryAcquireResult(resp),
        "release" => ClientRpcResponse::SemaphoreReleaseResult(resp),
        _ => ClientRpcResponse::SemaphoreStatusResult(resp),
    }
}

fn sem_err(variant: &str, msg: String) -> ClientRpcResponse {
    let resp = SemaphoreResultResponse {
        is_success: false,
        permits_acquired: None,
        available: None,
        capacity_permits: None,
        retry_after_ms: None,
        error: Some(msg),
    };
    match variant {
        "acquire" => ClientRpcResponse::SemaphoreAcquireResult(resp),
        "try_acquire" => ClientRpcResponse::SemaphoreTryAcquireResult(resp),
        "release" => ClientRpcResponse::SemaphoreReleaseResult(resp),
        _ => ClientRpcResponse::SemaphoreStatusResult(resp),
    }
}

pub fn handle_acquire(name: String, holder_id: String, capacity: u64, timeout_ms: u64) -> ClientRpcResponse {
    let capacity = capacity as u32;
    let deadline = if timeout_ms > 0 {
        current_time_ms() + timeout_ms
    } else {
        u64::MAX
    };

    loop {
        let k = sem_key(&name);
        match kv::cas_loop_json::<SemaphoreState, _>(&k, |current| {
            let mut state = current.unwrap_or(SemaphoreState {
                capacity,
                holders: Vec::new(),
            });
            state.capacity = capacity;

            if (state.holders.len() as u32) < state.capacity {
                if !state.holders.iter().any(|h| h.holder_id == holder_id) {
                    state.holders.push(SemaphoreHolder {
                        holder_id: holder_id.clone(),
                        acquired_at_ms: current_time_ms(),
                    });
                }
                Ok(state)
            } else {
                Err("semaphore at capacity".to_string())
            }
        }) {
            Ok(state) => {
                let available = state.capacity.saturating_sub(state.holders.len() as u32);
                return sem_ok("acquire", 1, available, state.capacity);
            }
            Err(e) if e.contains("semaphore at capacity") => {
                if current_time_ms() > deadline {
                    return sem_err("acquire", "semaphore acquisition timed out".to_string());
                }
                continue;
            }
            Err(e) => return sem_err("acquire", e),
        }
    }
}

pub fn handle_try_acquire(name: String, holder_id: String, capacity: u64) -> ClientRpcResponse {
    let capacity = capacity as u32;
    let k = sem_key(&name);
    match kv::cas_loop_json::<SemaphoreState, _>(&k, |current| {
        let mut state = current.unwrap_or(SemaphoreState {
            capacity,
            holders: Vec::new(),
        });
        state.capacity = capacity;

        if (state.holders.len() as u32) < state.capacity {
            if !state.holders.iter().any(|h| h.holder_id == holder_id) {
                state.holders.push(SemaphoreHolder {
                    holder_id: holder_id.clone(),
                    acquired_at_ms: current_time_ms(),
                });
            }
            Ok(state)
        } else {
            Err("semaphore at capacity".to_string())
        }
    }) {
        Ok(state) => {
            let available = state.capacity.saturating_sub(state.holders.len() as u32);
            sem_ok("try_acquire", 1, available, state.capacity)
        }
        Err(e) if e.contains("semaphore at capacity") => {
            let resp = SemaphoreResultResponse {
                is_success: false,
                permits_acquired: Some(0),
                available: Some(0),
                capacity_permits: Some(capacity),
                retry_after_ms: None,
                error: None,
            };
            ClientRpcResponse::SemaphoreTryAcquireResult(resp)
        }
        Err(e) => sem_err("try_acquire", e),
    }
}

pub fn handle_release(name: String, holder_id: String) -> ClientRpcResponse {
    let k = sem_key(&name);
    match kv::cas_loop_json::<SemaphoreState, _>(&k, |current| {
        let mut state = current.ok_or_else(|| "semaphore not found".to_string())?;
        let before = state.holders.len();
        state.holders.retain(|h| h.holder_id != holder_id);
        if state.holders.len() == before {
            return Err(format!("holder '{holder_id}' not found in semaphore"));
        }
        Ok(state)
    }) {
        Ok(state) => {
            let available = state.capacity.saturating_sub(state.holders.len() as u32);
            sem_ok("release", 0, available, state.capacity)
        }
        Err(e) => sem_err("release", e),
    }
}

pub fn handle_status(name: String) -> ClientRpcResponse {
    let k = sem_key(&name);
    match kv::get_json::<SemaphoreState>(&k) {
        Ok(Some(state)) => {
            let available = state.capacity.saturating_sub(state.holders.len() as u32);
            sem_ok("status", 0, available, state.capacity)
        }
        Ok(None) => sem_ok("status", 0, 0, 0),
        Err(e) => sem_err("status", e),
    }
}
