//! Distributed lock operation handlers.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::LockResultResponse;
use aspen_coordination::DistributedLock;
use aspen_coordination::LockConfig;
use aspen_coordination::LockEntry;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use aspen_core::validate_client_key;
use aspen_rpc_core::ClientProtocolContext;

fn lock_error(error: String) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        success: false,
        fencing_token: None,
        holder_id: None,
        deadline_ms: None,
        error: Some(error),
    })
}

fn lock_error_with_holder(error: String, entry: &LockEntry) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        success: false,
        fencing_token: Some(entry.fencing_token),
        holder_id: Some(entry.holder_id.clone()),
        deadline_ms: Some(entry.deadline_ms),
        error: Some(error),
    })
}

fn lock_success(fencing_token: u64, holder_id: String, deadline_ms: Option<u64>) -> ClientRpcResponse {
    ClientRpcResponse::LockResult(LockResultResponse {
        success: true,
        fencing_token: Some(fencing_token),
        holder_id: Some(holder_id),
        deadline_ms,
        error: None,
    })
}

pub(crate) async fn handle_lock_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = LockConfig {
        ttl_ms,
        acquire_timeout_ms: timeout_ms.unwrap_or(0), // 0 = no timeout
        ..Default::default()
    };
    let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

    match lock.acquire().await {
        Ok(guard) => {
            let token = guard.fencing_token().value();
            let deadline = guard.deadline_ms();
            std::mem::forget(guard);
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: true,
                fencing_token: Some(token),
                holder_id: Some(holder_id),
                deadline_ms: Some(deadline),
                error: None,
            }))
        }
        Err(e) => {
            use aspen_coordination::CoordinationError;
            let (holder, deadline) = match &e {
                CoordinationError::LockHeld { holder, deadline_ms } => (Some(holder.clone()), Some(*deadline_ms)),
                _ => (None, None),
            };
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: false,
                fencing_token: None,
                holder_id: holder,
                deadline_ms: deadline,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_lock_try_acquire(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(ClientRpcResponse::LockResult(LockResultResponse {
            success: false,
            fencing_token: None,
            holder_id: None,
            deadline_ms: None,
            error: Some(e.to_string()),
        }));
    }

    let config = LockConfig {
        ttl_ms,
        ..Default::default()
    };
    let lock = DistributedLock::new(ctx.kv_store.clone(), &key, &holder_id, config);

    match lock.try_acquire().await {
        Ok(guard) => {
            let token = guard.fencing_token().value();
            let deadline = guard.deadline_ms();
            std::mem::forget(guard);
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: true,
                fencing_token: Some(token),
                holder_id: Some(holder_id),
                deadline_ms: Some(deadline),
                error: None,
            }))
        }
        Err(e) => {
            use aspen_coordination::CoordinationError;
            let (holder, deadline) = match &e {
                CoordinationError::LockHeld { holder, deadline_ms } => (Some(holder.clone()), Some(*deadline_ms)),
                _ => (None, None),
            };
            Ok(ClientRpcResponse::LockResult(LockResultResponse {
                success: false,
                fencing_token: None,
                holder_id: holder,
                deadline_ms: deadline,
                error: Some(e.to_string()),
            }))
        }
    }
}

pub(crate) async fn handle_lock_release(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(lock_error(e.to_string()));
    }

    let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

    match read_result {
        Ok(result) => {
            let value = result.kv.map(|kv| kv.value).unwrap_or_default();
            release_with_cas(ctx, &key, &holder_id, fencing_token, &value).await
        }
        Err(aspen_core::KeyValueStoreError::NotFound { .. }) => Ok(lock_success(fencing_token, holder_id, None)),
        Err(e) => Ok(lock_error(format!("read failed: {}", e))),
    }
}

async fn release_with_cas(
    ctx: &ClientProtocolContext,
    key: &str,
    holder_id: &str,
    fencing_token: u64,
    value: &str,
) -> anyhow::Result<ClientRpcResponse> {
    let entry = match serde_json::from_str::<LockEntry>(value) {
        Ok(entry) => entry,
        Err(_) => return Ok(lock_error("invalid lock entry format".to_string())),
    };

    if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
        return Ok(lock_error_with_holder("lock not held by this holder".to_string(), &entry));
    }

    let released_json = serde_json::to_string(&entry.released())?;

    match ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndSwap {
                key: key.to_owned(),
                expected: Some(value.to_owned()),
                new_value: released_json,
            },
        })
        .await
    {
        Ok(_) => Ok(lock_success(fencing_token, holder_id.to_owned(), None)),
        Err(e) => Ok(lock_error(format!("release failed: {}", e))),
    }
}

pub(crate) async fn handle_lock_renew(
    ctx: &ClientProtocolContext,
    key: String,
    holder_id: String,
    fencing_token: u64,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    if let Err(e) = validate_client_key(&key) {
        return Ok(lock_error(e.to_string()));
    }

    let read_result = ctx.kv_store.read(ReadRequest::new(key.clone())).await;

    match read_result {
        Ok(result) => {
            let value = result.kv.map(|kv| kv.value).unwrap_or_default();
            renew_with_cas(ctx, &key, &holder_id, fencing_token, ttl_ms, &value).await
        }
        Err(e) => Ok(lock_error(format!("read failed: {}", e))),
    }
}

async fn renew_with_cas(
    ctx: &ClientProtocolContext,
    key: &str,
    holder_id: &str,
    fencing_token: u64,
    ttl_ms: u64,
    value: &str,
) -> anyhow::Result<ClientRpcResponse> {
    let entry = match serde_json::from_str::<LockEntry>(value) {
        Ok(entry) => entry,
        Err(_) => return Ok(lock_error("invalid lock entry format".to_string())),
    };

    if entry.holder_id != holder_id || entry.fencing_token != fencing_token {
        return Ok(lock_error_with_holder("lock not held by this holder".to_string(), &entry));
    }

    let renewed = LockEntry::new(holder_id.to_owned(), fencing_token, ttl_ms);
    let renewed_json = serde_json::to_string(&renewed)?;

    match ctx
        .kv_store
        .write(WriteRequest {
            command: WriteCommand::CompareAndSwap {
                key: key.to_owned(),
                expected: Some(value.to_owned()),
                new_value: renewed_json,
            },
        })
        .await
    {
        Ok(_) => Ok(lock_success(fencing_token, holder_id.to_owned(), Some(renewed.deadline_ms))),
        Err(e) => Ok(lock_error(format!("renew failed: {}", e))),
    }
}
