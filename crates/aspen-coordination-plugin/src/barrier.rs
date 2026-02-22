//! Barrier (multi-party synchronization) operations.
//!
//! State stored as JSON at `__coord:barrier:{name}`.

use aspen_client_api::BarrierResultResponse;
use aspen_client_api::ClientRpcResponse;
use serde::Deserialize;
use serde::Serialize;

use crate::kv;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BarrierState {
    participants: Vec<String>,
    required_count: u32,
}

fn barrier_key(name: &str) -> String {
    format!("__coord:barrier:{name}")
}

fn barrier_err(msg: String) -> ClientRpcResponse {
    ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
        is_success: false,
        current_count: None,
        required_count: None,
        phase: None,
        error: Some(msg),
    })
}

fn phase_for(count: u32, required: u32) -> &'static str {
    if count >= required { "ready" } else { "waiting" }
}

pub fn handle_enter(name: String, participant_id: String, required_count: u32) -> ClientRpcResponse {
    let k = barrier_key(&name);
    match kv::cas_loop_json::<BarrierState, _>(&k, |current| {
        let mut state = current.unwrap_or(BarrierState {
            participants: Vec::new(),
            required_count,
        });
        if !state.participants.contains(&participant_id) {
            state.participants.push(participant_id.clone());
        }
        state.required_count = required_count;
        Ok(state)
    }) {
        Ok(state) => {
            let count = state.participants.len() as u32;
            ClientRpcResponse::BarrierEnterResult(BarrierResultResponse {
                is_success: true,
                current_count: Some(count),
                required_count: Some(state.required_count),
                phase: Some(phase_for(count, state.required_count).to_string()),
                error: None,
            })
        }
        Err(e) => barrier_err(e),
    }
}

pub fn handle_leave(name: String, participant_id: String) -> ClientRpcResponse {
    let k = barrier_key(&name);
    match kv::cas_loop_json::<BarrierState, _>(&k, |current| {
        let mut state = current.ok_or_else(|| "barrier not found".to_string())?;
        state.participants.retain(|p| p != &participant_id);
        Ok(state)
    }) {
        Ok(state) => {
            let count = state.participants.len() as u32;
            ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
                is_success: true,
                current_count: Some(count),
                required_count: Some(state.required_count),
                phase: Some(
                    if count == 0 {
                        "leaving"
                    } else {
                        phase_for(count, state.required_count)
                    }
                    .to_string(),
                ),
                error: None,
            })
        }
        Err(e) => ClientRpcResponse::BarrierLeaveResult(BarrierResultResponse {
            is_success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: Some(e),
        }),
    }
}

pub fn handle_status(name: String) -> ClientRpcResponse {
    let k = barrier_key(&name);
    match kv::get_json::<BarrierState>(&k) {
        Ok(Some(state)) => {
            let count = state.participants.len() as u32;
            ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
                is_success: true,
                current_count: Some(count),
                required_count: Some(state.required_count),
                phase: Some(phase_for(count, state.required_count).to_string()),
                error: None,
            })
        }
        Ok(None) => ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            is_success: true,
            current_count: Some(0),
            required_count: Some(0),
            phase: Some("waiting".to_string()),
            error: None,
        }),
        Err(e) => ClientRpcResponse::BarrierStatusResult(BarrierResultResponse {
            is_success: false,
            current_count: None,
            required_count: None,
            phase: None,
            error: Some(e),
        }),
    }
}
