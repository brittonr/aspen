//! Token-bucket rate limiter operations.
//!
//! State stored as JSON at `__coord:rl:{key}`.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::RateLimiterResultResponse;
use aspen_wasm_guest_sdk::host::current_time_ms;
use serde::Deserialize;
use serde::Serialize;

use crate::kv;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BucketState {
    tokens: f64,
    last_refill_ms: u64,
    capacity: u64,
    refill_rate: f64,
}

fn rl_key(key: &str) -> String {
    format!("__coord:rl:{key}")
}

fn rl_ok(remaining: u64) -> ClientRpcResponse {
    ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
        is_success: true,
        tokens_remaining: Some(remaining),
        retry_after_ms: None,
        error: None,
    })
}

fn rl_denied(remaining: u64, retry_after_ms: u64) -> ClientRpcResponse {
    ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
        is_success: false,
        tokens_remaining: Some(remaining),
        retry_after_ms: Some(retry_after_ms),
        error: None,
    })
}

fn rl_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::RateLimiterResult(RateLimiterResultResponse {
        is_success: false,
        tokens_remaining: None,
        retry_after_ms: None,
        error: Some(msg),
    })
}

fn refill(state: &mut BucketState) {
    let now = current_time_ms();
    if now > state.last_refill_ms {
        let elapsed_secs = (now - state.last_refill_ms) as f64 / 1000.0;
        let added = elapsed_secs * state.refill_rate;
        state.tokens = (state.tokens + added).min(state.capacity as f64);
        state.last_refill_ms = now;
    }
}

pub fn handle_try_acquire(key: String, tokens: u64, capacity: u64, refill_rate: f64) -> ClientRpcResponse {
    let k = rl_key(&key);
    match kv::cas_loop_json::<BucketState, _>(&k, |current| {
        let mut state = current.unwrap_or(BucketState {
            tokens: capacity as f64,
            last_refill_ms: current_time_ms(),
            capacity,
            refill_rate,
        });
        refill(&mut state);
        if state.tokens >= tokens as f64 {
            state.tokens -= tokens as f64;
            Ok(state)
        } else {
            Err(format!("DENIED:{:.0}", state.tokens))
        }
    }) {
        Ok(state) => rl_ok(state.tokens as u64),
        Err(e) if e.starts_with("DENIED:") => {
            let remaining = e.trim_start_matches("DENIED:").parse::<f64>().unwrap_or(0.0) as u64;
            let deficit = tokens.saturating_sub(remaining);
            let retry_ms = if refill_rate > 0.0 {
                ((deficit as f64 / refill_rate) * 1000.0) as u64
            } else {
                0
            };
            rl_denied(remaining, retry_ms)
        }
        Err(e) => rl_err(e),
    }
}

pub fn handle_acquire(key: String, tokens: u64, capacity: u64, refill_rate: f64, timeout_ms: u64) -> ClientRpcResponse {
    let deadline = if timeout_ms > 0 {
        current_time_ms() + timeout_ms
    } else {
        u64::MAX
    };

    loop {
        let result = handle_try_acquire(key.clone(), tokens, capacity, refill_rate);
        if let ClientRpcResponse::RateLimiterResult(ref r) = result {
            if r.is_success {
                return result;
            }
        }
        if current_time_ms() > deadline {
            return rl_err("rate limiter acquisition timed out".to_string());
        }
    }
}

pub fn handle_available(key: String, capacity: u64, _refill_rate: f64) -> ClientRpcResponse {
    let k = rl_key(&key);
    match kv::get_json::<BucketState>(&k) {
        Ok(Some(mut state)) => {
            refill(&mut state);
            rl_ok(state.tokens as u64)
        }
        Ok(None) => rl_ok(capacity),
        Err(e) => rl_err(e),
    }
}

pub fn handle_reset(key: String, capacity: u64, refill_rate: f64) -> ClientRpcResponse {
    let k = rl_key(&key);
    let state = BucketState {
        tokens: capacity as f64,
        last_refill_ms: current_time_ms(),
        capacity,
        refill_rate,
    };
    match kv::put_json(&k, &state) {
        Ok(()) => rl_ok(capacity),
        Err(e) => rl_err(e),
    }
}
