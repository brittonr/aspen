//! Sequence (monotonic ID) operations.
//!
//! Stored as decimal `u64` string at `__coord:seq:{key}`.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::SequenceResultResponse;

use crate::kv;

fn seq_key(key: &str) -> String {
    format!("__coord:seq:{key}")
}

fn seq_ok(value: u64) -> ClientRpcResponse {
    ClientRpcResponse::SequenceResult(SequenceResultResponse {
        is_success: true,
        value: Some(value),
        error: None,
    })
}

fn seq_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::SequenceResult(SequenceResultResponse {
        is_success: false,
        value: None,
        error: Some(msg),
    })
}

fn parse_u64(s: Option<String>) -> Result<u64, String> {
    match s {
        Some(v) => v.parse::<u64>().map_err(|e| format!("corrupt sequence: {e}")),
        None => Ok(0),
    }
}

pub fn handle_next(key: String) -> ClientRpcResponse {
    let k = seq_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        Ok((val + 1).to_string())
    }) {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => seq_ok(v),
            Err(e) => seq_err(format!("parse error: {e}")),
        },
        Err(e) => seq_err(e),
    }
}

pub fn handle_reserve(key: String, count: u64) -> ClientRpcResponse {
    let k = seq_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        Ok((val + count).to_string())
    }) {
        // Return the start of the reserved range
        Ok(s) => match s.parse::<u64>() {
            Ok(end) => seq_ok(end - count + 1),
            Err(e) => seq_err(format!("parse error: {e}")),
        },
        Err(e) => seq_err(e),
    }
}

pub fn handle_current(key: String) -> ClientRpcResponse {
    let k = seq_key(&key);
    match kv::get_string(&k) {
        Ok(v) => match parse_u64(v) {
            Ok(val) => seq_ok(val),
            Err(e) => seq_err(e),
        },
        Err(e) => seq_err(e),
    }
}
