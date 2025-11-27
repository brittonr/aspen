use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    body::Bytes,
};
use serde::{Deserialize, Serialize};

use crate::state::InfraState;

// ===== Data Types =====

#[derive(Debug, Serialize, Deserialize)]
pub struct BlobResponse {
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinTopicRequest {
    pub topic_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub topic_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectPeerRequest {
    pub endpoint_addr: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndpointInfo {
    pub endpoint_id: String,
    pub addresses: Vec<String>,
}

// ===== HTTP Handlers =====

/// POST /iroh/blob/store
/// Store a blob and return its hash
pub async fn store_blob(
    State(infra): State<InfraState>,
    body: Bytes,
) -> Result<Json<BlobResponse>, AppError> {
    let hash = infra
        .iroh()
        .store_blob(body)
        .await
        .map_err(|e| AppError::NotImplemented(format!("Blob storage: {}", e)))?;

    Ok(Json(BlobResponse { hash }))
}

/// GET /iroh/blob/{hash}
/// Retrieve a blob by its hash
pub async fn retrieve_blob(
    State(infra): State<InfraState>,
    Path(hash_str): Path<String>,
) -> Result<Response, AppError> {
    let data = infra
        .iroh()
        .retrieve_blob(hash_str)
        .await
        .map_err(|e| AppError::NotImplemented(format!("Blob retrieval: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .body(axum::body::Body::from(data))
        .map_err(|e| AppError::NotImplemented(format!("Failed to build response: {}", e)))?)
}

/// POST /iroh/gossip/join
/// Join a gossip topic
pub async fn join_gossip_topic(
    State(infra): State<InfraState>,
    Json(req): Json<JoinTopicRequest>,
) -> Result<StatusCode, AppError> {
    infra
        .iroh()
        .join_topic(req.topic_id)
        .await
        .map_err(|e| AppError::NotImplemented(format!("Gossip join: {}", e)))?;

    Ok(StatusCode::OK)
}

/// POST /iroh/gossip/broadcast
/// Broadcast a message to a gossip topic
pub async fn broadcast_gossip(
    State(infra): State<InfraState>,
    Json(req): Json<BroadcastRequest>,
) -> Result<StatusCode, AppError> {
    let message = Bytes::from(req.message.into_bytes());

    infra
        .iroh()
        .broadcast_message(req.topic_id, message)
        .await
        .map_err(|e| AppError::NotImplemented(format!("Gossip broadcast: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /iroh/gossip/subscribe/{topic_id}
/// Subscribe to a gossip topic (Server-Sent Events stream)
pub async fn subscribe_gossip(
    State(_infra): State<InfraState>,
    Path(_topic_id_str): Path<String>,
) -> Result<StatusCode, AppError> {
    Err(AppError::NotImplemented(
        "Gossip subscribe not yet implemented - API verification needed".to_string(),
    ))
}

/// POST /iroh/connect
/// Connect to a peer by endpoint address
pub async fn connect_peer(
    State(infra): State<InfraState>,
    Json(req): Json<ConnectPeerRequest>,
) -> Result<StatusCode, AppError> {
    infra
        .iroh()
        .connect_peer(req.endpoint_addr)
        .await
        .map_err(|e| AppError::NotImplemented(format!("Peer connection: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /iroh/info
/// Get endpoint information (ID and addresses)
pub async fn endpoint_info(State(infra): State<InfraState>) -> Result<Json<EndpointInfo>, AppError> {
    let endpoint_id = infra.iroh().endpoint_id();
    let addresses = infra.iroh().local_endpoints();

    Ok(Json(EndpointInfo {
        endpoint_id: endpoint_id.to_string(),
        addresses,
    }))
}

// ===== Error Handling =====

#[derive(Debug)]
pub enum AppError {
    NotImplemented(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
        };

        (status, message).into_response()
    }
}
