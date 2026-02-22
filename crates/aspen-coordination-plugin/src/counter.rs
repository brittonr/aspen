//! Counter operations (unsigned and signed).
//!
//! Unsigned counters are stored as decimal string representations of `u64`.
//! Signed counters are stored as decimal string representations of `i64`.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::CounterResultResponse;
use aspen_client_api::SignedCounterResultResponse;

use crate::kv;

fn counter_key(key: &str) -> String {
    format!("__coord:counter:{key}")
}

fn signed_counter_key(key: &str) -> String {
    format!("__coord:scounter:{key}")
}

fn counter_ok(value: u64) -> ClientRpcResponse {
    ClientRpcResponse::CounterResult(CounterResultResponse {
        is_success: true,
        value: Some(value),
        error: None,
    })
}

fn counter_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::CounterResult(CounterResultResponse {
        is_success: false,
        value: None,
        error: Some(msg),
    })
}

fn signed_ok(value: i64) -> ClientRpcResponse {
    ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
        is_success: true,
        value: Some(value),
        error: None,
    })
}

fn signed_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
        is_success: false,
        value: None,
        error: Some(msg),
    })
}

fn parse_u64(s: Option<String>) -> Result<u64, String> {
    match s {
        Some(v) => v.parse::<u64>().map_err(|e| format!("corrupt counter: {e}")),
        None => Ok(0),
    }
}

fn parse_i64(s: Option<String>) -> Result<i64, String> {
    match s {
        Some(v) => v.parse::<i64>().map_err(|e| format!("corrupt signed counter: {e}")),
        None => Ok(0),
    }
}

pub fn handle_get(key: String) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::get_string(&k) {
        Ok(v) => match parse_u64(v) {
            Ok(val) => counter_ok(val),
            Err(e) => counter_err(e),
        },
        Err(e) => counter_err(e),
    }
}

pub fn handle_increment(key: String) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        Ok(val.saturating_add(1).to_string())
    }) {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => counter_ok(v),
            Err(e) => counter_err(format!("parse error: {e}")),
        },
        Err(e) => counter_err(e),
    }
}

pub fn handle_decrement(key: String) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        if val == 0 {
            return Err("counter underflow: cannot decrement below zero".to_string());
        }
        Ok(val.saturating_sub(1).to_string())
    }) {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => counter_ok(v),
            Err(e) => counter_err(format!("parse error: {e}")),
        },
        Err(e) => counter_err(e),
    }
}

pub fn handle_add(key: String, amount: u64) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        Ok(val.saturating_add(amount).to_string())
    }) {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => counter_ok(v),
            Err(e) => counter_err(format!("parse error: {e}")),
        },
        Err(e) => counter_err(e),
    }
}

pub fn handle_subtract(key: String, amount: u64) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_u64(current)?;
        if val < amount {
            return Err(format!("counter underflow: {val} < {amount}, cannot subtract"));
        }
        Ok(val.saturating_sub(amount).to_string())
    }) {
        Ok(s) => match s.parse::<u64>() {
            Ok(v) => counter_ok(v),
            Err(e) => counter_err(format!("parse error: {e}")),
        },
        Err(e) => counter_err(e),
    }
}

pub fn handle_set(key: String, value: u64) -> ClientRpcResponse {
    let k = counter_key(&key);
    match kv::put_string(&k, &value.to_string()) {
        Ok(()) => counter_ok(value),
        Err(e) => counter_err(e),
    }
}

pub fn handle_compare_and_set(key: String, expected: u64, new_value: u64) -> ClientRpcResponse {
    let k = counter_key(&key);
    let expected_str = expected.to_string();
    let new_str = new_value.to_string();
    match kv::cas_string(&k, Some(&expected_str), &new_str) {
        Ok(true) => counter_ok(new_value),
        Ok(false) => counter_err(format!("CAS conflict: expected {expected}")),
        Err(e) => counter_err(e),
    }
}

pub fn handle_signed_get(key: String) -> ClientRpcResponse {
    let k = signed_counter_key(&key);
    match kv::get_string(&k) {
        Ok(v) => match parse_i64(v) {
            Ok(val) => signed_ok(val),
            Err(e) => signed_err(e),
        },
        Err(e) => signed_err(e),
    }
}

pub fn handle_signed_add(key: String, amount: i64) -> ClientRpcResponse {
    let k = signed_counter_key(&key);
    match kv::cas_loop_string(&k, |current| {
        let val = parse_i64(current)?;
        Ok(val.saturating_add(amount).to_string())
    }) {
        Ok(s) => match s.parse::<i64>() {
            Ok(v) => signed_ok(v),
            Err(e) => signed_err(format!("parse error: {e}")),
        },
        Err(e) => signed_err(e),
    }
}
