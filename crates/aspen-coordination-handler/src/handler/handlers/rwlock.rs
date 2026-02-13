//! Reader-writer lock operation handlers.

use aspen_client_api::ClientRpcResponse;
use aspen_client_api::RWLockResultResponse;
use aspen_coordination::RWLockManager;
use aspen_rpc_core::ClientProtocolContext;

pub(crate) async fn handle_rwlock_acquire_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.acquire_read(&name, &holder_id, ttl_ms, timeout).await {
        Ok((token, deadline, count)) => Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
            success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(count),
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_try_acquire_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.try_acquire_read(&name, &holder_id, ttl_ms).await {
        Ok(Some((token, deadline, count))) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
            success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(count),
            writer_holder: None,
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some("lock unavailable".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_acquire_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
    timeout_ms: Option<u64>,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());
    let timeout = timeout_ms.map(std::time::Duration::from_millis);

    match manager.acquire_write(&name, &holder_id, ttl_ms, timeout).await {
        Ok((token, deadline)) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("write".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(0),
            writer_holder: Some(holder_id),
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_try_acquire_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.try_acquire_write(&name, &holder_id, ttl_ms).await {
        Ok(Some((token, deadline))) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("write".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(0),
            writer_holder: Some(holder_id),
            error: None,
        })),
        Ok(None) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some("lock unavailable".to_string()),
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockTryAcquireWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_release_read(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.release_read(&name, &holder_id).await {
        Ok(()) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
            success: true,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockReleaseReadResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_release_write(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    fencing_token: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.release_write(&name, &holder_id, fencing_token).await {
        Ok(()) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
            success: true,
            mode: Some("free".to_string()),
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockReleaseWriteResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_downgrade(
    ctx: &ClientProtocolContext,
    name: String,
    holder_id: String,
    fencing_token: u64,
    ttl_ms: u64,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.downgrade(&name, &holder_id, fencing_token, ttl_ms).await {
        Ok((token, deadline, count)) => Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
            success: true,
            mode: Some("read".to_string()),
            fencing_token: Some(token),
            deadline_ms: Some(deadline),
            reader_count: Some(count),
            writer_holder: None,
            error: None,
        })),
        Err(e) => Ok(ClientRpcResponse::RWLockDowngradeResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}

pub(crate) async fn handle_rwlock_status(
    ctx: &ClientProtocolContext,
    name: String,
) -> anyhow::Result<ClientRpcResponse> {
    let manager = RWLockManager::new(ctx.kv_store.clone());

    match manager.status(&name).await {
        Ok((mode, reader_count, writer_holder, token)) => {
            Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
                success: true,
                mode: Some(mode),
                fencing_token: Some(token),
                deadline_ms: None,
                reader_count: Some(reader_count),
                writer_holder,
                error: None,
            }))
        }
        Err(e) => Ok(ClientRpcResponse::RWLockStatusResult(RWLockResultResponse {
            success: false,
            mode: None,
            fencing_token: None,
            deadline_ms: None,
            reader_count: None,
            writer_holder: None,
            error: Some(e.to_string()),
        })),
    }
}
