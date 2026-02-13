//! Counter operation handlers (unsigned and signed).

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::CounterResultResponse;
use aspen_client_api::SignedCounterResultResponse;
use aspen_coordination::AtomicCounter;
use aspen_coordination::CounterConfig;
use aspen_coordination::SignedAtomicCounter;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

// ============================================================================
// Unsigned Counter Operation Handlers
// ============================================================================

pub(crate) async fn handle_counter_get(ctx: &ClientProtocolContext, key: String) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.get().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_increment(
    ctx: &ClientProtocolContext,
    key: String,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.increment().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_decrement(
    ctx: &ClientProtocolContext,
    key: String,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.decrement().await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_add(
    ctx: &ClientProtocolContext,
    key: String,
    amount: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.add(amount).await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_subtract(
    ctx: &ClientProtocolContext,
    key: String,
    amount: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.subtract(amount).await {
        Ok(value) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_set(
    ctx: &ClientProtocolContext,
    key: String,
    value: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.set(value).await {
        Ok(()) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_counter_compare_and_set(
    ctx: &ClientProtocolContext,
    key: String,
    expected: u64,
    new_value: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = AtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.compare_and_set(expected, new_value).await {
        Ok(true) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: true,
            value: Some(new_value),
            error: None,
        })),
        Ok(false) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some("compare-and-set condition not met".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::CounterResult(CounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

// ============================================================================
// Signed Counter Operation Handlers
// ============================================================================

pub(crate) async fn handle_signed_counter_get(
    ctx: &ClientProtocolContext,
    key: String,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.get().await {
        Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_signed_counter_add(
    ctx: &ClientProtocolContext,
    key: String,
    amount: i64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        }));
    }

    let counter = SignedAtomicCounter::new(ctx.kv_store.clone(), &key, CounterConfig::default());
    match counter.add(amount).await {
        Ok(value) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: true,
            value: Some(value),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::SignedCounterResult(SignedCounterResultResponse {
            success: false,
            value: None,
            error: Some(e.to_string()),
        })),
    }
}
